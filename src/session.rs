use std::sync::Arc;

use sqlx::PgPool;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use crate::command::{Command, CommandExecutionError, CommandParseError, CommandResult, handle_command};
use crate::entities::{EntityId, EntityRegistry, EntityRegistryError, Position, Name};
use crate::event::{EventBus, EventBusError, EventTargetResolver, GameEvent};
use crate::model::world::{World, RoomId};
use crate::db::{self, DatabaseError};
use crate::password::{self, PasswordError};

#[derive(Debug)]
pub enum SessionError {
    Login,
    Internal(String),
    Send,
    Recv
}

impl From<DatabaseError> for SessionError {
    fn from(e: DatabaseError) -> SessionError {
        match e {
            DatabaseError::SqlxError(e) => SessionError::Internal(format!("Database error: {e}"))
        }
    }
}

impl From<PasswordError> for SessionError {
    fn from(e: PasswordError) -> SessionError {
        match e {
            PasswordError::CouldNotHash => SessionError::Internal("Failed to hash password".into()),
            PasswordError::InvalidHash => SessionError::Internal("Malformed hash in database".into())
        }
    }
}

impl From<CommandExecutionError> for SessionError {
    fn from(value: CommandExecutionError) -> Self {
        match value {
            CommandExecutionError::Unrecoverable(s) => SessionError::Internal(s)
        }
    }
}

impl From<EventBusError> for SessionError {
    fn from(value: EventBusError) -> Self {
        match value {
            EventBusError::InvalidMutex => SessionError::Internal("Event bus holds invalid mutex".into()),
            EventBusError::CouldNotSend => SessionError::Internal("Event bus failed to send".into())
        }
    }
}

impl From<EntityRegistryError> for SessionError {
    fn from(value: EntityRegistryError) -> Self {
        match value {
            EntityRegistryError::InvalidMutex => SessionError::Internal("Entity registry holds invalid mutex".into()),
            EntityRegistryError::UnknownEntity(name) => SessionError::Internal(format!("Attempted to update property of unknown entity '{name}'")),
            EntityRegistryError::DuplicateSpawn(_) => SessionError::Login
        }
    }
}

pub struct SessionContext {
    pub player_id: EntityId,
    pub world: Arc<World>,
    pub pool: PgPool,
    pub event_bus: Arc<EventBus>,
    pub entities: Arc<EntityRegistry>,
    receiver: mpsc::Receiver<GameEvent>
}

impl SessionContext {
    pub fn new(username: String, room: RoomId, world: Arc<World>, pool: PgPool, event_bus: Arc<EventBus>, entities: Arc<EntityRegistry>) -> Result<SessionContext, SessionError> {
        let player_id = entities.spawn()?;
        tracing::debug!("Session started for player {username} (id: {player_id:?})");
        entities.update_component(&player_id, Position { room })?;
        entities.update_component(&player_id, Name { value: username })?;

        let receiver = event_bus.register(&player_id)?;

        Ok(SessionContext { player_id, world, pool, event_bus, receiver, entities })
    }

    pub fn player_name(&self) -> Result<Name, SessionError> {
        let name = self.entities.get_component::<Name>(&self.player_id)?.unwrap();
        Ok(name)
    }
}

impl Drop for SessionContext {
    fn drop(&mut self) {
        let _ = self.event_bus.unregister(&self.player_id);
        let _ = self.entities.despawn(&self.player_id);
    }
}

/// Send a line of text to the client.
async fn send(writer: &mut OwnedWriteHalf, s: &str) -> Result<(), SessionError> {
    writer.write_all(format!("{s}\n").as_bytes()).await.map_err(|_| SessionError::Send)
}

/// Receive a line of text from the client.
/// Blocks until a complete line is received.
///
/// Returns `Ok(None)` on EOF.
async fn recv(reader: &mut BufReader<OwnedReadHalf>) -> Result<Option<String>, SessionError> {
    let mut line = String::new();
    match reader.read_line(&mut line).await {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(line.trim().into())),
        Err(_) => Err(SessionError::Recv)
    }
}

/// Get the initial password from the player (on account creation).
/// Prompts the user to confirm the password and only exits once a valid confirmation is made.
async fn get_initial_password(writer: &mut OwnedWriteHalf, reader: &mut BufReader<OwnedReadHalf>) -> Result<Option<String>, SessionError> {
    loop {
        send(writer, "New account; enter your password:").await?;
        let initial_password = match recv(reader).await? {
            Some(s) => s,
            None => return Ok(None)
        };

        send(writer, "Confirm your password:").await?;
        let confirmation = match recv(reader).await? {
            Some(s) => s,
            None => return Ok(None)
        };

        if initial_password == confirmation {
            return Ok(Some(initial_password))
        }
        else {
            send(writer, "Passwords do not match.").await?;
        }
    }
}

/// Welcome the given player to the game.
async fn welcome(writer: &mut OwnedWriteHalf, context: &SessionContext) -> Result<(), SessionError> {
    let name = &context.player_name()?;
    send(writer, &format!("Welcome {name}!")).await
}

async fn handle_input(session_context: &mut SessionContext, input: &str) -> Result<Option<String>, SessionError> {
    let response = match Command::parse(input) {
        Ok(command) => {
            let result = handle_command(session_context, command).await?;

            match result {
                CommandResult::Query(q) => Some(q.response().into()),
                CommandResult::Action(a) => {
                    for event in a.events() {
                        let targets = session_context.entities.resolve(&event.target)?;
                        session_context.event_bus.publish(&event.event, &targets).await?;
                    }
                    a.response().clone()
                }
            }
        }
        Err(CommandParseError::UnknownCommand(s)) => Some(format!("Unknown command: '{s}'")),
        Err(CommandParseError::InvalidSyntax(s)) => Some(s)
    };

    Ok(response)
}

/// Execute the game loop for the given session.
async fn run_internal(writer: &mut OwnedWriteHalf, reader: &mut BufReader<OwnedReadHalf>, pool: PgPool, world: Arc<World>, event_bus: Arc<EventBus>, entities: Arc<EntityRegistry>) -> Result<(), SessionError> {
    send(writer, "Enter your username:").await?;
    let username = match recv(reader).await? {
        Some(s) => s,
        None => return Ok(())
    };

    let account = db::get_account(&pool, &username).await?;
    let account = match account {
        Some(a) => {
            // Account exists; verify password.
            send(writer, "Enter your password:").await?;
            let password = match recv(reader).await? {
                Some(s) => s,
                None => return Ok(())
            };
            if password::verify_password(&password, &a.password_hash)? {
                a
            }
            else {
                send(writer, "Incorrect password.").await?;
                return Ok(())
            }
        },
        None => {
            // Account does not exist; create it.
            let password = match get_initial_password(writer, reader).await? {
                Some(s) => s,
                None => return Ok(())
            };
            let password_hash = password::hash_password(&password)?;
            db::create_account(&pool, &username, &password_hash).await?
        }
    };

    let mut session_context = SessionContext::new(account.username, RoomId::new(account.current_room_id), world, pool, event_bus, entities)?;
    welcome(writer, &session_context).await?;

    loop {
        tokio::select! {
            line = recv(reader) => {
                match line {
                    Ok(Some(input)) => {
                        let response = handle_input(&mut session_context, &input).await?;
                        if let Some(s) = response {
                            send(writer, &s).await?;
                        }
                    },
                    Ok(None) => {
                        tracing::info!("Player '{}' disconnected", &session_context.player_name()?);
                        break;
                    },
                    Err(e) => return Err(e)
                }
            }
            event = session_context.receiver.recv() => {
                match event {
                    Some(e) => {
                        match e {
                            GameEvent::Message(s) => send(writer, &s).await?,
                            GameEvent::SessionEnded => {
                                tracing::debug!("Entity {:?} received SessionEnded", session_context.player_id);
                                break;
                            }
                        }
                    },
                    None => {
                        tracing::warn!("No senders in event bus");
                    }
                }
            }
        }
    }
    Ok(())
}

pub async fn run(writer: &mut OwnedWriteHalf, reader: &mut BufReader<OwnedReadHalf>, pool: PgPool, world: Arc<World>, event_bus: Arc<EventBus>, entities: Arc<EntityRegistry>) -> Result<(), SessionError> {
    let result = run_internal(writer, reader, pool, world, event_bus, entities).await;
    match &result {
        Ok(()) => (),
        Err(e) => {
            let response = match e {
                SessionError::Login => Some("An error occurred during login.".to_string()),
                SessionError::Internal(_) => Some("An internal error occurred.".to_string()),

                // If a send/receive error has occurred there is no point trying to use the connection again.
                SessionError::Recv => None,
                SessionError::Send => None
            };

            // We don't really care at this point if there is an error sending the session response.
            // The session is already unrecoverable.
            match response {
                Some(s) => { let _ = send(writer, &s).await; },
                None => ()
            };
        }
    }
    result
}
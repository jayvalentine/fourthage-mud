use std::sync::Arc;

use sqlx::PgPool;
use tokio::net::{tcp::WriteHalf, tcp::ReadHalf};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};

use crate::command::{Command, CommandParseError, CommandExecutionError, handle_go, handle_look};
use crate::model::player::Player;
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
            _ => SessionError::Login,
        }
    }
}

/// Send a line of text to the client.
async fn send(writer: &mut WriteHalf<'_>, s: &str) -> Result<(), SessionError> {
    writer.write_all(format!("{s}\n").as_bytes()).await.map_err(|_| SessionError::Send)
}

/// Receive a line of text from the client.
/// Blocks until a complete line is received.
///
/// Returns `Ok(None)` on EOF.
async fn recv(reader: &mut BufReader<ReadHalf<'_>>) -> Result<Option<String>, SessionError> {
    let mut line = String::new();
    match reader.read_line(&mut line).await {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(line.trim().into())),
        Err(_) => Err(SessionError::Recv)
    }
}

/// Welcome the given player to the game.
async fn welcome(writer: &mut WriteHalf<'_>, player: &Player) -> Result<(), SessionError> {
    let name = player.name();
    send(writer, &format!("Welcome {name}!")).await
}

/// Execute the game loop for the given session.
async fn run_internal(pool: PgPool, writer: &mut WriteHalf<'_>, reader: &mut BufReader<ReadHalf<'_>>, world: Arc<World>) -> Result<(), SessionError> {
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
            send(writer, "New account; enter your password:").await?;
            let password = match recv(reader).await? {
                Some(s) => s,
                None => return Ok(())
            };
            let password_hash = password::hash_password(&password)?;
            db::create_account(&pool, &username, &password_hash).await?
        }
    };

    let mut player = Player::new(account.username, RoomId::new(account.current_room_id));

    welcome(writer, &player).await?;

    let name = player.name().to_owned();

    loop {
        let response = match recv(reader).await? {
            None => {
                tracing::info!("Player '{name}' disconnected");
                break;
            }
            Some(input) => {
                match Command::parse(&input) {
                    Ok(command) => {
                        let result = match command {
                            Command::Go(direction) => handle_go(&world, &mut player, direction),
                            Command::Look => handle_look(&world, &mut player),
                            Command::Quit => {
                                let name = player.name();
                                tracing::info!("Player '{name}' quit");
                                send(writer, &format!("Goodbye {name}!")).await?;
                                break;
                            }
                        };

                        match result {
                            Ok(s) => s,
                            Err(CommandExecutionError::InvalidCommand(s)) => s,
                            Err(CommandExecutionError::Unrecoverable(s)) => {
                                tracing::error!("Unrecoverable error occurred processing command from '{name}': {s}");
                                format!("Cannot execute command: {s}")
                            }
                        }
                    }
                    Err(CommandParseError::UnknownCommand(s)) => format!("Unknown command: '{s}'"),
                    Err(CommandParseError::InvalidSyntax(s)) => s
                }
            }
        };

        send(writer, &response).await?
    }

    Ok(())
}

pub async fn run(pool: PgPool, writer: &mut WriteHalf<'_>, reader: &mut BufReader<ReadHalf<'_>>, world: Arc<World>) -> Result<(), SessionError> {
    let result = run_internal(pool, writer, reader, world).await;
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
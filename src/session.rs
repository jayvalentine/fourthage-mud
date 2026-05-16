use std::sync::Arc;

use sqlx::PgPool;
use tokio::net::{tcp::WriteHalf, tcp::ReadHalf};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};

use crate::command::{Command, CommandParseError, CommandExecutionError, handle_go, handle_look};
use crate::model::player::Player;
use crate::model::world::{World, RoomId};
use crate::db;

#[derive(Debug)]
pub enum SessionError {
    Login(String),
    Send,
    Recv
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

/// Login the player with the given username.
async fn login(pool: &PgPool, username: &str) -> Result<Player, SessionError> {
    let account = db::get_account(pool, username)
        .await
        .map_err(|_| SessionError::Login(format!("Error retrieving player '{username}' from database")))?;

    let account = match account {
        Some(a) => a,
        None => {
            db::create_account(pool, username)
                .await
                .map_err(|_| SessionError::Login(format!("Error creating player '{username}'")))?
        }
    };

    Ok(Player::new(account.username, RoomId::new(account.current_room_id)))
}

/// Welcome the given player to the game.
async fn welcome(writer: &mut WriteHalf<'_>, player: &Player) -> Result<(), SessionError> {
    let name = player.name();
    send(writer, &format!("Welcome {name}!")).await
}

/// Execute the game loop for the given session.
pub async fn run(pool: PgPool, writer: &mut WriteHalf<'_>, reader: &mut BufReader<ReadHalf<'_>>, world: Arc<World>) -> Result<(), SessionError> {
    send(writer, "Enter your username:").await?;
    let username = match recv(reader).await? {
        Some(s) => s,
        None => return Ok(())
    };

    let mut player = match login(&pool, &username).await {
        Ok(p) => p,
        Err(e) => {
            send(writer, "An error occurred during login.").await?;
            return Err(e);
        }
    };

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

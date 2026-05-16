use std::sync::Arc;

use sqlx::PgPool;
use tokio::net::{tcp::WriteHalf, tcp::ReadHalf};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};

use crate::command::{Command, CommandParseError, CommandExecutionError, handle_go, handle_look};
use crate::model::player::Player;
use crate::model::world::{World, RoomId};
use crate::db::{self, AccountRow};

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

/// Get an existing player if one exists with the given username.
/// Returns None if no named player exists.
async fn get_account(pool: &PgPool, username: &str) -> Result<Option<AccountRow>, SessionError> {
    db::get_account(pool, username)
        .await
        .map_err(|_| SessionError::Login(format!("Error retrieving account '{username}' from database")))
}

fn verify_account_password(account: &AccountRow, password: &str) -> bool {
    true
}

async fn create_account(pool: &PgPool, username: &str, password: &str) -> Result<AccountRow, SessionError> {
    db::create_account(pool, username)
        .await
        .map_err(|_| SessionError::Login(format!("Error creating account '{username}' from database")))
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

    let account = get_account(&pool, &username).await?;
    let account = match account {
        Some(a) => {
            // Account exists; verify password.
            send(writer, "Enter your password:").await?;
            let password = match recv(reader).await? {
                Some(s) => s,
                None => return Ok(())
            };
            if verify_account_password(&a, &password) {
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
            create_account(&pool, &username, &password).await?
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

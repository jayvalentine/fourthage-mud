use std::io::Error;
use std::sync::Arc;

use tokio::net::{tcp::WriteHalf, tcp::ReadHalf};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};

use crate::command::{Command, CommandParseError, CommandExecutionError, handle_go, handle_look};
use crate::model::player::Player;
use crate::model::world::{RoomId, World};

async fn send(writer: &mut WriteHalf<'_>, s: &str) -> Result<(), Error> {
    writer.write_all(format!("{s}\n").as_bytes()).await
}

/// Initialise the player for a session.
fn init() -> Player {
    Player::new(RoomId::new(0))
}

/// Welcome the given player to the game.
async fn welcome(writer: &mut WriteHalf<'_>, player: &Player) -> Result<(), Error> {
    let name = player.name();
    send(writer, &format!("Welcome {name}!")).await
}

/// Execute the game loop for the given session.
pub async fn run(writer: &mut WriteHalf<'_>, reader: &mut BufReader<ReadHalf<'_>>, world: Arc<World>) -> Result<(), Error> {
    let mut player = init();
    welcome(writer, &player).await?;

    let name = player.name().to_owned();

    let mut line = String::new();

    loop {
        line.clear();
        let response = match reader.read_line(&mut line).await? {
            0 => {
                tracing::info!("Player '{name}' disconnected");
                break;
            }
            _ => {
                let input = line.trim();
                match Command::parse(input) {
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

use std::io::Error;
use std::sync::Arc;
use tokio::net::{TcpListener, tcp::WriteHalf, tcp::ReadHalf};
use tokio::io::{AsyncWriteExt, AsyncBufReadExt, BufReader};

use crate::command::{Command, CommandParseError, CommandExecutionError, handle_go, handle_look};
use crate::model::player::Player;
use crate::model::world::{RoomId, World};

mod model;
mod command;

async fn send(writer: &mut WriteHalf<'_>, s: &str) -> Result<(), Error> {
    writer.write_all(format!("{s}\n").as_bytes()).await
}

/// Create a new player.
fn player_init() -> Player {
    Player::new(RoomId::new(0))
}

/// Welcome the given player to the game.
async fn player_welcome(writer: &mut WriteHalf<'_>, player: &Player) -> Result<(), Error> {
    let name = player.name();
    send(writer, &format!("Welcome {name}!")).await
}

/// Execute the game loop for the given player.
async fn player_loop(writer: &mut WriteHalf<'_>, reader: &mut BufReader<ReadHalf<'_>>, world: Arc<World>, mut player: Player) -> Result<(), Error> {
    let mut line = String::new();

    let name = player.name().to_owned();

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
                            Command::Look => handle_look(&world, &mut player)
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

        writer.write_all(format!("{response}\n").as_bytes()).await?
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let world = Arc::new(model::world::get_world());

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("Listening on port 8080");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        tracing::info!("Handling connection from {addr}");

        let world = world.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);

            let player = player_init();
            let result = player_welcome(&mut writer, &player).await;

            if result.is_ok()
            {
                player_loop(&mut writer, &mut reader, world, player).await.unwrap_or_else(|e| {
                    tracing::error!("Error during player loop: {e}");
                });
            }
            else {
                let e = result.unwrap_err();
                tracing::error!("Error welcoming player: {e}");
            }

            tracing::info!("Connection from {addr} closed");
        });
    }
}

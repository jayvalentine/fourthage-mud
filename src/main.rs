use std::sync::Arc;

use tokio::net::TcpListener;
use tokio::io::BufReader;

mod model;
mod command;
mod session;
mod data;
mod db;
mod password;
mod event;
mod entities;

use model::world::World;
use event::EventBus;

use crate::entities::EntityRegistry;

#[derive(Debug)]
enum AppError {
    InitialisationError
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        tracing::error!("DATABASE_URL not set");
        AppError::InitialisationError
    })?;

    tracing::info!("Connecting to database at {database_url}");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url).await.map_err(|e| {
            tracing::error!("Failed to connect to database: {e}");
            AppError::InitialisationError
        })?;
    sqlx::migrate!().run(&pool).await.map_err(|e| {
        tracing::error!("Failed to run database migrations: {e}");
        AppError::InitialisationError
    })?;

    let data_path = std::env::var("MUD_DATA_DIR").map_err(|e| {
       tracing::error!("Error reading MUD_DATA_DIR environment variable: {e}");
       AppError::InitialisationError 
    })?;

    let rooms = data::get_rooms(&format!("{data_path}/rooms/rooms.json")).map_err(|e| {
        tracing::error!("Error loading room data: {e}");
        AppError::InitialisationError
    })?;

    let world = Arc::new(World::new(rooms));
    let event_bus = Arc::new(EventBus::new());
    let entities = Arc::new(EntityRegistry::new());

    let listener = TcpListener::bind("0.0.0.0:8080").await.map_err(|e| {
        tracing::error!("Error starting TCP listener: {e}");
        AppError::InitialisationError
    })?;
    tracing::info!("Listening on port 8080");

    loop {
        match listener.accept().await {
            Ok((mut socket, addr)) => {
                tracing::info!("Handling connection from {addr}");
                let world = world.clone();
                let pool = pool.clone();
                let event_bus = event_bus.clone();
                let entities = entities.clone();

                tokio::spawn(async move {
                    let (reader, mut writer) = socket.split();
                    let mut reader = BufReader::new(reader);

                    session::run(&mut writer, &mut reader, pool, world, event_bus, entities).await.unwrap_or_else(|e| {
                        tracing::error!("Error during session from {addr}: {e:?}");
                    });

                    tracing::info!("Connection from {addr} closed");
                });
            },
            Err(e) => {
                tracing::error!("Error handling new connection: {e}");
                continue;
            }
        }
    }
}

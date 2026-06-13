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
mod persistence;
mod seed;

use model::world::World;
use event::EventBus;
use tokio::sync::oneshot::Receiver;
use uuid::Uuid;

use crate::entities::EntityRegistry;
use crate::model::ids::RoomId;
use crate::seed::{ItemSeeder, Seeder};

#[derive(Debug)]
pub enum AppError {
    InitialisationError
}

/// Helper function for password hashing for external services.
pub fn test_hash_password(password: &str) -> String {
    // Call your real hashing logic
    crate::password::hash_password(password).expect("Failed to hash password")
}

async fn accept_loop(listener: TcpListener, world: Arc<World>, pool: sqlx::PgPool, event_bus: Arc<EventBus>, entities: Arc<EntityRegistry>) {
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                tracing::info!("Handling connection from {addr}");
                let world = world.clone();
                let pool = pool.clone();
                let event_bus = event_bus.clone();
                let entities = entities.clone();

                tokio::spawn(async move {
                    let (reader, mut writer) = socket.into_split();
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

pub async fn run_server(listener: TcpListener, shutdown_rx: Receiver<()>, database_url: &str, data_path: &str, starting_room: Uuid) -> Result<(), AppError> {
    tracing::info!("Starting server...");

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

    let rooms = data::get_rooms(&format!("{data_path}/rooms.yaml")).map_err(|e| {
        tracing::error!("Error loading room data: {e:?}");
        AppError::InitialisationError
    })?;

    let world = Arc::new(World::new(rooms, RoomId::from_uuid(starting_room)));
    let event_bus = Arc::new(EventBus::new());
    let entities = Arc::new(EntityRegistry::new());

    ItemSeeder::seed(&format!("{data_path}/items.yaml"), &pool, &world, &entities).await.map_err(|e| {
        tracing::error!("Failed to seed items: {e:?}");
        AppError::InitialisationError
    })?;

    tracing::info!("Listening on port {}", listener.local_addr().map(|addr| addr.port()).unwrap_or(0));

    tokio::select! {
        _ = accept_loop(listener, world, pool, event_bus, entities) => {},
        _ = shutdown_rx => {
            tracing::info!("Shutdown signal received, stopping server");
        }
    }

    Ok(())
}

use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
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
mod system;

use model::rooms::RoomGraph;
use event::EventBus;
use tokio::sync::oneshot::Receiver;
use tokio::time::{Instant, interval, MissedTickBehavior};
use uuid::Uuid;

use crate::entities::EntityRegistry;
use crate::model::ids::RoomId;
use crate::persistence::PersistenceSystem;
use crate::seed::{ItemSeeder, NpcSeeder, RoomSeeder, Seeder};
use crate::system::{System, SystemContext, SystemError};

#[derive(Debug)]
pub enum AppError {
    InitialisationError,
    SystemExecutionError(SystemError)
}

impl From<SystemError> for AppError {
    fn from(value: SystemError) -> Self {
        AppError::SystemExecutionError(value)
    }
}

/// Helper function for password hashing for external services.
pub fn test_hash_password(password: &str) -> String {
    // Call your real hashing logic
    crate::password::hash_password(password).expect("Failed to hash password")
}

/// Seed game content from data files.
/// Dynamic content in the database is not overwritten.
///
/// Seeding occurs in a specific order because subsequent steps rely on previously-seeded data:
///
/// * Rooms
/// * Items
/// * NPCs
///
/// If a seeding step fails, the seeding process is aborted and subsequent steps are not run.
///
async fn seed(data_path: &str, pool: &PgPool, room_graph: &RoomGraph, entities: &EntityRegistry) -> Result<(), AppError> {
    RoomSeeder::seed(&format!("{data_path}/rooms.yaml"), pool, room_graph, entities).await.map_err(|e| {
        tracing::error!("Failed to seed rooms: {e:?}");
        AppError::InitialisationError
    })?;

    ItemSeeder::seed(&format!("{data_path}/items.yaml"), pool, room_graph, entities).await.map_err(|e| {
        tracing::error!("Failed to seed items: {e:?}");
        AppError::InitialisationError
    })?;

    NpcSeeder::seed(&format!("{data_path}/npcs.yaml"), pool, room_graph, entities).await.map_err(|e| {
        tracing::error!("Failed to seed NPCs: {e:?}");
        AppError::InitialisationError
    })?;

    Ok(())
}

async fn accept_loop(listener: TcpListener, world: Arc<RoomGraph>, pool: sqlx::PgPool, event_bus: Arc<EventBus>, entities: Arc<EntityRegistry>) {
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

const TICK_RATE: Duration = Duration::from_secs(1);

async fn game_loop(context: Arc<SystemContext>, systems: Vec<Arc<dyn System>>) -> ! {
    let mut interval = interval(TICK_RATE);
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        interval.tick().await;
        let tick_start = Instant::now();
        tracing::debug!("Game loop tick...");
        for system in &systems {
            let system_start = Instant::now();
            if let Err(e) = system.run(&context).await {
                tracing::error!("System {} returned with error: {:?}", system.name(), e);
            }
            let system_elapsed = system_start.elapsed();
            tracing::debug!("System {} completed in {:?}", system.name(), system_elapsed);
        }

        let elapsed = tick_start.elapsed();
        if elapsed > TICK_RATE {
            tracing::warn!("Game loop tick took longer than expected: {:?}", elapsed);
        } else {
            tracing::debug!("Game loop tick done in {:?}.", elapsed);
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

    let world = Arc::new(RoomGraph::new(RoomId::from_uuid(starting_room)));
    let event_bus = Arc::new(EventBus::new());
    let entities = Arc::new(EntityRegistry::new());

    seed(data_path, &pool, &world, &entities).await?;

    let system_context = Arc::new(SystemContext::new(entities.clone(), world.clone(), pool.clone(), event_bus.clone()));

    let systems = vec![
        Arc::new(PersistenceSystem) as Arc<dyn System>
    ];

    let game_loop_handle = tokio::spawn(game_loop(system_context, systems));

    tracing::info!("Listening on port {}", listener.local_addr().map(|addr| addr.port()).unwrap_or(0));

    tokio::select! {
        _ = accept_loop(listener, world, pool, event_bus, entities) => {},
        _ = shutdown_rx => {
            tracing::info!("Shutdown signal received, stopping server");
        }
    }

    game_loop_handle.abort();

    Ok(())
}

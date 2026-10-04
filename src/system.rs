use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::RwLock;
use sqlx::PgPool;

use crate::{db::DatabaseError, entities::{EntityRegistry, EntityRegistryError}, event::{EventBus, EventBusError}, model::{rooms::RoomGraph, time::Tick}};

#[derive(Debug)]
pub enum SystemError {
    Database(DatabaseError),
    EntityRegistry(EntityRegistryError),
    EventBus(EventBusError)
}

impl From<DatabaseError> for SystemError {
    fn from(err: DatabaseError) -> Self {
        SystemError::Database(err)
    }
}

impl From<EntityRegistryError> for SystemError {
    fn from(err: EntityRegistryError) -> Self {
        SystemError::EntityRegistry(err)
    }
}

impl From<EventBusError> for SystemError {
    fn from(value: EventBusError) -> Self {
        SystemError::EventBus(value)
    }
}

pub struct SystemContext {
    registry: Arc<EntityRegistry>,
    rooms: Arc<RoomGraph>,
    pool: PgPool,
    event_bus: Arc<EventBus>,
    current_tick: Tick
}

impl SystemContext {
    pub fn new(registry: Arc<EntityRegistry>, rooms: Arc<RoomGraph>, pool: PgPool, event_bus: Arc<EventBus>) -> Self {
        Self { registry, rooms, pool, event_bus, current_tick: Tick::default() }
    }

    pub fn entities(&self) -> &Arc<EntityRegistry> {
        &self.registry
    }

    pub fn rooms(&self) -> &Arc<RoomGraph> {
        &self.rooms
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    pub fn increment_tick(&mut self) {
        self.current_tick += 1.into();
    }

    pub fn current_tick(&self) -> Tick {
        self.current_tick
    }
}

#[async_trait]
pub trait System: Send + Sync {
    fn name(&self) -> &str;
    
    async fn run(&mut self, context: &SystemContext) -> Result<(), SystemError>;
}

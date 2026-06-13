use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::{db::DatabaseError, entities::{EntityRegistry, EntityRegistryError}, event::EventBus, model::world::World};

#[derive(Debug)]
pub enum SystemError {
    Database(DatabaseError),
    EntityRegistry(EntityRegistryError)
}

pub struct SystemContext {
    registry: Arc<EntityRegistry>,
    world: Arc<World>,
    pool: PgPool,
    event_bus: Arc<EventBus>
}

impl SystemContext {
    pub fn new(registry: Arc<EntityRegistry>, world: Arc<World>, pool: PgPool, event_bus: Arc<EventBus>) -> Self {
        Self { registry, world, pool, event_bus }
    }

    pub fn entities(&self) -> &EntityRegistry {
        &self.registry
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
    
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }
}

#[async_trait]
pub trait System: Send + Sync {
    async fn run(&self, context: &SystemContext) -> Result<(), SystemError>;
}

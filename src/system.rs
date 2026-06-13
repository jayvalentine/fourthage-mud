use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::{entities::EntityRegistry, event::EventBus, model::world::World};

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
}

#[async_trait]
pub trait System: Send + Sync {
    async fn run(&self, context: &SystemContext);
}

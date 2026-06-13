use async_trait::async_trait;
use sqlx::PgPool;

use crate::{db::{self, DatabaseError}, entities::Location, model::ids::EntityId, system::{System, SystemContext, SystemError}};

pub async fn seed_location(entity: &EntityId, component: &Location, pool: &PgPool) -> Result<Location, DatabaseError> {
    tracing::debug!("Location seeded for entity {entity}: {component:?}");
    db::insert_location_if_absent(pool, entity, &component.value).await
        .map(|e| Location { value: e })
}

pub async fn persist_location(entity: &EntityId, component: &Location, pool: &PgPool) -> Result<(), DatabaseError> {
    tracing::debug!("Position saved for entity {entity:?}: {component:?}");
    db::update_location(pool, entity, &component.value).await
}

pub async fn load_location(entity: &EntityId, pool: &PgPool) -> Result<Option<Location>, DatabaseError> {
    let component = db::get_location(pool, entity).await.map(|o| o.map(|id| Location { value: id }));
    tracing::debug!("Position loaded for entity {entity:?}: {component:?}");
    component
}

/// System for persisting entity components to the database.
/// Since the system only runs per tick, it is possible that updates may be lost
/// if the server crashes between a change in the entity registry and the next tick.
/// 
/// This risk is acceptable for this project since component updates are not expected
/// to be frequent.
pub struct PersistenceSystem;

impl From<DatabaseError> for SystemError {
    fn from(err: DatabaseError) -> Self {
        SystemError::Database(err)
    }
}

#[async_trait]
impl System for PersistenceSystem {
    fn name(&self) -> &str {
        "PersistenceSystem"
    }
    
    async fn run(&self, context: &SystemContext) -> Result<(), SystemError> {
        let locations = context.entities().query::<Location, _, _>(|iter| {
            Ok(iter
                .map(|(entity, location)| (entity.clone(), location.clone()))
                .collect::<Vec<(EntityId, Location)>>())
        }).map_err(SystemError::EntityRegistry)?;

        for (entity, location) in locations {
            persist_location(&entity, &location, context.pool()).await?;
        }

        Ok(())
    }
}

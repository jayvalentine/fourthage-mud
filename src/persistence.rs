use sqlx::PgPool;

use crate::{db::{self, DatabaseError}, entities::Location, model::ids::EntityId};

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

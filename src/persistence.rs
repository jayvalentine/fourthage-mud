use sqlx::PgPool;

use crate::{db::{self, DatabaseError}, entities::Location, model::ids::EntityId};

pub async fn persist_position(entity: &EntityId, component: &Location, pool: &PgPool) -> Result<(), DatabaseError> {
    tracing::debug!("Position saved for entity {entity:?}: {component:?}");
    db::update_location(pool, entity, &component.value).await
}

pub async fn load_position(entity: &EntityId, pool: &PgPool) -> Result<Option<Location>, DatabaseError> {
    let component = db::get_location(pool, entity).await.map(|o| o.map(|id| Location { value: id }));
    tracing::debug!("Position loaded for entity {entity:?}: {component:?}");
    component
}

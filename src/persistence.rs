use sqlx::PgPool;

use crate::{db::{self, DatabaseError}, entities::Position, model::ids::EntityId};

pub async fn persist_position(entity: &EntityId, component: &Position, pool: &PgPool) -> Result<(), DatabaseError> {
    tracing::debug!("Position saved for entity {entity:?}: {component:?}");
    db::update_position(pool, entity, &component.room).await
}

pub async fn load_position(entity: &EntityId, pool: &PgPool) -> Result<Option<Position>, DatabaseError> {
    let component = db::get_position(pool, entity).await.map(|o| o.map(|id| Position { room: id }));
    tracing::debug!("Position loaded for entity {entity:?}: {component:?}");
    component
}

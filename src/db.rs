use sqlx::{PgPool};

use crate::model::ids::EntityId;

#[derive(Debug)]
pub enum DatabaseError {
    SqlxError(sqlx::Error)
}

impl From<sqlx::Error> for DatabaseError {
    fn from(value: sqlx::Error) -> Self {
        DatabaseError::SqlxError(value)
    }
}

pub struct AccountRow {
    pub id: EntityId,
    pub username: String,
    pub is_admin: bool,
    pub password_hash: String
}

pub async fn get_account(pool: &PgPool, username: &str) -> Result<Option<AccountRow>, DatabaseError> {
    let account = sqlx::query!(
        "SELECT id, username, is_admin, password_hash FROM accounts WHERE username = $1",
        username
    ).fetch_optional(pool).await?;

    match account {
        Some(row) => Ok(Some(AccountRow {
            id: EntityId::from_uuid(row.id),
            username: row.username,
            is_admin: row.is_admin,
            password_hash: row.password_hash })),
        None => Ok(None)
    }
}

pub async fn create_account(pool: &PgPool, username: &str, password_hash: &str) -> Result<AccountRow, DatabaseError> {
    let row = sqlx::query!(
        "INSERT INTO accounts (username, password_hash) VALUES ($1, $2)
         RETURNING id, username, is_admin, password_hash",
        username,
        password_hash
    ).fetch_one(pool).await?;

    Ok(AccountRow { id: EntityId::from_uuid(row.id), username: row.username, is_admin: row.is_admin, password_hash: row.password_hash })
}

pub async fn update_location(pool: &PgPool, id: &EntityId, location_id: &EntityId) -> Result<(), DatabaseError> {
    sqlx::query!(
        "INSERT INTO positions (entity_id, room_id) VALUES ($1, $2)
         ON CONFLICT(entity_id) DO UPDATE SET room_id = $2",
        id.as_uuid(),
        location_id.as_uuid()
    ).execute(pool).await?;
    Ok(())
}

pub async fn insert_location_if_absent(pool: &PgPool, id: &EntityId, location_id: &EntityId) -> Result<EntityId, DatabaseError> {
    let row = sqlx::query!(
        "INSERT INTO positions (entity_id, room_id) VALUES ($1, $2)
         ON CONFLICT(entity_id) DO UPDATE SET room_id = positions.room_id
         RETURNING room_id",
        id.as_uuid(),
        location_id.as_uuid()
    ).fetch_one(pool).await?;
    Ok(EntityId::from_uuid(row.room_id))
}

pub async fn get_location(pool: &PgPool, id: &EntityId) -> Result<Option<EntityId>, DatabaseError> {
    let room = sqlx::query!(
        "SELECT room_id FROM positions WHERE entity_id = $1",
        id.as_uuid()
    ).fetch_optional(pool).await?;

    match room {
        Some(id) => Ok(Some(EntityId::from_uuid(id.room_id))),
        None => Ok(None)
    }
}

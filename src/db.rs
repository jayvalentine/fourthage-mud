use sqlx::{PgPool};

use crate::model::ids::{EntityId, RoomId};

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
    pub password_hash: String
}

pub async fn get_account(pool: &PgPool, username: &str) -> Result<Option<AccountRow>, DatabaseError> {
    let account = sqlx::query!(
        "SELECT id, username, password_hash FROM accounts WHERE username = $1",
        username
    ).fetch_optional(pool).await?;

    match account {
        Some(row) => Ok(Some(AccountRow { id: EntityId::from_uuid(row.id), username: row.username, password_hash: row.password_hash })),
        None => Ok(None)
    }
}

pub async fn create_account(pool: &PgPool, username: &str, password_hash: &str) -> Result<AccountRow, DatabaseError> {
    let row = sqlx::query!(
        "INSERT INTO accounts (username, password_hash) VALUES ($1, $2)
         RETURNING id, username, password_hash",
        username,
        password_hash
    ).fetch_one(pool).await?;

    Ok(AccountRow { id: EntityId::from_uuid(row.id), username: row.username, password_hash: row.password_hash })
}

pub async fn update_position(pool: &PgPool, id: &EntityId, room_id: &RoomId) -> Result<(), DatabaseError> {
    sqlx::query!(
        "UPDATE positions SET room_id = $1 WHERE entity_id = $2",
        room_id.as_uuid(),
        id.as_uuid()
    ).execute(pool).await?;
    Ok(())
}

pub async fn get_position(pool: &PgPool, id: &EntityId) -> Result<Option<RoomId>, DatabaseError> {
    let room = sqlx::query!(
        "SELECT room_id FROM positions WHERE entity_id = $1",
        id.as_uuid()
    ).fetch_optional(pool).await?;

    match room {
        Some(id) => Ok(Some(RoomId::from_uuid(id.room_id))),
        None => Ok(None)
    }
}

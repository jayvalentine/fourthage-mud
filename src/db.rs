use sqlx::PgPool;

use crate::model::{player::Player, world::RoomId};

pub enum DatabaseError {
    SqlxError(sqlx::Error)
}

pub struct AccountRow {
    pub username: String,
    pub password_hash: String,
    pub current_room_id: i32
}

pub async fn get_account(pool: &PgPool, username: &str) -> Result<Option<AccountRow>, DatabaseError> {
    let account = sqlx::query!(
        "SELECT id, username, password_hash, current_room_id FROM accounts WHERE username = $1",
        username
    ).fetch_optional(pool).await.map_err(|e| DatabaseError::SqlxError(e))?;

    match account {
        Some(row) => Ok(Some(AccountRow { username: row.username, password_hash: row.password_hash, current_room_id: row.current_room_id })),
        None => Ok(None)
    }
}

pub async fn create_account(pool: &PgPool, username: &str) -> Result<AccountRow, DatabaseError> {
    sqlx::query!(
        "INSERT INTO accounts (username, password_hash) VALUES ($1, $2)",
        username,
        ""
    ).execute(pool).await.map_err(|e| DatabaseError::SqlxError(e))?;

    let row = sqlx::query!(
        "SELECT id, username, password_hash, current_room_id FROM accounts WHERE username = $1",
        username
    ).fetch_one(pool).await.map_err(|e| DatabaseError::SqlxError(e))?;

    Ok(AccountRow { username: row.username, password_hash: row.password_hash, current_room_id: row.current_room_id })
}

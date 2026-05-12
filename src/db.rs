use sqlx::PgPool;

use crate::model::{player::Player, world::RoomId};

pub enum DatabaseError {
    SqlxError(sqlx::Error)
}

pub async fn get_player(pool: &PgPool, username: &str) -> Result<Option<Player>, DatabaseError> {
    let player = sqlx::query!(
        "SELECT id, username, password_hash, current_room_id FROM accounts WHERE username = $1",
        username
    ).fetch_optional(pool).await.map_err(|e| DatabaseError::SqlxError(e))?;

    match player {
        Some(row) => Ok(Some(Player::new(row.username, RoomId::new(row.current_room_id)))),
        None => Ok(None)
    }
}

pub async fn create_player(pool: &PgPool, username: &str) -> Result<Player, DatabaseError> {
    sqlx::query!(
        "INSERT INTO accounts (username, password_hash) VALUES ($1, $2)",
        username,
        ""
    ).execute(pool).await.map_err(|e| DatabaseError::SqlxError(e))?;

    let row = sqlx::query!(
        "SELECT id, username, password_hash, current_room_id FROM accounts WHERE username = $1",
        username
    ).fetch_one(pool).await.map_err(|e| DatabaseError::SqlxError(e))?;

    Ok(Player::new(row.username, RoomId::new(row.current_room_id)))
}

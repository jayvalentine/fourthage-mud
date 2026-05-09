use std::io::Error;

use crate::model::world::Room;

pub fn get_rooms(file: &str) -> Result<Vec<Room>, Error> {
    let json = std::fs::read_to_string(file)?;
    let json: Vec<Room> = serde_json::from_str(&json)?;
    Ok(json)
}

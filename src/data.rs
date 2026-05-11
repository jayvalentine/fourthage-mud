use std::fmt;

use crate::model::world::Room;

#[derive(Debug)]
pub enum DataLoadError {
    FileRead,
    Deserialization
}

impl fmt::Display for DataLoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DataLoadError::FileRead => write!(f, "Error loading data file"),
            DataLoadError::Deserialization => write!(f, "Error deserializing data from file")
        }
    }
}

impl From<std::io::Error> for DataLoadError {
    fn from(_: std::io::Error) -> DataLoadError {
        DataLoadError::FileRead
    }
}

impl From<serde_json::Error> for DataLoadError {
    fn from(_: serde_json::Error) -> DataLoadError {
        DataLoadError::Deserialization
    }
}

pub fn get_rooms(file: &str) -> Result<Vec<Room>, DataLoadError> {
    let json = std::fs::read_to_string(file)?;
    let json: Vec<Room> = serde_json::from_str(&json)?;
    Ok(json)
}

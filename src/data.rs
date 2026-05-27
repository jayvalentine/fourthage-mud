use std::collections::HashMap;

use serde::de::Error;
use serde::Deserialize;
use uuid::Uuid;

use crate::model::{ids::RoomId, world::Room};

#[derive(Debug)]
pub enum DataLoadError {
    FileRead,
    Deserialization,
    UuidDeserialization
}

impl From<std::io::Error> for DataLoadError {
    fn from(_: std::io::Error) -> DataLoadError {
        DataLoadError::FileRead
    }
}

impl From<serde_yaml::Error> for DataLoadError {
    fn from(_: serde_yaml::Error) -> DataLoadError {
        DataLoadError::Deserialization
    }
}

impl From<uuid::Error> for DataLoadError {
    fn from(_: uuid::Error) -> Self {
        DataLoadError::UuidDeserialization
    }
}

impl<'de> Deserialize<'de> for RoomId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        let id = Uuid::parse_str(&s).map_err(|_| D::Error::custom(format!("Invalid RoomId: {s}")))?;
        Ok(RoomId::from_uuid(id))
    }
}

pub fn get_rooms(file: &str) -> Result<HashMap<RoomId, Room>, DataLoadError> {
    let yaml = std::fs::read_to_string(file)?;
    let yaml: HashMap<RoomId, Room> = serde_yaml::from_str(&yaml)?;

    Ok(yaml)
}

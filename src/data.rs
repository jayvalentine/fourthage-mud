use std::{collections::HashMap, sync::Arc};

use serde::{Serialize, de::Error};
use serde::Deserialize;
use uuid::Uuid;

use crate::model::ids::EntityId;
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

#[derive(Debug)]
pub enum DataWriteError {
    FileWrite,
    Serialization
}

impl From<std::io::Error> for DataWriteError {
    fn from(_: std::io::Error) -> DataWriteError {
        DataWriteError::FileWrite
    }
}

impl From<serde_yaml::Error> for DataWriteError {
    fn from(_: serde_yaml::Error) -> DataWriteError {
        DataWriteError::Serialization
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

impl Serialize for RoomId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let s = self.as_uuid().to_string();
        String::serialize(&s, serializer)
    }
}

impl<'de> Deserialize<'de> for EntityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        let id = Uuid::parse_str(&s).map_err(|_| D::Error::custom(format!("Invalid EntityId: {s}")))?;
        Ok(EntityId::from_uuid(id))
    }
}

impl Serialize for EntityId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let s = self.as_uuid().to_string();
        String::serialize(&s, serializer)
    }
}

pub fn get_rooms(file: &str) -> Result<HashMap<RoomId, Room>, DataLoadError> {
    let yaml = std::fs::read_to_string(file)?;
    let yaml: HashMap<RoomId, Room> = serde_yaml::from_str(&yaml)?;

    Ok(yaml)
}

pub fn save_rooms(file: &str, rooms: &HashMap<RoomId, Arc<Room>>) -> Result<(), DataWriteError> {
    let yaml = serde_yaml::to_string(rooms)?;
    std::fs::write(file, yaml)?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct ItemData {
    pub alias: String,
    pub name: String,
    pub spawn_location: EntityId
}

pub fn save_items(file: &str, items: &HashMap<EntityId, ItemData>) -> Result<(), DataWriteError> {
    let yaml = serde_yaml::to_string(items)?;
    std::fs::write(file, yaml)?;
    Ok(())
}
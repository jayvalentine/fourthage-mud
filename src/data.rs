use std::collections::HashMap;

use serde::{Serialize, de::Error};
use serde::Deserialize;
use uuid::Uuid;

use crate::model::ids::{Alias, EntityId};
use crate::model::ids::RoomId;
use crate::model::rooms::Direction;

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

impl<'de> Deserialize<'de> for Direction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Direction::from_string(&s).map_err(|_| D::Error::custom(format!("Invalid direction: {s}")))
    }
}

impl Serialize for Direction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let s = format!("{}", self);
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

#[derive(Serialize, Deserialize)]
pub struct RoomData {
    pub alias: Alias,
    pub name: String,
    pub description: String,
    pub exits: HashMap<Direction, RoomId>
}

pub fn load_rooms(file: &str) -> Result<HashMap<RoomId, RoomData>, DataLoadError> {
    let yaml = std::fs::read_to_string(file)?;
    let yaml = serde_yaml::from_str(&yaml)?;

    Ok(yaml)
}

pub fn save_rooms(file: &str, rooms: &HashMap<RoomId, RoomData>) -> Result<(), DataWriteError> {
    let yaml = serde_yaml::to_string(rooms)?;
    std::fs::write(file, yaml)?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct ItemData {
    pub alias: Alias,
    pub name: String,
    pub description: String,
    pub spawn_location: Alias
}

pub fn load_items(file: &str) -> Result<HashMap<EntityId, ItemData>, DataLoadError> {
    let yaml = std::fs::read_to_string(file)?;
    Ok(serde_yaml::from_str(&yaml)?)
}

pub fn save_items(file: &str, items: &HashMap<EntityId, ItemData>) -> Result<(), DataWriteError> {
    let yaml = serde_yaml::to_string(items)?;
    std::fs::write(file, yaml)?;
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct NpcData {
    pub alias: Alias,
    pub name: String,
    pub description: String,
    pub spawn_location: Alias
}

pub fn load_npcs(file: &str) -> Result<HashMap<EntityId, NpcData>, DataLoadError> {
    let yaml = std::fs::read_to_string(file)?;
    Ok(serde_yaml::from_str(&yaml)?)
}

pub fn save_npcs(file: &str, items: &HashMap<EntityId, NpcData>) -> Result<(), DataWriteError> {
    let yaml = serde_yaml::to_string(items)?;
    std::fs::write(file, yaml)?;
    Ok(())
}

use std::fmt::Display;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct EntityId(Uuid);

impl EntityId {
    pub fn from_uuid(id: Uuid) -> EntityId {
        EntityId(id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Generate a new unique ID
    pub fn generate() -> EntityId {
        let uuid = uuid::Uuid::now_v7();
        EntityId(uuid)
    }
}

impl Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_uuid())
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct RoomId(EntityId);

impl RoomId {
    pub fn from_entity(id: EntityId) -> RoomId {
        RoomId(id)
    }

    pub fn from_uuid(id: Uuid) -> RoomId {
        RoomId(EntityId(id))
    }

    /// Generate a new unique ID.
    pub fn generate() -> RoomId {
        RoomId(EntityId::generate())
    }

    pub fn as_entity(&self) -> EntityId {
        self.0
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0.as_uuid()
    }
}

impl Display for RoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_uuid())
    }
}


#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Alias(String);

impl From<String> for Alias {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Alias {
    fn from(value: &str) -> Self {
        Alias(value.to_string())
    }
}

impl Display for Alias {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for Alias {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Ok(Alias::from(s))
    }
}

impl Serialize for Alias {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        String::serialize(&self.0, serializer)
    }
}

use std::{collections::HashMap, sync::{Mutex, PoisonError}};

use crate::{event::{EventTarget, EventTargetResolver}, model::world::RoomId};

#[derive(Clone)]
pub struct Position {
    pub room: RoomId
}

pub enum EntityRegistryError {
    InvalidMutex,
    UnknownEntity(String),
    DuplicateSpawn(String)
}

impl<T> From<PoisonError<T>> for EntityRegistryError {
    fn from(_: PoisonError<T>) -> Self {
        EntityRegistryError::InvalidMutex
    }
}

pub struct EntityRegistry {
    positions: Mutex<HashMap<String, Position>>
}

impl EntityRegistry {
    pub fn new() -> EntityRegistry {
        EntityRegistry { positions: Mutex::new(HashMap::new()) }
    }

    pub fn spawn(&self, name: String, starting_room: RoomId) -> Result<(), EntityRegistryError> {
        let mut positions = self.positions.lock()?;
        if positions.contains_key(&name) {
            return Err(EntityRegistryError::DuplicateSpawn(name));
        }
        positions.insert(name, Position { room: starting_room });
        Ok(())
    }

    pub fn despawn(&self, name: &str) -> Result<(), EntityRegistryError> {
        self.positions.lock()?.remove(name);
        Ok(())
    }

    pub fn update_position(&self, name: &str, new_position: RoomId) -> Result<(), EntityRegistryError> {
        let mut positions = self.positions.lock()?;
        let position = positions.get_mut(name).ok_or(EntityRegistryError::UnknownEntity(name.into()))?;
        position.room = new_position;
        Ok(())
    }

    pub fn get_position(&self, name: &str) -> Result<Position, EntityRegistryError> {
        let positions = self.positions.lock()?;
        let position = positions.get(name).ok_or(EntityRegistryError::UnknownEntity(name.into()))?;
        Ok(position.clone())
    }
}

impl EventTargetResolver<EntityRegistryError> for EntityRegistry {
    fn resolve(&self, target: &EventTarget) -> Result<Vec<String>, EntityRegistryError> {
        match target {
            EventTarget::Player(s) => Ok(vec![s.into()]),
            EventTarget::RoomExcept(id, s) => Ok(self.positions.lock()?.iter().filter(|v| v.1.room.eq(id) && !v.0.eq(s)).map(|v| v.0.to_string()).collect())
        }
    }
}

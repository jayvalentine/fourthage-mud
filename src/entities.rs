use std::{collections::{HashMap, HashSet}, sync::{Mutex, PoisonError}};

use crate::{event::{EventTarget, EventTargetResolver}, model::world::RoomId};

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Position {
    pub room: RoomId
}

struct PositionMap {
    position_by_id: HashMap<String, Position>,
    id_by_position: HashMap<Position, HashSet<String>>
}

impl PositionMap {
    pub fn new() -> PositionMap {
        PositionMap {
            position_by_id: HashMap::new(),
            id_by_position: HashMap::new()
        }
    }

    pub fn get_position<'a>(&'a self, id: &str) -> Option<&'a Position> {
        self.position_by_id.get(id)
    }

    pub fn update_position(&mut self, id: &str, new_position: Position) {
        // Remove entry from set for old position if present.
        // Entity may not be present, e.g. if this is the first time the position is being set.
        if let Some(p) = self.position_by_id.get(id) {
            if let Some(entry) = self.id_by_position.get_mut(p) {
                entry.remove(id);
            }
        }

        // Add entity to map for new position.
        self.id_by_position
            .entry(new_position.clone()).or_default()
            .insert(id.into());

        // Update position in ID mapping.
        self.position_by_id.insert(id.into(), new_position);
    }

    /// Remove the position of the given entity from the map.
    pub fn remove_position(&mut self, id: &str) {
        if let Some(p) = self.position_by_id.get(id) {
            if let Some(entry) = self.id_by_position.get_mut(p) {
                entry.remove(id);
            }
        }

        self.position_by_id.remove(id);
    }

    pub fn get_at_position(&self, position: &Position) -> Option<&HashSet<String>> {
        self.id_by_position.get(position)
    }
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

struct EntityRegistryInternal {
    entities: HashSet<String>,
    positions: PositionMap
}

pub struct EntityRegistry {
    internal: Mutex<EntityRegistryInternal>
}

impl EntityRegistry {
    pub fn new() -> EntityRegistry {
        let internal = EntityRegistryInternal {
            entities: HashSet::new(),
            positions: PositionMap::new()
        };
        EntityRegistry {
            internal: Mutex::new(internal)
        }
    }

    pub fn spawn(&self, name: String, starting_room: RoomId) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.lock()?;

        if internal.entities.contains(&name) {
            return Err(EntityRegistryError::DuplicateSpawn(name));
        }

        internal.entities.insert(name.clone());
        internal.positions.update_position(&name, Position { room: starting_room });
        Ok(())
    }

    pub fn despawn(&self, name: &str) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.lock()?;

        internal.entities.remove(name);
        internal.positions.remove_position(name);
        Ok(())
    }

    pub fn update_position(&self, name: &str, new_position: RoomId) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.lock()?;

        internal.positions.update_position(name, Position { room: new_position });
        Ok(())
    }

    pub fn get_position(&self, name: &str) -> Result<Position, EntityRegistryError> {
        let internal = self.internal.lock()?;

        let position = internal.positions.get_position(name).ok_or(EntityRegistryError::UnknownEntity(name.into()))?;
        Ok(position.clone())
    }
}

impl EventTargetResolver<EntityRegistryError> for EntityRegistry {
    fn resolve(&self, target: &EventTarget) -> Result<Vec<String>, EntityRegistryError> {
        match target {
            EventTarget::Player(s) => Ok(vec![s.into()]),
            EventTarget::RoomExcept(id, s) => {
                let internal = self.internal.lock()?;

                let targets = match internal.positions.get_at_position(&Position { room: id.clone() }) {
                    Some(entities) => entities.iter().map(|e| e.clone()).filter(|e| e != s).collect(),
                    None => Vec::new()
                };
                Ok(targets)
            }
        }
    }
}

use core::fmt;
use std::{collections::{HashMap, HashSet}, sync::{Mutex, PoisonError}};

use crate::{event::{EventTarget, EventTargetResolver}, model::world::RoomId};

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct EntityId(u32);

impl EntityId {
    pub fn new(id: u32) -> EntityId {
        EntityId(id)
    }
}

struct PositionMap {
    position_by_id: HashMap<EntityId, Position>,
    id_by_position: HashMap<Position, HashSet<EntityId>>
}

impl PositionMap {
    pub fn new() -> PositionMap {
        PositionMap {
            position_by_id: HashMap::new(),
            id_by_position: HashMap::new()
        }
    }

    pub fn update_position(&mut self, id: &EntityId, new_position: Position) {
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
            .insert(id.clone());

        // Update position in ID mapping.
        self.position_by_id.insert(id.clone(), new_position);
    }

    /// Remove the position of the given entity from the map.
    pub fn remove_position(&mut self, id: &EntityId) {
        if let Some(p) = self.position_by_id.get(id) {
            if let Some(entry) = self.id_by_position.get_mut(p) {
                entry.remove(id);
            }
        }

        self.position_by_id.remove(id);
    }

    pub fn get_at_position(&self, position: &Position) -> Option<&HashSet<EntityId>> {
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
    entities: HashSet<EntityId>,
    next_entity: u32,
    positions: PositionMap,
    names: HashMap<EntityId, Name>
}

pub trait ComponentStorage {
    fn get<'a>(entities: &'a EntityRegistryInternal, entity: &EntityId) -> Option<&'a Self>
    where Self: Sized;

    fn update(entities: &mut EntityRegistryInternal, entity: &EntityId, component: Self)
    where Self: Sized;

    fn remove(entities: &mut EntityRegistryInternal, entity: &EntityId)
    where Self: Sized;
}

pub struct EntityRegistry {
    internal: Mutex<EntityRegistryInternal>,
}

impl EntityRegistry {
    pub fn new() -> EntityRegistry {
        let internal = EntityRegistryInternal {
            entities: HashSet::new(),
            next_entity: 0,
            positions: PositionMap::new(),
            names: HashMap::new()
        };
        EntityRegistry {
            internal: Mutex::new(internal)
        }
    }

    pub fn spawn(&self) -> Result<EntityId, EntityRegistryError> {
        let mut internal = self.internal.lock()?;

        let id = internal.next_entity;
        internal.next_entity += 1;

        Ok(EntityId::new(id))
    }

    pub fn despawn(&self, id: &EntityId) -> Result<(), EntityRegistryError> {
        self.internal.lock()?.entities.remove(id);

        // When new components are added, ensure they are handled here.
        self.remove_component::<Position>(id)?;
        self.remove_component::<Name>(id)?;

        Ok(())
    }

    pub fn get_component<T: ComponentStorage + Clone>(&self, e: &EntityId) -> Result<Option<T>, EntityRegistryError> {
        let internal = self.internal.lock()?;
        Ok(T::get(&internal, e).cloned())
    }

    pub fn remove_component<T: ComponentStorage>(&self, e: &EntityId) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.lock()?;
        T::remove(&mut internal, e);
        Ok(())
    }

    pub fn update_component<T: ComponentStorage>(&self, e: &EntityId, c: T) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.lock()?;
        T::update(&mut internal, e, c);

        Ok(())
    }

    pub fn online_players(&self) -> Result<HashSet<EntityId>, EntityRegistryError> {
        let internal = self.internal.lock()?;

        Ok(internal.entities.clone())
    }
}

impl EventTargetResolver<EntityRegistryError> for EntityRegistry {
    fn resolve(&self, target: &EventTarget) -> Result<Vec<EntityId>, EntityRegistryError> {
        match target {
            EventTarget::Entity(id) => Ok(vec![id.clone()]),
            EventTarget::RoomExcept(room_id, entity_id) => {
                let internal = self.internal.lock()?;

                let targets = match internal.positions.get_at_position(&Position { room: room_id.clone() }) {
                    Some(entities) => entities.iter().map(|e| e.clone()).filter(|e| e != entity_id).collect(),
                    None => Vec::new()
                };
                Ok(targets)
            }
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Position {
    pub room: RoomId
}

impl ComponentStorage for Position {
    fn get<'a>(entities: &'a EntityRegistryInternal, entity: &EntityId) -> Option<&'a Self>
    where Self: Sized
    {
        entities.positions.position_by_id.get(entity)
    }

    fn update(entities: &mut EntityRegistryInternal, entity: &EntityId, component: Position)
    where Self: Sized
    {
        entities.positions.update_position(entity, component);
    }

    fn remove(entities: &mut EntityRegistryInternal, entity: &EntityId)
    where Self: Sized
    {
        entities.positions.remove_position(entity);
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Name {
    pub value: String
}

impl ComponentStorage for Name {
    fn get<'a>(entities: &'a EntityRegistryInternal, entity: &EntityId) -> Option<&'a Self>
    where Self: Sized
    {
        entities.names.get(entity)
    }

    fn update(entities: &mut EntityRegistryInternal, entity: &EntityId, component: Self)
    where Self: Sized
    {
        entities.names.insert(entity.clone(), component);
    }

    fn remove(entities: &mut EntityRegistryInternal, entity: &EntityId)
    where Self: Sized
    {
        entities.names.remove(entity);
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}
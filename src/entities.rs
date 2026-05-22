use core::fmt;
use std::{collections::{HashMap, HashSet}, sync::{PoisonError, RwLock}};

use crate::{event::{EventTarget, EventTargetResolver}, model::ids::{RoomId, EntityId}};

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
    UnknownEntity(EntityId),
    DuplicateSpawn(EntityId)
}

impl<T> From<PoisonError<T>> for EntityRegistryError {
    fn from(_: PoisonError<T>) -> Self {
        EntityRegistryError::InvalidMutex
    }
}

struct EntityRegistryInternal {
    entities: HashSet<EntityId>,
    positions: PositionMap,
    names: HashMap<EntityId, Name>
}

trait ComponentStorage {
    fn get<'a>(entities: &'a EntityRegistryInternal, entity: &EntityId) -> Option<&'a Self>
    where Self: Sized;

    fn update(entities: &mut EntityRegistryInternal, entity: &EntityId, component: Self)
    where Self: Sized;

    fn remove(entities: &mut EntityRegistryInternal, entity: &EntityId)
    where Self: Sized;

    fn storage(entities: &EntityRegistryInternal) -> &HashMap<EntityId, Self>
    where Self: Sized;
}

pub struct EntityRegistry {
    internal: RwLock<EntityRegistryInternal>,
}

impl EntityRegistry {
    pub fn new() -> EntityRegistry {
        let internal = EntityRegistryInternal {
            entities: HashSet::new(),
            positions: PositionMap::new(),
            names: HashMap::new()
        };
        EntityRegistry {
            internal: RwLock::new(internal)
        }
    }

    pub fn spawn(&self, entity_id: EntityId) -> Result<EntityId, EntityRegistryError> {
        let mut internal = self.internal.write()?;
        if internal.entities.contains(&entity_id) {
            return Err(EntityRegistryError::DuplicateSpawn(entity_id))
        }
        
        internal.entities.insert(entity_id.clone());

        Ok(entity_id)
    }

    pub fn despawn(&self, id: &EntityId) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.write()?;
        Self::validate_entity(&internal, id)?;

        // When new component types are added, they must be removed here.
        Position::remove(&mut internal, id);
        Name::remove(&mut internal, id);

        internal.entities.remove(id);

        Ok(())
    }

    #[allow(private_bounds)]
    pub fn get_component<T: ComponentStorage + Clone>(&self, e: &EntityId) -> Result<Option<T>, EntityRegistryError> {
        let internal = self.internal.read()?;
        Self::validate_entity(&internal, e)?;
        Ok(T::get(&internal, e).cloned())
    }

    #[allow(private_bounds)]
    pub fn remove_component<T: ComponentStorage>(&self, e: &EntityId) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.write()?;
        Self::validate_entity(&internal, e)?;
        T::remove(&mut internal, e);
        Ok(())
    }

    #[allow(private_bounds)]
    pub fn update_component<T: ComponentStorage>(&self, e: &EntityId, c: T) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.write()?;
        Self::validate_entity(&internal, e)?;
        T::update(&mut internal, e, c);

        Ok(())
    }

    #[allow(private_bounds)]
    pub fn query<T, R, F>(&self, f: F) -> Result<R, EntityRegistryError>
    where
        T: ComponentStorage,
        F: FnOnce(&mut dyn Iterator<Item = (&EntityId, &T)>) -> Result<R, EntityRegistryError>
    {
        let internal = self.internal.read()?;

        let mut iter = T::storage(&internal).iter();
        f(&mut iter)
    }

    pub fn online_players(&self) -> Result<HashSet<EntityId>, EntityRegistryError> {
        let internal = self.internal.read()?;

        Ok(internal.entities.clone())
    }

    /// Helper function to validate if an entity ID is valid.
    fn validate_entity(internal: &EntityRegistryInternal, entity: &EntityId) -> Result<(), EntityRegistryError> {
        if internal.entities.contains(entity) {
            Ok(())
        } else {
            Err(EntityRegistryError::UnknownEntity(entity.clone()))
        }
    }
}

impl EventTargetResolver<EntityRegistryError> for EntityRegistry {
    fn resolve(&self, target: &EventTarget) -> Result<Vec<EntityId>, EntityRegistryError> {
        match target {
            EventTarget::Entity(id) => Ok(vec![id.clone()]),
            EventTarget::RoomExcept(room_id, entity_id) => {
                let internal = self.internal.read()?;

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

    fn update(entities: &mut EntityRegistryInternal, entity: &EntityId, component: Self)
    where Self: Sized
    {
        entities.positions.update_position(entity, component);
    }

    fn remove(entities: &mut EntityRegistryInternal, entity: &EntityId)
    where Self: Sized
    {
        entities.positions.remove_position(entity);
    }

    fn storage(entities: &EntityRegistryInternal) -> &HashMap<EntityId, Self>
    where Self: Sized
    {
        &entities.positions.position_by_id
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

    fn storage(entities: &EntityRegistryInternal) -> &HashMap<EntityId, Self>
    where Self: Sized
    {
        &entities.names
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}
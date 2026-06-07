use core::fmt;
use std::{collections::{HashMap, HashSet}, fmt::Display};
use parking_lot::RwLock;

use crate::{event::{EventTarget, EventTargetResolver}, model::ids::EntityId};

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

struct LocationMap {
    location_by_id: HashMap<EntityId, Location>,
    id_by_location: HashMap<Location, HashSet<EntityId>>
}

impl LocationMap {
    pub fn new() -> LocationMap {
        LocationMap {
            location_by_id: HashMap::new(),
            id_by_location: HashMap::new()
        }
    }

    pub fn update_position(&mut self, id: &EntityId, new_location: Location) {
        // Remove entry from set for old position if present.
        // Entity may not be present, e.g. if this is the first time the position is being set.
        if let Some(p) = self.location_by_id.get(id) {
            if let Some(entry) = self.id_by_location.get_mut(p) {
                entry.remove(id);
            }
        }

        // Add entity to map for new position.
        self.id_by_location
            .entry(new_location.clone()).or_default()
            .insert(id.clone());

        // Update position in ID mapping.
        self.location_by_id.insert(id.clone(), new_location);
    }

    /// Remove the position of the given entity from the map.
    pub fn remove_position(&mut self, id: &EntityId) {
        if let Some(p) = self.location_by_id.get(id) {
            if let Some(entry) = self.id_by_location.get_mut(p) {
                entry.remove(id);
            }
        }

        self.location_by_id.remove(id);
    }

    pub fn get_at_position(&self, position: &Location) -> Option<&HashSet<EntityId>> {
        self.id_by_location.get(position)
    }
}

#[derive(Debug)]
pub enum EntityRegistryError {
    UnknownEntity(EntityId),
    DuplicateSpawn(EntityId),
    DuplicateAlias(Alias),
    InconsistentInternalState
}

struct EntityRegistryInternal {
    id_to_alias: HashMap<EntityId, Alias>,
    alias_to_id: HashMap<Alias, EntityId>, 
    locations: LocationMap,
    spawn_locations: HashMap<EntityId, SpawnLocation>,
    names: HashMap<EntityId, Name>,
    players: HashMap<EntityId, Player>,
    items: HashMap<EntityId, Item>,
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
            id_to_alias: HashMap::new(),
            alias_to_id: HashMap::new(),
            locations: LocationMap::new(),
            spawn_locations: HashMap::new(),
            names: HashMap::new(),
            players: HashMap::new(),
            items: HashMap::new()
        };
        EntityRegistry {
            internal: RwLock::new(internal)
        }
    }

    pub fn spawn(&self, entity_id: Option<EntityId>, alias: Alias) -> Result<EntityId, EntityRegistryError> {
        let mut internal = self.internal.write();

        let id = match entity_id {
            Some(id) => {
                if internal.id_to_alias.contains_key(&id) {
                    return Err(EntityRegistryError::DuplicateSpawn(id))
                }
                id
            },
            None => {
                EntityId::generate()
            }
        };

        internal.id_to_alias.insert(id.clone(), alias.clone());

        if internal.alias_to_id.contains_key(&alias) {
            return Err(EntityRegistryError::DuplicateAlias(alias))
        }

        internal.alias_to_id.insert(alias, id.clone());
        Ok(id)
    }

    pub fn despawn(&self, id: &EntityId) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.write();
        Self::validate_entity(&internal, id)?;

        // When new component types are added, they must be removed here.
        Location::remove(&mut internal, id);
        Name::remove(&mut internal, id);
        Player::remove(&mut internal, id);
        Item::remove(&mut internal, id);

        let alias = internal.id_to_alias.get(id).unwrap().clone();
        internal.alias_to_id.remove(&alias);
        internal.id_to_alias.remove(id);

        Ok(())
    }

    #[allow(private_bounds)]
    pub fn get_alias(&self, e: &EntityId) -> Result<Alias, EntityRegistryError> {
        let internal = self.internal.read();
        Self::validate_entity(&internal, e)?;
        let alias = internal.id_to_alias.get(e).ok_or(EntityRegistryError::InconsistentInternalState)?;
        Ok(alias.clone())
    }

    #[allow(private_bounds)]
    pub fn has_component<T: ComponentStorage>(&self, e: &EntityId) -> Result<bool, EntityRegistryError> {
        let internal = self.internal.read();
        Self::validate_entity(&internal, e)?;
        Ok(T::get(&internal, e).is_some())
    }

    #[allow(private_bounds)]
    pub fn get_component<T: ComponentStorage + Clone>(&self, e: &EntityId) -> Result<Option<T>, EntityRegistryError> {
        let internal = self.internal.read();
        Self::validate_entity(&internal, e)?;
        Ok(T::get(&internal, e).cloned())
    }

    #[allow(private_bounds)]
    pub fn remove_component<T: ComponentStorage>(&self, e: &EntityId) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.write();
        Self::validate_entity(&internal, e)?;
        T::remove(&mut internal, e);
        Ok(())
    }

    #[allow(private_bounds)]
    pub fn update_component<T: ComponentStorage>(&self, e: &EntityId, c: T) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.write();
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
        let internal = self.internal.read();

        let mut iter = T::storage(&internal).iter();
        f(&mut iter)
    }

    #[allow(private_bounds)]
    pub fn query_location<T, R, F>(&self, location: &Location, f: F) -> Result<R, EntityRegistryError>
    where
        T: ComponentStorage + Clone,
        F: FnOnce(&mut dyn Iterator<Item = (&EntityId, &T)>) -> Result<R, EntityRegistryError>
    {
        let internal = self.internal.read();

        let entities_in_location = internal.locations.id_by_location.get(location);

        let storage = T::storage(&internal);
        let mut iter = entities_in_location.iter()
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| storage.get(id).map(|c| (id, c)));

        f(&mut iter)
    }

    #[allow(private_bounds)]
    pub fn query2<T1, T2, R, F>(&self, f: F) -> Result<R, EntityRegistryError>
    where
        T1: ComponentStorage,
        T2: ComponentStorage,
        F: FnOnce(&mut dyn Iterator<Item = (&EntityId, (&T1, &T2))>) -> Result<R, EntityRegistryError>
    {
        let internal = self.internal.read();

        let storage1 = T1::storage(&internal);
        let storage2 = T2::storage(&internal);

        let mut iter: Box<dyn Iterator<Item = (&EntityId, (&T1, &T2))>> = if storage1.len() <= storage2.len() {
            Box::new(storage1.iter()
                .filter(|(id, _)| storage2.contains_key(id))
                .map(|(id, c1)| (id, (c1, storage2.get(id).unwrap()))))
        } else {
            Box::new(storage2.iter()
                .filter(|(id, _)| storage1.contains_key(id))
                .map(|(id, c2)| (id, (storage1.get(id).unwrap(), c2))))
        };

        f(&mut iter)
    }

    #[allow(private_bounds)]
    pub fn query3<T1, T2, T3, R, F>(&self, f: F) -> Result<R, EntityRegistryError>
    where
        T1: ComponentStorage,
        T2: ComponentStorage,
        T3: ComponentStorage,
        F: FnOnce(&mut dyn Iterator<Item = (&EntityId, (&T1, &T2, &T3))>) -> Result<R, EntityRegistryError>
    {
        let internal = self.internal.read();

        let storage1 = T1::storage(&internal);
        let storage2 = T2::storage(&internal);
        let storage3 = T3::storage(&internal);

        let min_len = storage1.len().min(storage2.len()).min(storage3.len());

        let mut iter: Box<dyn Iterator<Item = (&EntityId, (&T1, &T2, &T3))>> = if min_len == storage1.len() {
            let iter = storage1.iter()
                .filter(|(id, _)| storage2.contains_key(id) && storage3.contains_key(id))
                .filter_map(|(id, c1)| {
                    Some((id, (c1, storage2.get(id)?, storage3.get(id)?)))
                });
            Box::new(iter)
        } else if min_len == storage2.len() {
            let iter = storage2.iter()
                .filter(|(id, _)| storage1.contains_key(id) && storage3.contains_key(id))
                .filter_map(|(id, c2)| {
                    Some((id, (storage1.get(id)?, c2, storage3.get(id)?)))
                });
            Box::new(iter)
        } else {
            let iter = storage3.iter()
                .filter(|(id, _)| storage1.contains_key(id) && storage2.contains_key(id))
                .filter_map(|(id, c3)| {
                    Some((id, (storage1.get(id)?, storage2.get(id)?, c3)))
                });
            Box::new(iter)
        };

        f(&mut iter)
    }

    pub fn resolve_alias(&self, alias: &Alias) -> Option<EntityId> {
        let internal = self.internal.read();

        internal.alias_to_id.get(alias).cloned()
    }

    /// Helper function to validate if an entity ID is valid.
    fn validate_entity(internal: &EntityRegistryInternal, entity: &EntityId) -> Result<(), EntityRegistryError> {
        if internal.id_to_alias.contains_key(entity) {
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
            EventTarget::LocationExcept(location, entity_id) => {
                let internal = self.internal.read();

                let targets = match internal.locations.get_at_position(&location) {
                    Some(entities) => entities.iter().map(|e| e.clone()).filter(|e| e != entity_id).collect(),
                    None => Vec::new()
                };
                Ok(targets)
            }
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct Location {
    pub value: EntityId
}

impl ComponentStorage for Location {
    fn get<'a>(entities: &'a EntityRegistryInternal, entity: &EntityId) -> Option<&'a Self>
    where Self: Sized
    {
        entities.locations.location_by_id.get(entity)
    }

    fn update(entities: &mut EntityRegistryInternal, entity: &EntityId, component: Self)
    where Self: Sized
    {
        entities.locations.update_position(entity, component);
    }

    fn remove(entities: &mut EntityRegistryInternal, entity: &EntityId)
    where Self: Sized
    {
        entities.locations.remove_position(entity);
    }

    fn storage(entities: &EntityRegistryInternal) -> &HashMap<EntityId, Self>
    where Self: Sized
    {
        &entities.locations.location_by_id
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

pub struct Player;

impl ComponentStorage for Player {
    fn get<'a>(entities: &'a EntityRegistryInternal, entity: &EntityId) -> Option<&'a Self>
    where Self: Sized
    {
        entities.players.get(entity)
    }

    fn remove(entities: &mut EntityRegistryInternal, entity: &EntityId)
    where Self: Sized
    {
        entities.players.remove(entity);
    }

    fn update(entities: &mut EntityRegistryInternal, entity: &EntityId, component: Self)
    where Self: Sized
    {
        entities.players.insert(entity.clone(), component);
    }

    fn storage(entities: &EntityRegistryInternal) -> &HashMap<EntityId, Self>
    where Self: Sized
    {
        &entities.players
    }
}

/// Marker component for items.
pub struct Item;

impl ComponentStorage for Item {
    fn get<'a>(entities: &'a EntityRegistryInternal, entity: &EntityId) -> Option<&'a Self>
    where Self: Sized
    {
        entities.items.get(entity)
    }

    fn remove(entities: &mut EntityRegistryInternal, entity: &EntityId)
    where Self: Sized
    {
        entities.items.remove(entity);
    }

    fn update(entities: &mut EntityRegistryInternal, entity: &EntityId, component: Self)
    where Self: Sized
    {
        entities.items.insert(entity.clone(), component);
    }

    fn storage(entities: &EntityRegistryInternal) -> &HashMap<EntityId, Self>
    where Self: Sized
    {
        &entities.items
    }
}

pub struct SpawnLocation {
    pub value: EntityId
}

impl ComponentStorage for SpawnLocation {
    fn get<'a>(entities: &'a EntityRegistryInternal, entity: &EntityId) -> Option<&'a Self>
    where Self: Sized
    {
        entities.spawn_locations.get(entity)
    }

    fn remove(entities: &mut EntityRegistryInternal, entity: &EntityId)
    where Self: Sized
    {
        entities.spawn_locations.remove(entity);
    }

    fn update(entities: &mut EntityRegistryInternal, entity: &EntityId, component: Self)
    where Self: Sized
    {
        entities.spawn_locations.insert(entity.clone(), component);
    }

    fn storage(entities: &EntityRegistryInternal) -> &HashMap<EntityId, Self>
    where Self: Sized
    {
        &entities.spawn_locations
    }
}

impl From<Location> for SpawnLocation {
    fn from(value: Location) -> Self {
        Self { value: value.value }
    }
}

impl From<&Location> for SpawnLocation {
    fn from(value: &Location) -> Self {
        Self { value: value.value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ids::RoomId;

    /// Tests that components can be updated and retrieved for entities.
    #[test]
    fn test_update_component() {
        let entities = EntityRegistry::new();

        let e1 = entities.spawn(None, "e1".into()).unwrap();
        let e2 = entities.spawn(None, "e2".into()).unwrap();

        entities.update_component(&e1, Name { value: "entity 1".to_string() }).unwrap();

        let name1 = entities.get_component::<Name>(&e1).unwrap().unwrap();
        assert_eq!("entity 1", name1.value);

        let name2 = entities.get_component::<Name>(&e2).unwrap();
        assert!(name2.is_none())
    }

    /// Tests that components can be queried by location.
    #[test]
    fn test_get_component_by_location() {
        let entities = EntityRegistry::new();

        let e1 = entities.spawn(None, "e1".into()).unwrap();
        let e2 = entities.spawn(None, "e2".into()).unwrap();
        let e3 = entities.spawn(None, "e3".into()).unwrap();

        let room1 = RoomId::generate();
        let room2 = RoomId::generate();
        let loc1 = Location { value: room1.as_entity() };
        let loc2 = Location { value: room2.as_entity() };

        entities.update_component(&e1, loc1.clone()).unwrap();
        entities.update_component(&e1, Name { value: "entity 1".to_string() }).unwrap();
        entities.update_component(&e2, loc1.clone()).unwrap();
        entities.update_component(&e3, loc2.clone()).unwrap();
        entities.update_component(&e3, Name { value: "entity 3".to_string() }).unwrap();

        entities.query_location::<Name, _, _>(&loc1, |iter| {
            let (e, n) = iter.next().unwrap();

            // Only one entity is expected since only one exists in the location with a Name.
            assert_eq!(&e1, e);
            assert_eq!("entity 1", n.value);

            assert!(iter.next().is_none());
            Ok(())
        }).unwrap();
    }

    #[test]
    fn test_query3() {
        let entities = EntityRegistry::new();

        let e1 = entities.spawn(None, "e1".into()).unwrap();
        let e2 = entities.spawn(None, "e2".into()).unwrap();
        let e3 = entities.spawn(None, "e3".into()).unwrap();

        let loc = Location { value: RoomId::generate().as_entity() };
        let name = Name { value: "Some Name".into() };

        // e1 has location and name but not item.
        entities.update_component(&e1, loc.clone()).unwrap();
        entities.update_component(&e1, name.clone()).unwrap();

        // e2 has name and item but not location.
        entities.update_component(&e2, name.clone()).unwrap();
        entities.update_component(&e2, Item).unwrap();

        // e3 has all three components.
        entities.update_component(&e3, loc.clone()).unwrap();
        entities.update_component(&e3, name.clone()).unwrap();
        entities.update_component(&e3, Item).unwrap();

        entities.query3::<Name, Item, Location, _, _>(|iter| {
            let (e, _) = iter.next().unwrap();

            // Only one entity is expected since only one has all three components.
            assert_eq!(&e3, e);

            assert!(iter.next().is_none());
            Ok(())
        }).unwrap();
    }
}

use core::fmt;
use std::{any::TypeId, collections::{HashMap, HashSet}};
use parking_lot::RwLock;
use fourthage_mud_macros::ComponentStorage;

use crate::{event::{EventTarget, EventTargetResolver}, model::ids::{Alias, EntityId}};

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
    descriptions: HashMap<EntityId, Description>,
    players: HashMap<EntityId, Player>,
    items: HashMap<EntityId, Item>,

    dirty: HashMap<TypeId, HashSet<EntityId>>
}

trait ComponentStorage: 'static {
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
            descriptions: HashMap::new(),
            players: HashMap::new(),
            items: HashMap::new(),
            dirty: HashMap::new()
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

    /// Removes the component of type T from the entity with the given ID.
    /// If the entity does not have a component of type T, this function does nothing.
    ///
    /// Removes any dirty tracking for the component type T for the given entity.
    #[allow(private_bounds)]
    pub fn remove_component<T: ComponentStorage>(&self, e: &EntityId) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.write();
        Self::validate_entity(&internal, e)?;
        T::remove(&mut internal, e);
        if let Some(entities) = internal.dirty.get_mut(&TypeId::of::<T>()) {
            entities.remove(e);
        }
        Ok(())
    }

    /// Updates the component of type T for the entity with the given ID.
    /// If the entity does not have a component of type T, this function adds it.
    ///
    /// Automatically adds a dirty tracker for this component.
    #[allow(private_bounds)]
    pub fn update_component<T: ComponentStorage>(&self, e: &EntityId, c: T) -> Result<(), EntityRegistryError> {
        let mut internal = self.internal.write();
        Self::validate_entity(&internal, e)?;
        T::update(&mut internal, e, c);
        internal.dirty.entry(TypeId::of::<T>()).or_default().insert(*e);

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
    pub fn query2_location<T1, T2, R, F>(&self, location: &Location, f: F) -> Result<R, EntityRegistryError>
    where
        T1: ComponentStorage + Clone,
        T2: ComponentStorage + Clone,
        F: FnOnce(&mut dyn Iterator<Item = (&EntityId, (&T1, &T2))>) -> Result<R, EntityRegistryError>
    {
        let internal = self.internal.read();

        let entities_in_location = internal.locations.id_by_location.get(location);

        let storage1 = T1::storage(&internal);
        let storage2 = T2::storage(&internal);

        let mut iter = entities_in_location.iter()
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| storage1.get(id).map(|c| (id, c)))
            .filter_map(|(id, c1)| storage2.get(id).map(|c2| (id, (c1, c2))));

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

    /// Returns ID/component pairs for all entities with pending changes.
    /// Clears all dirty flags for that component.
    #[allow(private_bounds)]
    pub fn take_dirty<T: ComponentStorage + Clone>(&self) -> Vec<(EntityId, T)> {
        let mut internal = self.internal.write();

        let dirty_ids = internal.dirty.remove(&TypeId::of::<T>()).unwrap_or_default();
        let storage = T::storage(&internal);

        dirty_ids.into_iter()
                 .filter_map(|id| storage.get(&id).map(|c| (id, c.clone())))
                 .collect()
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

impl Location {
    pub fn new(id: EntityId) -> Location {
        Location { value: id }
    }
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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[derive(ComponentStorage)]
#[component(field = "names")]
pub struct Name(String);

impl From<String> for Name {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl Name {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(ComponentStorage)]
#[component(field = "players")]
pub struct Player;

/// Marker component for items.
#[derive(Clone)]
#[derive(ComponentStorage)]
#[component(field = "items")]
pub struct Item;

#[derive(ComponentStorage)]
#[component(field = "spawn_locations")]
pub struct SpawnLocation {
    pub value: EntityId
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

#[derive(Clone)]
#[derive(ComponentStorage)]
#[component(field = "descriptions")]
pub struct Description(String);

impl From<String> for Description {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Description {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
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

        entities.update_component(&e1, Name::from("entity 1")).unwrap();

        let name1 = entities.get_component::<Name>(&e1).unwrap().unwrap();
        assert_eq!("entity 1", name1.as_str());

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
        entities.update_component(&e1, Name::from("entity 1")).unwrap();
        entities.update_component(&e2, loc1.clone()).unwrap();
        entities.update_component(&e3, loc2.clone()).unwrap();
        entities.update_component(&e3, Name::from("entity 3")).unwrap();

        entities.query_location::<Name, _, _>(&loc1, |iter| {
            let (e, n) = iter.next().unwrap();

            // Only one entity is expected since only one exists in the location with a Name.
            assert_eq!(&e1, e);
            assert_eq!("entity 1", n.as_str());

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
        let name = Name::from("Some Name");

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

    #[test]
    fn test_query2_location() {
        let entities = EntityRegistry::new();

        let e1 = entities.spawn(None, "e1".into()).unwrap();
        let e2 = entities.spawn(None, "e2".into()).unwrap();
        let e3 = entities.spawn(None, "e3".into()).unwrap();

        let room1 = RoomId::generate();
        let room2 = RoomId::generate();
        let loc1 = Location { value: room1.as_entity() };
        let loc2 = Location { value: room2.as_entity() };

        entities.update_component(&e1, loc1.clone()).unwrap();
        entities.update_component(&e1, Name::from("entity 1")).unwrap();
        entities.update_component(&e2, loc1.clone()).unwrap();
        entities.update_component(&e2, Name::from("entity 2")).unwrap();
        entities.update_component(&e2, Item).unwrap();
        entities.update_component(&e3, loc2.clone()).unwrap();
        entities.update_component(&e3, Name::from("entity 3")).unwrap();

        entities.query2_location::<Name, Item, _, _>(&loc1, |iter| {
            let (e, (n, _i)) = iter.next().unwrap();

            // Only one entity is expected since only one exists in the location
            // with both Name and Item components.
            assert_eq!(&e2, e);
            assert_eq!("entity 2", n.as_str());

            assert!(iter.next().is_none());
            Ok(())
        }).unwrap();
    }

    #[test]
    fn test_take_dirty() {
        let entities = EntityRegistry::new();

        let e1 = entities.spawn(None, "e1".into()).unwrap();
        let e2 = entities.spawn(None, "e2".into()).unwrap();

        entities.update_component(&e1, Name::from("name1")).unwrap();
        entities.update_component(&e2, Name::from("name2")).unwrap();

        let dirty = entities.take_dirty::<Name>();
        assert_eq!(2, dirty.len());
        assert!(dirty.contains(&(e1, Name::from("name1"))));
        assert!(dirty.contains(&(e2, Name::from("name2"))));

        assert_eq!(Some(Name::from("name1")), entities.get_component(&e1).unwrap());
        assert_eq!(Some(Name::from("name2")), entities.get_component(&e2).unwrap());

        entities.update_component(&e2, Name::from("name2*")).unwrap();

        let dirty = entities.take_dirty::<Name>();
        assert_eq!(1, dirty.len());
        assert!(dirty.contains(&(e2, Name::from("name2*"))));
    }
}

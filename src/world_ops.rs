use crate::{entities::{EntityRegistry, EntityRegistryError, Location, Name}, event::{Event, EventTarget, GameEvent}, model::ids::{EntityId, RoomId}};

#[derive(Debug)]
pub enum WorldOperationError {
    EntityRegistry(EntityRegistryError),
    MissingLocation(EntityId),
    MissingName(EntityId)
}

impl From<EntityRegistryError> for WorldOperationError {
    fn from(value: EntityRegistryError) -> Self {
        WorldOperationError::EntityRegistry(value)
    }
}

pub fn get_entity_location(entities: &EntityRegistry, e: &EntityId) -> Result<Location, WorldOperationError> {
    entities.get_component::<Location>(e)?
        .ok_or(WorldOperationError::MissingLocation(*e))
}

pub fn get_entity_name(entities: &EntityRegistry, e: &EntityId) -> Result<Name, WorldOperationError> {
    entities.get_component::<Name>(e)?
        .ok_or(WorldOperationError::MissingName(*e))
}

/// Move the given entity to a destination room.
/// Produces events for the left and entered locations.
pub fn move_entity(entities: &EntityRegistry, e: EntityId, destination: RoomId) -> Result<Vec<Event>, WorldOperationError> {
    let location = get_entity_location(entities, &e)?;

    let new_position = Location { value: destination.as_entity() };
    entities.update_component(&e, new_position.clone())?;

    Ok(vec![
        Event {
            target: EventTarget::LocationExcept(location, e),
            event: GameEvent::EntityLeft(e)
        },
        Event {
            target: EventTarget::LocationExcept(new_position, e),
            event: GameEvent::EntityEntered(e)
        }
    ])
}

/// Places the given item in an entities inventory.
pub fn take_item(entities: &EntityRegistry, e: EntityId, item: EntityId) -> Result<Vec<Event>, WorldOperationError> {
    let new_location = Location::new(e);
    entities.update_component(&item, new_location.clone())?;

    let item_name = get_entity_name(entities, &item)?;
    let entity_name = get_entity_name(entities, &e)?;
    let entity_location = get_entity_location(entities, &e)?;

    let message = format!("{entity_name} picked up {item_name}.");
    Ok(vec![
        Event {
            target: EventTarget::LocationExcept(entity_location, e),
            event: GameEvent::Message(message)
        }
    ])
}

/// Drops the given item in the room occupied by an entity.
pub fn drop_item(entities: &EntityRegistry, e: EntityId, item: EntityId) -> Result<Vec<Event>, WorldOperationError> {
    let new_location = get_entity_location(entities, &e)?;
    entities.update_component(&item, new_location.clone())?;

    let item_name = get_entity_name(entities, &item)?;
    let entity_name = get_entity_name(entities, &e)?;

    let message = format!("{entity_name} dropped {item_name}.");
    Ok(vec![
        Event {
            target: EventTarget::LocationExcept(new_location, e),
            event: GameEvent::Message(message)
        }
    ])
}

/// Transmits a message from the given entity to others in the same room.
pub fn say(entities: &EntityRegistry, e: EntityId, message: String) -> Result<Vec<Event>, WorldOperationError> {
    let location = get_entity_location(entities, &e)?;
    let name = get_entity_name(entities, &e)?;
    let event = Event {
        target: EventTarget::LocationExcept(location, e),
        event: GameEvent::EntitySaid(e, name, message)
    };

    Ok(vec![event])
}

/// Transmits an emote from the given entity to others in the same room.
pub fn emote(entities: &EntityRegistry, e: EntityId, emote: String) -> Result<Vec<Event>, WorldOperationError> {
    let location = get_entity_location(entities, &e)?;
    let name = get_entity_name(entities, &e)?;
    let event = Event {
        target: EventTarget::LocationExcept(location, e),
        event: GameEvent::EntityEmoted(e, name, emote)
    };

    Ok(vec![event])
}


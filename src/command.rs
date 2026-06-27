use std::collections::HashMap;
use std::fmt;

use crate::data::{ItemData, NpcData, RoomData};
use crate::db::DatabaseError;
use crate::entities::{Description, EntityRegistryError, Item, Location, Name, Npc, Player, SpawnLocation};
use crate::event::{Event, EventTarget, GameEvent};
use crate::model::rooms::{DirectionParseError, RoomGraphNode};
use crate::model::{rooms::Direction, ids::{EntityId, RoomId, Alias}};
use crate::session::SessionContext;
use crate::data;

pub struct Keywords(pub Vec<String>);

impl fmt::Display for Keywords {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}",self.0.join(" "))
    }
}

pub enum Command {
    Go(Direction),
    Say(String),
    Who,
    Look,
    Inventory,

    /// take <keywords>
    Take(Keywords),
    /// drop <keywords>
    Drop(Keywords),
    /// inspect <keywords>
    Inspect(Keywords),

    // Admin commands
    Edit(EditTarget, EditField, String),
    Save(SaveTarget, String),
    Link(Direction, Alias),
    Unlink(Direction),
    Create(Direction, Alias),
    Spawn(SpawnTarget, Alias),
    RoomInfo,

    // Session management commands
    Quit
}

pub enum SpawnTarget {
    Item,
    Npc
}

pub enum EditTarget {
    Room,
    Entity(Alias)
}

pub enum EditField {
    Description,
    Name
}

pub enum SaveTarget {
    Rooms,
    Items,
    Npcs
}

pub enum CommandParseError {
    UnknownCommand(String),
    InvalidSyntax(String),
    InvalidDirection(String)
}

impl From<DirectionParseError> for CommandParseError {
    fn from(value: DirectionParseError) -> Self {
        match value {
            DirectionParseError::Invalid(s) => CommandParseError::InvalidDirection(s)
        }
    }
}

pub struct ActionResult {
    events: Vec<Event>,
    response: Option<String>
}

impl ActionResult {
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn response(&self) -> &Option<String> {
        &self.response
    }
}

pub struct QueryResult {
    response: String
}

impl QueryResult {
    pub fn response(&self) -> &str {
        &self.response
    }
}

impl From<&str> for QueryResult {
    fn from(value: &str) -> Self {
        QueryResult { response: value.to_string() }
    }
}

impl From<String> for QueryResult {
    fn from(value: String) -> Self {
        QueryResult { response: value }
    }
}

pub enum CommandResult {
    Action(ActionResult),
    Query(QueryResult),
    Unauthorized
}

pub enum CommandExecutionError {
    /// Command could not be executed due to an unrecoverable error.
    Unrecoverable(String)
}

impl From<EntityRegistryError> for CommandExecutionError {
    fn from(value: EntityRegistryError) -> Self {
        CommandExecutionError::Unrecoverable(format!("Entity registry error: {value:?}"))
    }
}

impl From<DatabaseError> for CommandExecutionError {
    fn from(value: DatabaseError) -> Self {
        CommandExecutionError::Unrecoverable(format!("Database error: {value:?}"))
    }
}

impl Command {
    fn collect_keywords<I, S>(parts: I) -> Option<Keywords>
        where I: Iterator<Item = S>,
              S: ToString
    {
        let keywords = parts.map(|s| s.to_string().to_lowercase()).collect::<Vec<String>>();
        if keywords.is_empty() {
            None
        } else {
            Some(Keywords(keywords))
        }
    }

    pub fn parse(input: &str) -> Result<Command, CommandParseError> {
        // split input into verb and optional argument
        let mut parts = input.split_whitespace();
        let verb = parts.next().unwrap_or("").to_lowercase();

        match verb.as_str() {
            "go" => {
                let direction = match parts.next() {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("Go where?".into()))
                };
                let direction = match Direction::from_string(direction) {
                    Ok(d) => d,
                    Err(_) => return Err(CommandParseError::InvalidSyntax(format!("You can't go {direction}!")))
                };
                Ok(Command::Go(direction))
            },
            "say" => {
                let sentence = parts.collect::<Vec<&str>>().join(" ");
                let sentence = sentence.trim();
                match sentence {
                    "" => Err(CommandParseError::InvalidSyntax("Say what?".into())),
                    _ => Ok(Command::Say(sentence.into()))
                }
            },
            "who" => Ok(Command::Who),
            "look" => Ok(Command::Look),
            "inventory" => Ok(Command::Inventory),
            "take" => {
                let keywords = match Self::collect_keywords(parts) {
                    Some(k) => k,
                    None => return Err(CommandParseError::InvalidSyntax("Take what?".into()))
                };

                Ok(Command::Take(keywords))
            }
            "drop" => {
                let keywords = match Self::collect_keywords(parts) {
                    Some(k) => k,
                    None => return Err(CommandParseError::InvalidSyntax("Drop what?".into()))
                };

                Ok(Command::Drop(keywords))
            },
            "inspect" => {
                let keywords = match Self::collect_keywords(parts) {
                    Some(k) => k,
                    None => return Err(CommandParseError::InvalidSyntax("Inspect what?".into()))
                };

                Ok(Command::Inspect(keywords))
            }

            "edit" => {
                let target = match parts.next() {
                    Some("room") => EditTarget::Room,
                    Some(s) => EditTarget::Entity(s.into()),
                    None => return Err(CommandParseError::InvalidSyntax("Edit what?".into()))
                };

                let field = match parts.next() {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("Edit which field?".into()))
                };
                let field = match field {
                    "desc" | "description" => EditField::Description,
                    "name" | "title" => EditField::Name,
                    s => return Err(CommandParseError::UnknownCommand(format!("Unknown edit field '{s}'")))
                };
                let content = parts.collect::<Vec<&str>>().join(" ");
                if content.is_empty() {
                    return Err(CommandParseError::InvalidSyntax("No edit content!".into()))
                }
                Ok(Command::Edit(target, field, content))
            },
            "save" => {
                let target = match parts.next() {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("Save what?".into()))
                };
                let target = match target {
                    "rooms" => SaveTarget::Rooms,
                    "items" => SaveTarget::Items,
                    "npcs" => SaveTarget::Npcs,
                    s => return Err(CommandParseError::UnknownCommand(format!("Don't know how to save '{s}'!")))
                };
                let path = match parts.next() {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("Save to where?".into()))
                };
                Ok(Command::Save(target, path.to_string()))
            },
            "link" => {
                let direction = match parts.next() {
                    Some(s) => Direction::from_string(s)?,
                    None => return Err(CommandParseError::InvalidSyntax("Link in which direction?".into()))
                };
                let alias = match parts.next() {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("Link to where?".into()))
                };
                Ok(Command::Link(direction, Alias::from(alias)))
            },
            "unlink" => {
                let direction = match parts.next() {
                    Some(s) => Direction::from_string(s)?,
                    None => return Err(CommandParseError::InvalidSyntax("Unlink which direction?".into()))
                };
                Ok(Command::Unlink(direction))
            }
            "create" => {
                let direction = match parts.next() {
                    Some(s) => Direction::from_string(s)?,
                    None => return Err(CommandParseError::InvalidSyntax("Create in which direction?".into()))
                };
                let alias = match parts.next() {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("Create what?".into()))
                };
                Ok(Command::Create(direction, Alias::from(alias)))
            },
            "spawn" => {
                let what = match parts.next() {
                    Some("item") => SpawnTarget::Item,
                    Some("npc") => SpawnTarget::Npc,
                    Some(s) => return Err(CommandParseError::InvalidSyntax(format!("Cannot spawn {s}!"))),
                    None => return Err(CommandParseError::InvalidSyntax("Spawn what?".into()))
                };
                let alias = match parts.next() {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("With what alias?".into()))
                };
                Ok(Command::Spawn(what, Alias::from(alias)))
            }
            "roominfo" => Ok(Command::RoomInfo),

            "quit" => Ok(Command::Quit),
            _ => Err(CommandParseError::UnknownCommand(input.to_string())),
        }
    }
}

fn get_current_position(context: &SessionContext) -> Result<Location, CommandExecutionError> {
    context.entities.get_component::<Location>(&context.player_id)
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get current position of entity {:?}", &context.player_id)))?
        .ok_or(CommandExecutionError::Unrecoverable(format!("Entity {:?} has no position component", &context.player_id)))
}

fn get_room_description(context: &SessionContext, id: &RoomId) -> Result<String, CommandExecutionError> {
    let current_room = context.rooms.get_room(id)
        .ok_or(CommandExecutionError::Unrecoverable("Could not retrieve room based on current room ID".into()))?;

    let room_name = match context.entities.get_component::<Name>(&id.as_entity())? {
        Some(n) => n,
        None => "Unnamed Room".into()
    };
    let room_desc = match context.entities.get_component::<Description>(&id.as_entity())? {
        Some(n) => n,
        None => "No Description".into()
    };
    
    let exits: Vec<String> = current_room.exits().into_iter().map(|e| e.to_string()).collect();
    let exits = exits.join(", ");

    let entities: Vec<String> = context.entities.query_location::<Name, _, _>(&Location { value: id.as_entity() }, |iter| {
        Ok(iter
            .filter(|(id, _)| *id != &context.player_id)
            .map(|(_, n)| n.to_string())
            .collect())
    })?;
    let entities = if entities.is_empty() {
        "nothing".into()
    } else {
        entities.join(", ")
    };

    let response = format!("{room_name}\n\n{room_desc}\n\nHere is: {entities}\n\nFrom here you can go: {exits}\n");
    Ok(response)
}

fn get_entity_name(context: &SessionContext, entity: &EntityId) -> Result<Name, CommandExecutionError> {
    match context.entities.get_component::<Name>(entity)? {
        Some(n) => Ok(n),
        None => Err(CommandExecutionError::Unrecoverable(format!("Entity {entity} has no name (error in name resolution).")))
    }
}

fn get_entity_description(context: &SessionContext, entity: &EntityId) -> Result<Option<Description>, CommandExecutionError> {
    match context.entities.get_component::<Description>(entity)? {
        Some(d) => Ok(Some(d)),
        None => Ok(None)
    }
}

/// Resolves a set of keywords to specific entities in a given context. 
fn resolve_entities_in_location(context: &SessionContext, location: &Location, keywords: &Keywords) -> Result<Vec<EntityId>, CommandExecutionError> {
    context.entities.query_location::<Name, _, _>(location, |iter| {
        let iter = iter.filter_map(|(id, name)| {
            if keywords.0.iter().all(|k| name.as_str().to_lowercase().contains(k)) {
                Some(*id)
            } else {
                None
            }
        });
        Ok(iter.collect::<Vec<EntityId>>())
    }).map_err(CommandExecutionError::from)
}

fn resolve_entities_in_current_room(context: &SessionContext, keywords: &Keywords) -> Result<Vec<EntityId>, CommandExecutionError> {
    let location = get_current_position(context)?;
    resolve_entities_in_location(context, &location, keywords)
}

fn resolve_entities_in_inventory(context: &SessionContext, keywords: &Keywords) -> Result<Vec<EntityId>, CommandExecutionError> {
    let location = Location { value: context.player_id };
    resolve_entities_in_location(context, &location, keywords)
}

/// Resolve entities in either the current room or the player's inventory.
fn resolve_entities_in_context(context: &SessionContext, keywords: &Keywords) -> Result<Vec<EntityId>, CommandExecutionError> {
    let mut in_room = resolve_entities_in_current_room(context, keywords)?;
    let mut in_inventory = resolve_entities_in_inventory(context, keywords)?;
    in_inventory.append(&mut in_room);
    Ok(in_inventory)
}

fn get_player_name(context: &SessionContext) -> Result<Name, CommandExecutionError> {
    context.entities.get_component::<Name>(&context.player_id)
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get name for entity: {:?}", context.player_id)))?
        .ok_or(CommandExecutionError::Unrecoverable(format!("Entity {:?} had no Name component", context.player_id)))
}

async fn handle_go(context: &mut SessionContext, direction: Direction) -> Result<CommandResult, CommandExecutionError> {
    let location = get_current_position(context)?;
    let current_room_id = RoomId::from_entity(location.value);
    let current_room = context.rooms.get_room(&current_room_id)
        .ok_or(CommandExecutionError::Unrecoverable("Could not retrieve room based on current room ID".into()))?;

    let destination_room_id = match current_room.get_destination(direction) {
        Some(id) => id,
        None => return Ok(CommandResult::Query(QueryResult { response: format!("You cannot go {direction} from here.") }))
    };

    let new_position = Location { value: destination_room_id.as_entity().clone() };
    context.entities.update_component(&context.player_id, new_position.clone())
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not update position of entity '{:?}'", &context.player_id)))?;

    let description = get_room_description(context, destination_room_id)?;

    let response = format!("You go {direction}.\n\n{description}");
    let result = ActionResult { events: Vec::new(), response: Some(response) };
    Ok(CommandResult::Action(result))
}

fn handle_say(context: &SessionContext, sentence: &str) -> Result<CommandResult, CommandExecutionError> {
    let name = get_player_name(context)?;
    let position = get_current_position(context)?;

    let message = format!("{name} says: {sentence}");
    let result = ActionResult {
        events: vec![
            Event {
                target: EventTarget::LocationExcept(position, context.player_id.clone()),
                event: GameEvent::Message(message)
            }
        ],
        response: Some(format!("You say: {sentence}"))
    };
    Ok(CommandResult::Action(result))
}

fn handle_who(context: &SessionContext) -> Result<CommandResult, CommandExecutionError> {
    let online: Vec<(EntityId, Name)> = context.entities.query2::<Player, Name, _, _>(|iter| Ok(iter.map(|(e, (_, name))| (e.clone(), name.clone())).collect()))
        .map_err(|_| CommandExecutionError::Unrecoverable("Could not get online player list".into()))?;

    let mut strings: Vec<String> = Vec::new();
    for (e, name) in online {
        if e == context.player_id {
            continue;
        }

        strings.push(format!("    {name}"));
    }
    strings.sort();

    let response = if strings.is_empty() {
        "No other players online.".to_string()
    } else {
        let list = strings.join("\n");
        format!("Online:\n{list}")
    };

    Ok(CommandResult::Query(QueryResult { response }))
}

fn handle_look(context: &SessionContext) -> Result<CommandResult, CommandExecutionError> {
    let position = context.entities.get_component::<Location>(&context.player_id)
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get current position of entity {:?}", &context.player_id)))?
        .ok_or(CommandExecutionError::Unrecoverable(format!("Entity {:?} has no position component", &context.player_id)))?;
    let response = get_room_description(context, &RoomId::from_entity(position.value))?;
    Ok(CommandResult::Query(QueryResult { response }))
}

fn handle_inventory(context: &SessionContext) -> Result<CommandResult, CommandExecutionError> {
    let location = Location::new(context.player_id.clone());
    let items_in_inventory: Vec<String> = context.entities.query2_location::<Item, Name, _, _>(&location, |iter| {
        let names = iter.map(|(_id, (_, name))| format!("    {name}"));
        Ok(names.collect())
    })?;

    let response = if items_in_inventory.is_empty() {
        "You aren't carrying anything.".into()
    } else {
        format!("You are carrying:\n{}", items_in_inventory.join("\n"))
    };
    Ok(CommandResult::Query(response.into()))
}

async fn handle_take(context: &SessionContext, keywords: Keywords) -> Result<CommandResult, CommandExecutionError> {
    // Resolve the entity described by the user.
    let matching = resolve_entities_in_current_room(context, &keywords)?;
    let target = match matching.len() {
        0 => return Ok(CommandResult::Query(format!("There is no '{}' here.", keywords).into())),
        1 => matching.first().unwrap(),
        _ => return Ok(CommandResult::Query(format!("Which '{}'?", keywords).into()))
    };

    let item_name = get_entity_name(context, target)?;

    // Check that the resolved entity is actually an item.
    if context.entities.get_component::<Item>(target)?.is_none() {
        return Ok(CommandResult::Query(format!("You can't take '{item_name}'!").into()))
    }

    // Update position of target entity.
    let new_location = Location::new(context.player_id.clone());
    context.entities.update_component::<Location>(target, new_location.clone())?;

    let player_name = get_player_name(context)?;
    let player_location = get_current_position(context)?;
    let message = format!("{player_name} picked up '{item_name}'.");
    let action = ActionResult {
        events: vec![
            Event {
                target: EventTarget::LocationExcept(player_location, context.player_id.clone()),
                event: GameEvent::Message(message)
            }
        ],
        response: Some(format!("You took '{item_name}'"))
    };
    Ok(CommandResult::Action(action))
}

async fn handle_drop(context: &SessionContext, keywords: Keywords) -> Result<CommandResult, CommandExecutionError> {
    // Resolve the entity described by the user.
    let matching = resolve_entities_in_inventory(context, &keywords)?;
    let target = match matching.len() {
        0 => return Ok(CommandResult::Query(format!("There is no '{}' in your inventory.", keywords).into())),
        1 => matching.first().unwrap(),
        _ => return Ok(CommandResult::Query(format!("Which '{}'?", keywords).into()))
    };

    let item_name = get_entity_name(context, target)?;

    // Check that the resolved entity is actually an item.
    if context.entities.get_component::<Item>(target)?.is_none() {
        return Ok(CommandResult::Query(format!("You can't drop '{item_name}'!").into()))
    }

    // Update position of target entity.
    let new_location = get_current_position(context)?;
    context.entities.update_component::<Location>(target, new_location.clone())?;

    let player_name = get_player_name(context)?;
    let player_location = get_current_position(context)?;
    let message = format!("{player_name} dropped '{item_name}'.");
    let action = ActionResult {
        events: vec![
            Event {
                target: EventTarget::LocationExcept(player_location, context.player_id.clone()),
                event: GameEvent::Message(message)
            }
        ],
        response: Some(format!("You dropped '{item_name}'"))
    };
    Ok(CommandResult::Action(action))
}

fn handle_inspect(context: &SessionContext, keywords: Keywords) -> Result<CommandResult, CommandExecutionError> {
    // Resolve the entity described by the user.
    let matching = resolve_entities_in_context(context, &keywords)?;
    let target = match matching.len() {
        0 => return Ok(CommandResult::Query(format!("There is no '{}' in your inventory.", keywords).into())),
        1 => matching.first().unwrap(),
        _ => return Ok(CommandResult::Query(format!("Which '{}'?", keywords).into()))
    };

    let item_name = get_entity_name(context, target)?;
    let item_description = get_entity_description(context, target)?;
    let item_description = match item_description {
        Some(d) => format!("\n\n{d}"),
        None => "".into()
    };
    let response = format!("{item_name}{item_description}");
    Ok(CommandResult::Query(response.into()))
}

fn handle_edit(context: &SessionContext, target: EditTarget, field: EditField, content: String) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    let (entity_id, alias) = match target {
        EditTarget::Room => {
            let location = get_current_position(context)?;
            (location.value, context.entities.get_alias(&location.value)?)
        },
        EditTarget::Entity(alias) => {
            let alias = Alias::from(alias);
            let id = match context.entities.resolve_alias(&alias) {
                Some(e) => e,
                None => return Ok(CommandResult::Query(format!("Could not resolve alias '{alias}'").into()))
            };
            (id, alias)
        }
    };
    
    match field {
        EditField::Description => {
            let description = Description::from(content);
            context.entities.update_component(&entity_id, description)?;
            Ok(CommandResult::Query(format!("Updated description of '{alias}'").into()))
        }
        EditField::Name => {
            let name = Name::from(content);
            context.entities.update_component(&entity_id, name)?;
            Ok(CommandResult::Query(format!("Updated name of '{alias}'").into()))
        }
    }
}

fn handle_save(context: &SessionContext, target: SaveTarget, path: String) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    if path.contains("..") {
        return Ok(CommandResult::Query(format!("Invalid path: {path}").into()))
    }

    let response = match target {
        SaveTarget::Rooms => {
            let mut rooms = HashMap::new();
            for (id, room) in context.rooms.rooms().iter() {
                let alias = context.entities.get_alias(&id.as_entity())?;
                let name = match context.entities.get_component::<Name>(&id.as_entity())? {
                    Some(n) => n,
                    None => return Ok(CommandResult::Query(format!("Cannot serialize rooms - no name for '{}'", alias).into()))
                };
                let desc = match context.entities.get_component::<Description>(&id.as_entity())? {
                    Some(n) => n,
                    None => return Ok(CommandResult::Query(format!("Cannot serialize rooms - no description for '{}'", alias).into()))
                };

                let exits = room.exits().iter()
                    .filter_map(|d| room.get_destination(*d).map(|dest| (*d, *dest)))
                    .collect();

                let room_data = RoomData {
                    alias,
                    name: name.to_string(),
                    description: desc.to_string(),
                    exits
                };

                rooms.insert(*id, room_data);
            }
            match data::save_rooms(&format!("data/{path}"), &rooms) {
                Ok(_) => format!("Rooms saved to 'data/{path}'"),
                Err(e) => format!("Could not save rooms to 'data/{path}': {e:?}")
            }
        },
        SaveTarget::Items => {
            let items: HashMap<EntityId, (String, EntityId)> = context.entities.query3::<Item, Name, SpawnLocation, _, _>(|iter| {
                Ok(iter.map(|(e, (_, name, spawn))| (e.clone(), (name.to_string(), spawn.value))).collect())
            })?;
            let mut item_data = HashMap::new();
            for (e, (name, spawn)) in items {
                tracing::debug!("item: {0} ({1})", &e, &name);
                let alias = context.entities.get_alias(&e)?;
                let room_alias = match context.entities.get_alias(&spawn) {
                    Ok(r) => r,
                    Err(_) => return Ok(CommandResult::Query(format!("Invalid room ID: {spawn}").into()))
                };

                let description = match context.entities.get_component::<Description>(&e)? {
                    Some(d) => d.to_string(),
                    None => return Ok(CommandResult::Query(format!("Cannot serialize items - missing description for '{}'", alias).into()))
                };
                    
                item_data.insert(e, ItemData {
                    alias: alias.clone(),
                    name,
                    description,
                    spawn_location: room_alias
                });
            }

            match data::save_items(&format!("data/{path}"), &item_data) {
                Ok(_) => format!("Items saved to 'data/{path}'"),
                Err(e) => format!("Could not save items to 'data/{path}': {e:?}")
            }
        },
        SaveTarget::Npcs => {
            let npcs: HashMap<EntityId, (String, EntityId)> = context.entities.query3::<Npc, Name, SpawnLocation, _, _>(|iter| {
                Ok(iter.map(|(e, (_, name, spawn))| (e.clone(), (name.to_string(), spawn.value))).collect())
            })?;
            let mut npc_data = HashMap::new();
            for (e, (name, spawn)) in npcs {
                tracing::debug!("npc: {0} ({1})", &e, &name);
                let alias = context.entities.get_alias(&e)?;
                let room_alias = match context.entities.get_alias(&spawn) {
                    Ok(r) => r,
                    Err(_) => return Ok(CommandResult::Query(format!("Invalid room ID: {spawn}").into()))
                };

                let description = match context.entities.get_component::<Description>(&e)? {
                    Some(d) => d.to_string(),
                    None => return Ok(CommandResult::Query(format!("Cannot serialize NPCs - missing description for '{}'", alias).into()))
                };
                    
                npc_data.insert(e, NpcData {
                    alias: alias.clone(),
                    name,
                    description,
                    spawn_location: room_alias
                });
            }

            match data::save_npcs(&format!("data/{path}"), &npc_data) {
                Ok(_) => format!("NPCs saved to 'data/{path}'"),
                Err(e) => format!("Could not save NPCs to 'data/{path}': {e:?}")
            }
        }
    };

    Ok(CommandResult::Query(QueryResult { response }))
}

fn handle_link(context: &SessionContext, direction: Direction, target: Alias) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    let location = get_current_position(context)?;
    let current_room_id = RoomId::from_entity(location.value);
    let current_room = match context.rooms.get_room(&current_room_id) {
        Some(r) => r,
        None => return Ok(CommandResult::Query(format!("Could not get current room (ID: {0})", current_room_id).into()))
    };

    let other_room_id = match context.entities.resolve_alias(&target) {
        Some(r) => r,
        None => return Ok(CommandResult::Query(format!("Unknown room alias: '{target}'").into()))
    };
    let other_room_id = RoomId::from_entity(other_room_id);
    let other_room = match context.rooms.get_room(&other_room_id) {
        Some(r) => r,
        None => return Ok(CommandResult::Query(format!("Could not get room with alias: '{target}' (ID: {other_room_id})").into()))
    };

    let opposite_direction = direction.opposite();

    if current_room.has_exit(&direction) {
        return Ok(CommandResult::Query(format!("Current room already has an exit to the {direction}").into()))
    };
    if other_room.has_exit(&opposite_direction) {
        return Ok(CommandResult::Query(format!("Destination room already has an exit to the {opposite_direction}").into()))
    };

    let current_room_alias = context.entities.get_alias(&current_room_id.as_entity())?;
    let other_room_alias = context.entities.get_alias(&other_room_id.as_entity())?;

    let mut current_room = RoomGraphNode::clone(&current_room);
    let mut other_room = RoomGraphNode::clone(&other_room);

    current_room.set_exit(direction, other_room_id.clone());
    other_room.set_exit(opposite_direction, current_room_id);

    let response = format!("Linked '{0}' to '{1}'", current_room_alias, other_room_alias);

    context.rooms.update_room(current_room_id, current_room);
    context.rooms.update_room(other_room_id, other_room);

    Ok(CommandResult::Query(response.into()))
}

fn handle_unlink(context: &SessionContext, direction: Direction) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    let location = get_current_position(context)?;
    let current_room_id = RoomId::from_entity(location.value);
    let current_room = match context.rooms.get_room(&current_room_id) {
        Some(r) => r,
        None => return Ok(CommandResult::Query(format!("Could not get current room (ID: {0})", current_room_id).into()))
    };

    if !current_room.has_exit(&direction) {
        return Ok(CommandResult::Query(format!("Current room has no {direction} exit.").into()))
    }

    let other_room_id = match current_room.get_destination(direction) {
        Some(id) => id,
        None => return Err(CommandExecutionError::Unrecoverable(format!("Could not get destination for {direction} exit of current room.")))
    };
    let other_room = match context.rooms.get_room(&other_room_id) {
        Some(r) => r,
        None => return Err(CommandExecutionError::Unrecoverable(format!("Could not get destination room with ID: {other_room_id}")))
    };

    let current_room_alias = context.entities.get_alias(&current_room_id.as_entity())?;
    let other_room_alias = context.entities.get_alias(&other_room_id.as_entity())?;

    let mut current_room = RoomGraphNode::clone(&current_room);
    let mut other_room = RoomGraphNode::clone(&other_room);

    current_room.remove_exit(&direction);
    other_room.remove_exit(&direction.opposite());

    let response = format!("Removed link between '{0}' and '{1}'", current_room_alias, other_room_alias);

    context.rooms.update_room(current_room_id, current_room);
    context.rooms.update_room(other_room_id.clone(), other_room);

    Ok(CommandResult::Query(response.into()))
}

fn handle_create(context: &SessionContext, direction: Direction, target: Alias) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    let location = get_current_position(context)?;
    let current_room_id = RoomId::from_entity(location.value);
    let current_room = match context.rooms.get_room(&current_room_id) {
        Some(r) => r,
        None => return Ok(CommandResult::Query(format!("Could not get current room (ID: {0})", current_room_id).into()))
    };

    if current_room.has_exit(&direction) {
        return Ok(CommandResult::Query(format!("Current room already has an exit to the {direction}").into()))
    }

    let mut current_room = RoomGraphNode::clone(&current_room);
    let other_room_id = context.entities.spawn(None, target.clone())?;
    let other_room_id = RoomId::from_entity(other_room_id);
    let mut other_room = RoomGraphNode::new(HashMap::new());

    current_room.set_exit(direction, other_room_id);
    other_room.set_exit(direction.opposite(), current_room_id.clone());

    let response = format!("Created room '{0}' to the {1}.", target, direction);

    context.rooms.update_room(current_room_id, current_room);
    context.rooms.update_room(other_room_id, other_room);

    Ok(CommandResult::Query(QueryResult { response }))
}

async fn handle_spawn(context: &SessionContext, target: SpawnTarget, alias: Alias) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }
    
    let location = get_current_position(context)?;
    let current_room_id = RoomId::from_entity(location.value);

    let entity_id = match context.entities.spawn(None, alias.clone()) {
        Ok(id) => id,
        Err(EntityRegistryError::DuplicateAlias(a)) => return Ok(CommandResult::Query(format!("An entity already exists with alias '{a}'").into())),
        Err(e) => return Err(CommandExecutionError::from(e))
    };

    let location = Location { value: current_room_id.as_entity() };
    context.entities.update_component(&entity_id, SpawnLocation::from(&location))?;
    context.entities.update_component(&entity_id, location)?;

    let name = Name::from("Unnamed");
    context.entities.update_component(&entity_id, name)?;

    // Generate marker component depending on spawn target.
    match target {
        SpawnTarget::Item => {
            context.entities.update_component(&entity_id, Item)?;
            Ok(CommandResult::Query(format!("Spawned item '{alias}'").into()))
        },
        SpawnTarget::Npc => {
            context.entities.update_component(&entity_id, Npc)?;
            Ok(CommandResult::Query(format!("Spawned npc '{alias}'").into()))
        }
    }

}

fn handle_roominfo(context: &SessionContext) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    let location = get_current_position(context)?;
    let current_room_id = RoomId::from_entity(location.value);
    let current_room = match context.rooms.get_room(&current_room_id) {
        Some(r) => r,
        None => return Ok(CommandResult::Query(format!("Could not get current room (ID: {0})", current_room_id).into()))
    };
    let current_room_alias = context.entities.get_alias(&current_room_id.as_entity())?;

    let mut exits: Vec<String> = Vec::new();
    for exit in current_room.exits() {
        let destination_id = match current_room.get_destination(exit) {
            Some(id) => id,
            None => return Err(CommandExecutionError::Unrecoverable(format!("Could not get destination for exit '{exit}' of current room.")))
        };
        let destination_alias = context.entities.get_alias(&destination_id.as_entity())?;

        exits.push(format!("{0}: {1} ({2})", exit, destination_alias, destination_id));
    }

    let response = format!("{0} ({1})\n\n{2}", current_room_alias, current_room_id, exits.join("\n"));
    Ok(CommandResult::Query(response.into()))
}

pub async fn handle_command(context: &mut SessionContext, command: Command) -> Result<CommandResult, CommandExecutionError> {
    match command {
        Command::Go(direction) => handle_go(context, direction).await,
        Command::Say(sentence) => handle_say(context, &sentence),
        Command::Who => handle_who(context),
        Command::Look => handle_look(context),
        Command::Inventory => handle_inventory(context),
        Command::Take(target) => handle_take(context, target).await,
        Command::Drop(target) => handle_drop(context, target).await,
        Command::Inspect(target) => handle_inspect(context, target),

        Command::Edit(target, field, content) => handle_edit(context, target, field, content),
        Command::Save(target, path) => handle_save(context, target, path),
        Command::Link(direction, target) => handle_link(context, direction, target),
        Command::Unlink(direction) => handle_unlink(context, direction),
        Command::Create(direction, target) => handle_create(context, direction, target),
        Command::Spawn(target, alias) => handle_spawn(context, target, alias).await,
        Command::RoomInfo => handle_roominfo(context),

        Command::Quit => {
            let name = context.entities.get_component::<Name>(&context.player_id)
                .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get name for entity: {:?}", context.player_id)))?
                .ok_or(CommandExecutionError::Unrecoverable(format!("Entity {:?} had no Name component", context.player_id)))?;
            tracing::info!("Player '{:?}' quit", context.player_id);
            let quit_event = Event {
                event: GameEvent::SessionEnded,
                target: EventTarget::Entity(context.player_id.clone())
            };
            let result = ActionResult {
                events: vec![quit_event],
                response: Some(format!("Goodbye {name}!"))
            };
            Ok(CommandResult::Action(result))
        }
    }

}

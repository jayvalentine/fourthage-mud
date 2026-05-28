use std::collections::HashMap;

use crate::entities::{EntityRegistryError, Name, Player, Position};
use crate::event::{Event, EventTarget, GameEvent};
use crate::model::world::{DirectionParseError, Room};
use crate::model::{world::Direction, ids::{EntityId, RoomId}};
use crate::session::SessionContext;
use crate::{data, persistence};

pub enum Command {
    Go(Direction),
    Say(String),
    Who,
    Look,

    // Admin commands
    Edit(EditField, String),
    Save(SaveTarget, String),
    Link(Direction, String),
    Create(Direction, String),
    RoomInfo,

    // Session management commands
    Quit
}

pub enum EditField {
    Description,
    Name
}

pub enum SaveTarget {
    World
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
        match value {
            _ => CommandExecutionError::Unrecoverable("Entity registry error".into())
        }
    }
}

impl Command {
    fn parse_direction(s: &str) -> Option<Direction> {
        match s {
            "north" => Some(Direction::North),
            "south" => Some(Direction::South),
            "east" => Some(Direction::East),
            "west" => Some(Direction::West),
            _ => None
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
                let direction = match Self::parse_direction(direction) {
                    Some(d) => d,
                    None => return Err(CommandParseError::InvalidSyntax(format!("You can't go {direction}!")))
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

            "edit" => {
                let field = match parts.next() {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("Edit what?".into()))
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
                Ok(Command::Edit(field, content))
            },
            "save" => {
                let target = match parts.next() {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("Save what?".into()))
                };
                let target = match target {
                    "world" => SaveTarget::World,
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
                Ok(Command::Link(direction, alias.to_string()))
            },
            "create" => {
                let direction = match parts.next() {
                    Some(s) => Direction::from_string(s)?,
                    None => return Err(CommandParseError::InvalidSyntax("Create in which direction?".into()))
                };
                let alias = match parts.next() {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("Create what?".into()))
                };
                Ok(Command::Create(direction, alias.to_string()))
            },
            "roominfo" => Ok(Command::RoomInfo),

            "quit" => Ok(Command::Quit),
            _ => Err(CommandParseError::UnknownCommand(input.to_string())),
        }
    }
}

fn get_current_position(context: &SessionContext) -> Result<Position, CommandExecutionError> {
    context.entities.get_component::<Position>(&context.player_id)
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get current position of entity {:?}", &context.player_id)))?
        .ok_or(CommandExecutionError::Unrecoverable(format!("Entity {:?} has no position component", &context.player_id)))
}

fn get_room_description(context: &SessionContext, id: &RoomId) -> Result<String, CommandExecutionError> {
    let current_room = context.world.get_room(id)
        .ok_or(CommandExecutionError::Unrecoverable("Could not retrieve room based on current room ID".into()))?;

    let room_name = current_room.name();
    let room_desc = current_room.description();
    let exits: Vec<String> = current_room.exits().into_iter().map(|e| e.to_string()).collect();
    let exits = exits.join(", ");

    let response = format!("{room_name}\n\n{room_desc}\n\nFrom here you can go: {exits}\n");
    Ok(response)
}

async fn handle_go(context: &mut SessionContext, direction: Direction) -> Result<CommandResult, CommandExecutionError> {
    let position = get_current_position(context)?;
    let current_room = context.world.get_room(&position.room)
        .ok_or(CommandExecutionError::Unrecoverable("Could not retrieve room based on current room ID".into()))?;

    let destination_room_id = match current_room.get_destination(direction) {
        Some(id) => id,
        None => return Ok(CommandResult::Query(QueryResult { response: format!("You cannot go {direction} from here.") }))
    };

    let new_position = Position { room: destination_room_id.clone() };
    context.entities.update_component(&context.player_id, new_position.clone())
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not update position of entity '{:?}'", &context.player_id)))?;
    persistence::persist_position(&context.player_id, &new_position, &context.pool)
        .await.map_err(|_| CommandExecutionError::Unrecoverable("Failed to update room ID in database".into()))?;

    let description = get_room_description(context, destination_room_id)?;

    let response = format!("You go {direction}.\n\n{description}");
    let result = ActionResult { events: Vec::new(), response: Some(response) };
    Ok(CommandResult::Action(result))
}

fn handle_say(context: &SessionContext, sentence: &str) -> Result<CommandResult, CommandExecutionError> {
    let name = context.entities.get_component::<Name>(&context.player_id)
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get name for entity: {:?}", context.player_id)))?
        .ok_or(CommandExecutionError::Unrecoverable(format!("Entity {:?} had no Name component", context.player_id)))?;
    let position = context.entities.get_component::<Position>(&context.player_id)
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get current position of entity {:?}", &context.player_id)))?
        .ok_or(CommandExecutionError::Unrecoverable(format!("Entity {:?} has no position component", &context.player_id)))?;
    let message = format!("{name} says: {sentence}");
    let result = ActionResult {
        events: vec![
            Event {
                target: EventTarget::RoomExcept(position.room, context.player_id.clone()),
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
    let position = context.entities.get_component::<Position>(&context.player_id)
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get current position of entity {:?}", &context.player_id)))?
        .ok_or(CommandExecutionError::Unrecoverable(format!("Entity {:?} has no position component", &context.player_id)))?;
    let response = get_room_description(context, &position.room)?;
    Ok(CommandResult::Query(QueryResult { response }))
}

fn handle_edit(context: &SessionContext, field: EditField, content: String) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    let position = get_current_position(context)?;
    let response = if let Some(room) = context.world.get_room(&position.room) {
        let mut updated = Room::clone(&room);
        match field {
            EditField::Description => { updated.set_description(content); },
            EditField::Name => { updated.set_name(content); }
        }
        context.world.update_room(position.room.clone(), updated);
        CommandResult::Query(QueryResult { response: "Updated room.".into() })
    } else {
        CommandResult::Query(QueryResult { response: format!("Cannot update room '{0:?}'", position.room) })
    };
    Ok(response)
}

fn handle_save(context: &SessionContext, target: SaveTarget, path: String) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    let response = match target {
        SaveTarget::World => {
            let rooms = context.world.rooms();
            match data::save_rooms(&format!("data/{path}"), &rooms) {
                Ok(_) => format!("World saved to '{path}'"),
                Err(e) => format!("Could not save world to '{path}': {e:?}")
            }
        }
    };

    Ok(CommandResult::Query(QueryResult { response }))
}

fn handle_link(context: &SessionContext, direction: Direction, target: String) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    let position = get_current_position(context)?;
    let current_room = match context.world.get_room(&position.room) {
        Some(r) => r,
        None => return Ok(CommandResult::Query(format!("Could not get current room (ID: {0})", position.room).into()))
    };

    let other_room_id = match context.world.resolve_alias(&target) {
        Some(r) => r,
        None => return Ok(CommandResult::Query(format!("Unknown room alias: '{target}'").into()))
    };
    let other_room = match context.world.get_room(&other_room_id) {
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

    let mut current_room = Room::clone(&current_room);
    let mut other_room = Room::clone(&other_room);

    current_room.set_exit(direction, other_room_id.clone());
    other_room.set_exit(opposite_direction, position.room.clone());

    let response = format!("Linked '{0}' to '{1}'", current_room.alias(), other_room.alias());

    context.world.update_room(position.room, current_room);
    context.world.update_room(other_room_id, other_room);

    Ok(CommandResult::Query(response.into()))
}

fn handle_create(context: &SessionContext, direction: Direction, target: String) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    let position = get_current_position(context)?;
    let current_room = match context.world.get_room(&position.room) {
        Some(r) => r,
        None => return Ok(CommandResult::Query(format!("Could not get current room (ID: {0})", position.room).into()))
    };

    if current_room.has_exit(&direction) {
        return Ok(CommandResult::Query(format!("Current room already has an exit to the {direction}").into()))
    }

    let mut current_room = Room::clone(&current_room);
    let other_room_id = RoomId::generate();
    let mut other_room = Room::new(target, "Unnamed Room".into(), "This room has no description.".into(), HashMap::new());

    current_room.set_exit(direction, other_room_id.clone());
    other_room.set_exit(direction.opposite(), position.room.clone());

    let response = format!("Created room '{0}' to the {1}.", other_room.alias(), direction);

    context.world.update_room(position.room, current_room);
    context.world.update_room(other_room_id, other_room);

    Ok(CommandResult::Query(QueryResult { response }))
}

fn handle_roominfo(context: &SessionContext) -> Result<CommandResult, CommandExecutionError> {
    if !context.is_admin {
        return Ok(CommandResult::Unauthorized)
    }

    let position = get_current_position(context)?;
    let current_room = match context.world.get_room(&position.room) {
        Some(r) => r,
        None => return Ok(CommandResult::Query(format!("Could not get current room (ID: {0})", position.room).into()))
    };

    let mut exits: Vec<String> = Vec::new();
    for exit in current_room.exits() {
        let destination_id = match current_room.get_destination(exit) {
            Some(id) => id,
            None => return Err(CommandExecutionError::Unrecoverable(format!("Could not get destination for exit '{exit}' of current room.")))
        };
        let destination = match context.world.get_room(&destination_id) {
            Some(r) => r,
            None => return Err(CommandExecutionError::Unrecoverable(format!("Could not get destination room from ID '{destination_id}'.")))
        };
        exits.push(format!("{0}: {1} ({2})", exit, destination.alias(), destination_id));
    }

    let response = format!("{0} ({1})\n\n{2}", current_room.alias(), position.room, exits.join("\n"));
    Ok(CommandResult::Query(response.into()))
}

pub async fn handle_command(context: &mut SessionContext, command: Command) -> Result<CommandResult, CommandExecutionError> {
    match command {
        Command::Go(direction) => handle_go(context, direction).await,
        Command::Say(sentence) => handle_say(context, &sentence),
        Command::Who => handle_who(context),
        Command::Look => handle_look(context),

        Command::Edit(field, content) => handle_edit(context, field, content),
        Command::Save(target, path) => handle_save(context, target, path),
        Command::Link(direction, target) => handle_link(context, direction, target),
        Command::Create(direction, target) => handle_create(context, direction, target),
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

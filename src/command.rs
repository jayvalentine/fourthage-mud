use crate::{db, event::{Event, EventTarget, GameEvent}, model::world::{Direction, RoomId}, session::SessionContext};

pub enum Command {
    Go(Direction),
    Say(String),
    Look,
    Quit
}

pub enum CommandParseError {
    UnknownCommand(String),
    InvalidSyntax(String)
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

pub enum CommandResult {
    Action(ActionResult),
    Query(QueryResult)
}

pub enum CommandExecutionError {
    /// Command could not be executed due to an unrecoverable error.
    Unrecoverable(String)
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
        let mut parts = input.split(' ');
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
            }
            "look" => Ok(Command::Look),
            "quit" => Ok(Command::Quit),
            _ => Err(CommandParseError::UnknownCommand(input.to_string())),
        }
    }
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
    let position = context.entities.get_position(&context.player_name)
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get current position of entity {0}", &context.player_name)))?;
    let current_room = context.world.get_room(&position.room)
        .ok_or(CommandExecutionError::Unrecoverable("Could not retrieve room based on current room ID".into()))?;

    let destination_room_id = match current_room.get_destination(direction) {
        Some(id) => id,
        None => return Ok(CommandResult::Query(QueryResult { response: format!("You cannot go {direction} from here.") }))
    };

    context.entities.update_position(&context.player_name, destination_room_id.clone())
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not update position of entity '{0}'", &context.player_name)))?;
    db::update_room_id(&context.pool, &context.player_name, destination_room_id.value())
        .await.map_err(|_| CommandExecutionError::Unrecoverable("Failed to update room ID in database".into()))?;

    let description = get_room_description(context, destination_room_id)?;

    let response = format!("You go {direction}.\n\n{description}");
    let result = ActionResult { events: Vec::new(), response: Some(response) };
    Ok(CommandResult::Action(result))
}

fn handle_say(context: &SessionContext, sentence: &str) -> Result<CommandResult, CommandExecutionError> {
    let name = &context.player_name;
    let position = context.entities.get_position(&context.player_name)
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get current position of entity {0}", &context.player_name)))?;
    let message = format!("{name} says: {sentence}");
    let result = ActionResult {
        events: vec![
            Event {
                target: EventTarget::RoomExcept(position.room, name.clone()),
                event: GameEvent::Message(message)
            }
        ],
        response: None
    };
    Ok(CommandResult::Action(result))
}

fn handle_look(context: &SessionContext) -> Result<CommandResult, CommandExecutionError> {
    let position = context.entities.get_position(&context.player_name)
        .map_err(|_| CommandExecutionError::Unrecoverable(format!("Could not get current position of entity {0}", &context.player_name)))?;
    let response = get_room_description(context, &position.room)?;
    Ok(CommandResult::Query(QueryResult { response }))
}

pub async fn handle_command(context: &mut SessionContext, command: Command) -> Result<CommandResult, CommandExecutionError> {
    match command {
        Command::Go(direction) => handle_go(context, direction).await,
        Command::Say(sentence) => handle_say(context, &sentence),
        Command::Look => handle_look(context),
        Command::Quit => {
            let name = &context.player_name;
            tracing::info!("Player '{name}' quit");
            let quit_event = Event {
                event: GameEvent::SessionEnded,
                target: EventTarget::Player(name.into())
            };
            let result = ActionResult {
                events: vec![quit_event],
                response: Some(format!("Goodbye {name}!"))
            };
            Ok(CommandResult::Action(result))
        }
    }

}

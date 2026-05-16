use crate::{db, model::{player::Player, world::Direction}, session::SessionContext};

pub enum Command {
    Go(Direction),
    Look,
    Quit
}

pub enum CommandParseError {
    UnknownCommand(String),
    InvalidSyntax(String)
}

pub enum CommandExecutionError {
    /// Command was invalid; string provides player-readable reason why.
    InvalidCommand(String),

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
        let mut parts = input.splitn(2, ' ');
        let verb = parts.next().unwrap_or("").to_lowercase();
        let arg = parts.next();

        match verb.as_str() {
            "go" => {
                let direction = match arg {
                    Some(s) => s,
                    None => return Err(CommandParseError::InvalidSyntax("Go where?".into()))
                };
                let direction = match Self::parse_direction(direction) {
                    Some(d) => d,
                    None => return Err(CommandParseError::InvalidSyntax(format!("You can't go {direction}!")))
                };
                Ok(Command::Go(direction))
            },
            "look" => Ok(Command::Look),
            "quit" => Ok(Command::Quit),
            _ => Err(CommandParseError::UnknownCommand(input.to_string())),
        }
    }
}

/// Handle 'go <direction>'
pub async fn handle_go(context: &mut SessionContext, player: &mut Player, direction: Direction) -> Result<String, CommandExecutionError> {
    let current_room = context.world.get_room(player.current_room())
        .ok_or(CommandExecutionError::Unrecoverable("Could not retrieve room based on current room ID".into()))?;

    let destination_room_id = current_room.get_destination(direction)
        .ok_or(CommandExecutionError::InvalidCommand(format!("You cannot go {direction} from here.")))?;

    player.move_to(destination_room_id);
    db::update_room_id(&context.pool, player.name(), destination_room_id.value())
        .await.map_err(|_| CommandExecutionError::Unrecoverable("Failed to update room ID in database".into()))?;

    let description = handle_look(context, player)?;

    Ok(format!("You go {direction}.\n\n{description}"))
}

pub fn handle_look(context: &SessionContext, player: &Player) -> Result<String, CommandExecutionError> {
    let current_room = context.world.get_room(player.current_room())
        .ok_or(CommandExecutionError::Unrecoverable("Could not retrieve room based on current room ID".into()))?;

    let room_name = current_room.name();
    let room_desc = current_room.description();
    let exits: Vec<String> = current_room.exits().into_iter().map(|e| e.to_string()).collect();
    let exits = exits.join(", ");

    Ok(format!("{room_name}\n\n{room_desc}\n\nFrom here you can go: {exits}\n"))
}

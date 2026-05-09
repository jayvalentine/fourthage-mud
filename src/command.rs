use crate::model::{player::Player, world::{Direction, World}};

pub enum Command {
    Go(Direction)
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
                    None => return Err(CommandParseError::UnknownCommand(input.to_string()))
                };
                Ok(Command::Go(direction))
            }
            _ => Err(CommandParseError::UnknownCommand(input.to_string())),
        }
    }
}

/// Handle 'go <direction>'
pub fn handle_go(world: &World, player: &mut Player, direction: Direction) -> Result<(), CommandExecutionError> {
    let current_room = world.get_room(player.current_room())
        .ok_or(CommandExecutionError::Unrecoverable("Could not retrieve room based on current room ID".into()))?;

    let destination_room_id = current_room.get_destination(direction)
        .ok_or(CommandExecutionError::InvalidCommand(format!("You cannot go {direction} from here.")))?;

    player.move_to(destination_room_id);
    Ok(())
}


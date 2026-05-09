use crate::model::world::Direction;

pub enum Command {
    Go(Direction)
}

pub enum CommandParseError {
    UnknownCommand(String),
    InvalidSyntax(String)
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

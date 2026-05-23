use std::{collections::HashMap, fmt};
use serde::Deserialize;
use serde::de::Error;

use super::ids::RoomId;

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum Direction {
    North,
    South,
    East,
    West
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Direction::North => write!(f, "north"),
            Direction::South => write!(f, "south"),
            Direction::East => write!(f, "east"),
            Direction::West => write!(f, "west")
        }
    }
}

impl<'de> Deserialize<'de> for Direction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "north" => Ok(Direction::North),
            "south" => Ok(Direction::South),
            "east" => Ok(Direction::East),
            "west" => Ok(Direction::West),
            invalid => Err(D::Error::custom(format!("Invalid direction: {invalid}")))
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct Room {
    id: String,
    name: String,
    description: String,
    exits: HashMap<Direction, RoomId>
}

impl Room {
    pub fn get_destination(&self, direction: Direction) -> Option<&RoomId> {
        self.exits.get(&direction)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn exits(&self) -> Vec<Direction> {
        self.exits.keys().copied().collect()
    }
}

pub struct World {
    rooms: HashMap<RoomId, Room>
}

impl World {
    pub fn new(rooms: HashMap<RoomId, Room>) -> World {
        World { rooms }
    }

    pub fn get_room(&self, id: &RoomId) -> Option<&Room> {
        self.rooms.get(id)
    }
}

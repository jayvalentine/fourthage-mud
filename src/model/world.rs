use std::{collections::HashMap, fmt};
use serde::Deserialize;

#[derive(Clone, Hash, PartialEq, Eq, Deserialize, Debug)]
pub struct RoomId(u32);

impl RoomId {
    pub fn new(id: u32) -> RoomId {
        RoomId(id)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct Room {
    id: RoomId,
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
    pub fn new(rooms: Vec<Room>) -> World {
        let rooms: HashMap<RoomId, Room> = rooms.into_iter().map(|room| (room.id.clone(), room)).collect();

        World { rooms }
    }

    pub fn get_room(&self, id: &RoomId) -> Option<&Room> {
        self.rooms.get(id)
    }
}

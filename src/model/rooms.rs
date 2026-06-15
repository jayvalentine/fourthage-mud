use std::sync::Arc;
use parking_lot::lock_api::MappedRwLockReadGuard;
use parking_lot::{RawRwLock, RwLock, RwLockReadGuard};
use std::{collections::HashMap, fmt};

use super::ids::RoomId;

pub enum DirectionParseError {
    Invalid(String)
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum Direction {
    North,
    South,
    East,
    West
}

impl Direction {
    pub fn from_string(s: &str) -> Result<Direction, DirectionParseError> {
        match s.to_ascii_lowercase().as_str() {
            "n" | "north" => Ok(Direction::North),
            "s" | "south" => Ok(Direction::South),
            "e" | "east" => Ok(Direction::East),
            "w" | "west" => Ok(Direction::West),
            s => Err(DirectionParseError::Invalid(s.to_string()))
        }
    }

    pub fn opposite(&self) -> Direction {
        match self {
            Direction::North => Direction::South,
            Direction::South => Direction::North,
            Direction::East => Direction::West,
            Direction::West => Direction::East
        }
    }
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

#[derive(Clone, Debug)]
pub struct RoomGraphNode {
    exits: HashMap<Direction, RoomId>,
}

impl RoomGraphNode {
    pub fn new(exits: HashMap<Direction, RoomId>) -> RoomGraphNode {
        RoomGraphNode {
            exits
        }
    }

    pub fn get_destination(&self, direction: Direction) -> Option<&RoomId> {
        self.exits.get(&direction)
    }

    pub fn set_exit(&mut self, direction: Direction, destination: RoomId) {
        self.exits.insert(direction, destination);
    }

    pub fn remove_exit(&mut self, direction: &Direction) {
        self.exits.remove(direction);
    }

    pub fn exits(&self) -> Vec<Direction> {
        self.exits.keys().copied().collect()
    }

    pub fn has_exit(&self, direction: &Direction) -> bool {
        self.exits.contains_key(direction)
    }
}

struct RoomGraphInner {
    rooms: HashMap<RoomId, Arc<RoomGraphNode>>,
}

pub struct RoomGraph {
    inner: RwLock<RoomGraphInner>,
    starting_room: RoomId
}

impl RoomGraph {
    pub fn new(starting_room: RoomId) -> RoomGraph {
        RoomGraph {
            inner: RwLock::new(RoomGraphInner {
                rooms: HashMap::new()
            }),
            starting_room
        }
    }

    pub fn get_room(&self, id: &RoomId) -> Option<Arc<RoomGraphNode>> {
        let read = self.inner.read();
        let room = read.rooms.get(id);
        room.map(|r| r.clone())
    }

    pub fn update_room(&self, id: RoomId, room: RoomGraphNode) {
        let mut write = self.inner.write();
        write.rooms.insert(id, Arc::new(room));
    }

    pub fn rooms(&self) -> MappedRwLockReadGuard<RawRwLock, HashMap<RoomId, Arc<RoomGraphNode>>> {
        RwLockReadGuard::map(self.inner.read(), |inner| &inner.rooms)
    }

    pub fn default_room_id(&self) -> RoomId {
        self.starting_room.clone()
    }
}

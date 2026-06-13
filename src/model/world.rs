use std::sync::Arc;
use parking_lot::lock_api::MappedRwLockReadGuard;
use parking_lot::{RawRwLock, RwLock, RwLockReadGuard};
use std::{collections::HashMap, fmt};
use serde::{Deserialize, Serialize};
use serde::de::Error;
use uuid::uuid;

use crate::model::ids::Alias;

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

impl<'de> Deserialize<'de> for Direction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Direction::from_string(&s).map_err(|_| D::Error::custom(format!("Invalid direction: {s}")))
    }
}

impl Serialize for Direction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let s = format!("{}", self);
        String::serialize(&s, serializer)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Room {
    alias: Alias,
    name: String,
    description: String,
    exits: HashMap<Direction, RoomId>,
}

impl Room {
    pub fn new(alias: Alias, name: String, description: String, exits: HashMap<Direction, RoomId>) -> Room {
        Room {
            alias,
            name,
            description,
            exits
        }
    }

    pub fn get_destination(&self, direction: Direction) -> Option<&RoomId> {
        self.exits.get(&direction)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn alias(&self) -> &Alias {
        &self.alias
    }

    pub fn set_description(&mut self, desc: String) {
        self.description = desc;
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
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

struct WorldInner {
    rooms: HashMap<RoomId, Arc<Room>>,
    aliases: HashMap<Alias, RoomId>
}

pub struct World {
    inner: RwLock<WorldInner>,
    starting_room: RoomId
}

impl World {
    pub fn new(rooms: HashMap<RoomId, Room>, starting_room: RoomId) -> World {
        let mut aliases = HashMap::new();
        for (id, room) in rooms.iter() {
            aliases.insert(room.alias.clone(), id.clone());
        }

        let rooms = rooms.into_iter().map(|(id, room)| (id, Arc::new(room))).collect();

        World {
            inner: RwLock::new(WorldInner {
                rooms,
                aliases
            }),
            starting_room
        }
    }

    pub fn get_room(&self, id: &RoomId) -> Option<Arc<Room>> {
        let read = self.inner.read();
        let room = read.rooms.get(id);
        room.map(|r| r.clone())
    }

    pub fn update_room(&self, id: RoomId, room: Room) {
        let mut write = self.inner.write();
        let new_alias = room.alias.clone();
        if let Some(old_room) = write.rooms.insert(id, Arc::new(room)) {
            write.aliases.remove(&old_room.alias);
        }
        write.aliases.insert(new_alias, id.clone());
    }

    pub fn resolve_alias(&self, alias: &Alias) -> Option<RoomId> {
        let read = self.inner.read();
        read.aliases.get(alias).cloned()
    }

    pub fn rooms(&self) -> MappedRwLockReadGuard<RawRwLock, HashMap<RoomId, Arc<Room>>> {
        RwLockReadGuard::map(self.inner.read(), |inner| &inner.rooms)
    }

    pub fn default_room_id(&self) -> RoomId {
        self.starting_room.clone()
    }
}

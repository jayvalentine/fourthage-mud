use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard};
use std::{collections::HashMap, fmt};
use serde::{Deserialize, Serialize};
use serde::de::Error;
use uuid::uuid;

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

impl Serialize for Direction {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        let s = match self {
            Direction::North => "north",
            Direction::South => "south",
            Direction::East => "east",
            Direction::West => "west"
        };
        let s = s.to_string();
        String::serialize(&s, serializer)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Room {
    alias: String,
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

    pub fn set_description(&mut self, desc: String) {
        self.description = desc;
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn exits(&self) -> Vec<Direction> {
        self.exits.keys().copied().collect()
    }
}

pub enum WorldError {
    InvalidMutex
}

impl<T> From<PoisonError<T>> for WorldError {
    fn from(_: PoisonError<T>) -> Self {
        WorldError::InvalidMutex
    }
}

struct WorldInner {
    rooms: HashMap<RoomId, Arc<Room>>,
    aliases: HashMap<String, RoomId>
}

pub struct World {
    inner: RwLock<WorldInner>
}

impl World {
    pub fn new(rooms: HashMap<RoomId, Room>) -> World {
        let mut aliases = HashMap::new();
        for (id, room) in rooms.iter() {
            aliases.insert(room.alias.clone(), id.clone());
        }

        let rooms = rooms.into_iter().map(|(id, room)| (id, Arc::new(room))).collect();

        World {
            inner: RwLock::new(WorldInner {
                rooms,
                aliases
            })
        }
    }

    pub fn get_room(&self, id: &RoomId) -> Result<Option<Arc<Room>>, WorldError> {
        let read = self.inner.read()?;
        let room = read.rooms.get(id);
        Ok(room.map(|r| r.clone()))
    }

    pub fn update_room(&self, id: RoomId, room: Room) -> Result<(), WorldError> {
        let mut write = self.inner.write()?;
        let new_alias = room.alias.clone();
        if let Some(old_room) = write.rooms.insert(id, Arc::new(room)) {
            write.aliases.remove(&old_room.alias);
        }
        write.aliases.insert(new_alias, id.clone());
        Ok(())
    }

    pub fn rooms(&self) -> Result<HashMap<RoomId, Arc<Room>>, WorldError> {
        let rooms = self.inner.read()?.rooms.clone();
        Ok(rooms)
    }

    pub fn default_room_id() -> RoomId {
        RoomId::from_uuid(uuid!("019e5690-0757-7256-97c1-a403f4d347ca"))
    }
}

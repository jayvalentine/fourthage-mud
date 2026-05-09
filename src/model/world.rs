use std::{collections::HashMap, fmt};

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct RoomId(u32);

impl RoomId {
    pub fn new(id: u32) -> RoomId {
        RoomId(id)
    }
}

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
        self.exits.keys().map(|d| *d).collect()
    }
}

pub struct World {
    rooms: HashMap<RoomId, Room>
}

impl World {
    pub fn get_room(&self, id: &RoomId) -> Option<&Room> {
        self.rooms.get(id)
    }
}

pub fn get_world() -> World {
    let rooms: Vec<Room> = vec![
        Room {
            id: RoomId(0),
            name: "North Room".into(),
            description: "The room in the north".into(),
            exits: HashMap::from([
                (Direction::South, RoomId(4))
            ])
        },
        Room {
            id: RoomId(1),
            name: "West Room".into(),
            description: "The room in the west".into(),
            exits: HashMap::from([
                (Direction::East, RoomId(4))
            ])
        },
        Room {
            id: RoomId(2),
            name: "South Room".into(),
            description: "The room in the south".into(),
            exits: HashMap::from([
                (Direction::North, RoomId(4))
            ])
        },
        Room {
            id: RoomId(3),
            name: "East Room".into(),
            description: "The room in the east".into(),
            exits: HashMap::from([
                (Direction::West, RoomId(4))
            ])
        },
        Room {
            id: RoomId(4),
            name: "Central Room".into(),
            description: "The room in the middle".into(),
            exits: HashMap::from([
                (Direction::North, RoomId(0)),
                (Direction::West, RoomId(1)),
                (Direction::South, RoomId(2)),
                (Direction::East, RoomId(3)),
            ])
        }
    ];

    let rooms: HashMap<RoomId, Room> = rooms.into_iter().map(|room| (room.id.clone(), room)).collect();

    World { rooms }
}

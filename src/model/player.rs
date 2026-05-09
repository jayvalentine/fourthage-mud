use super::world::RoomId;

pub struct Player {
    current_room: RoomId
}

impl Player {
    pub fn new(starting_room: RoomId) -> Player {
        Player { current_room: starting_room }
    }
}

use super::world::RoomId;

pub struct Player {
    name: String,
    current_room: RoomId
}

impl Player {
    pub fn new(starting_room: RoomId) -> Player {
        Player { name: "Player".into(), current_room: starting_room }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

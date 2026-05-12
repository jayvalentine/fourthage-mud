use super::world::RoomId;

pub struct Player {
    name: String,
    current_room: RoomId
}

impl Player {
    pub fn new(name: String, starting_room: RoomId) -> Player {
        Player { name, current_room: starting_room }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn current_room(&self) -> &RoomId {
        &self.current_room
    }

    pub fn move_to(&mut self, destination_room_id: &RoomId) {
        self.current_room = destination_room_id.clone();
    }
}

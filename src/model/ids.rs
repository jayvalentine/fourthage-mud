use serde::Deserialize;
use sqlx::types::Uuid;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct EntityId(Uuid);

impl EntityId {
    pub fn new(id: Uuid) -> EntityId {
        EntityId(id)
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Deserialize, Debug)]
pub struct RoomId(i32);

impl RoomId {
    pub fn new(id: i32) -> RoomId {
        RoomId(id)
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}

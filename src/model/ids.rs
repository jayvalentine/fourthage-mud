use std::fmt::Display;

use uuid::Uuid;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct EntityId(Uuid);

impl EntityId {
    pub fn from_uuid(id: Uuid) -> EntityId {
        EntityId(id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_uuid())
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct RoomId(EntityId);

impl RoomId {
    pub fn from_entity(id: EntityId) -> RoomId {
        RoomId(id)
    }

    pub fn from_uuid(id: Uuid) -> RoomId {
        RoomId(EntityId(id))
    }

    pub fn as_entity(&self) -> EntityId {
        self.0
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0.as_uuid()
    }
}

impl Display for RoomId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_uuid())
    }
}

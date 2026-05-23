use uuid::Uuid;

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct EntityId(Uuid);

impl EntityId {
    pub fn new(id: Uuid) -> EntityId {
        EntityId(id)
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub struct RoomId(EntityId);

impl RoomId {
    pub fn new(id: EntityId) -> RoomId {
        RoomId(id)
    }

    pub fn from_uuid(id: Uuid) -> RoomId {
        RoomId(EntityId(id))
    }

    pub fn value(&self) -> EntityId {
        self.0
    }

    pub fn uuid(&self) -> Uuid {
        self.0.0
    }
}

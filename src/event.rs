use std::{collections::HashMap, str::FromStr, sync::Arc};

use async_trait::async_trait;
use tokio::sync::mpsc::{self, error::SendError};
use parking_lot::Mutex;

use crate::{entities::{Location, Name}, model::ids::EntityId};

#[derive(Debug)]
pub enum EventTarget {
    /// The named entity.
    Entity(EntityId),

    /// All entities in the given location, except the named entity.
    LocationExcept(Location, EntityId)
}

#[derive(Clone, Debug)]
pub enum GameEvent {
    EntityEntered(EntityId),
    EntityLeft(EntityId),

    Message(String),
    
    /// id, name, message
    EntitySaid(EntityId, Name, String),

    /// id, name, emote
    EntityEmoted(EntityId, Name, String),

    SessionEnded,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EventType {
    EntityEntered,
    EntityLeft,
    Message,
    EntitySaid,
    EntityEmoted,
    SessionEnded
}

impl From<&GameEvent> for EventType {
    fn from(event: &GameEvent) -> Self {
        match event {
            GameEvent::EntityEntered(_) => EventType::EntityEntered,
            GameEvent::EntityLeft(_) => EventType::EntityLeft,
            GameEvent::Message(_) => EventType::Message,
            GameEvent::EntitySaid(_, _, _) => EventType::EntitySaid,
            GameEvent::EntityEmoted(_, _, _) => EventType::EntityEmoted,
            GameEvent::SessionEnded => EventType::SessionEnded
        }
    }
}

impl ToString for EventType {
    fn to_string(&self) -> String {
        match self {
            EventType::EntityEntered => "entity_entered".to_string(),
            EventType::EntityLeft => "entity_left".to_string(),
            EventType::Message => "message".to_string(),
            EventType::EntitySaid => "entity_said".to_string(),
            EventType::EntityEmoted => "entity_emoted".to_string(),
            EventType::SessionEnded => "session_ended".to_string()
        }
    }
}

impl FromStr for EventType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entity_entered" => Ok(EventType::EntityEntered),
            "entity_left" => Ok(EventType::EntityLeft),
            "message" => Ok(EventType::Message),
            "entity_said" => Ok(EventType::EntitySaid),
            "entity_emoted" => Ok(EventType::EntityEmoted),
            "session_ended" => Ok(EventType::SessionEnded),
            _ => Err(())
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub target: EventTarget,
    pub event: GameEvent
}

#[derive(Debug)]
pub enum EventBusError {
    CouldNotSend
}

impl From<SendError<GameEvent>> for EventBusError {
    fn from(_: SendError<GameEvent>) -> Self {
        EventBusError::CouldNotSend
    }
}

pub trait EventTargetResolver<T> {
    fn resolve(&self, target: &EventTarget) -> Result<Vec<EntityId>, T>;
}

#[async_trait]
pub trait EventSender: Send + Sync {
    async fn send(&self, event: GameEvent) -> Result<(), SendError<GameEvent>>;
}

pub struct SessionEventSender {
    sender: mpsc::Sender<GameEvent>
}

#[async_trait]
impl EventSender for SessionEventSender {
    async fn send(&self, event: GameEvent) -> Result<(), SendError<GameEvent>> {
        self.sender.send(event).await
    }
}

impl SessionEventSender {
    pub fn new(sender: mpsc::Sender<GameEvent>) -> Self {
        SessionEventSender { sender }
    }
}

pub struct NpcEventSender {
    entity_id: EntityId,
    sender: mpsc::Sender<(EntityId, GameEvent)>
}

#[async_trait]
impl EventSender for NpcEventSender {
    async fn send(&self, event: GameEvent) -> Result<(), SendError<GameEvent>> {
        self.sender.send((self.entity_id.clone(), event)).await.map_err(|e| SendError(e.0 .1))
    }
}

impl NpcEventSender {
    pub fn new(entity_id: EntityId, sender: mpsc::Sender<(EntityId, GameEvent)>) -> Self {
        NpcEventSender { entity_id, sender }
    }
}

pub struct EventBus {
    subscribers: Mutex<HashMap<EntityId, Arc<dyn EventSender>>>
}

impl EventBus {
    pub const BUFFER_SIZE: usize = 32;

    pub fn new() -> EventBus {
        EventBus { subscribers: Mutex::new(HashMap::new()) }
    }

    pub fn register(&self, id: &EntityId, sender: Arc<dyn EventSender>) {
        self.subscribers.lock().insert(id.clone(), sender);
        tracing::debug!("Entity '{id:?}' registered on event bus");
    }

    pub fn unregister(&self, id: &EntityId) {
        self.subscribers.lock().remove(id);
        tracing::debug!("Entity '{id:?}' un-registered from event bus");
    }

    fn resolve_targets(subscribers: &HashMap<EntityId, Arc<dyn EventSender>>, targets: &[EntityId]) -> Vec<Arc<dyn EventSender>> {
        let mut senders = Vec::new();
        for target in targets {
            tracing::debug!("Resolved target entity: {:?}", target);
            if let Some(t) = subscribers.get(target) {
                tracing::debug!("Got sender for entity: {:?}", target);
                senders.push(t.clone());
            }
        };
        senders
    }

    pub async fn publish(&self, event: &GameEvent, targets: &[EntityId]) -> Result<(), EventBusError> {
        tracing::debug!("Publishing event: {0:?}", event);
        let senders: Vec<_> = {
            let subscribers = self.subscribers.lock();
            Self::resolve_targets(&subscribers, targets)
        };

        for sender in senders {
            sender.send(event.clone()).await?;
        };
        Ok(())
    }
}

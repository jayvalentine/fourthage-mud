use std::{collections::HashMap, sync::{Mutex, PoisonError}};

use tokio::sync::mpsc::{self, error::SendError};

use crate::{entities::EntityId, model::world::RoomId};

pub enum EventTarget {
    Entity(EntityId),
    RoomExcept(RoomId, EntityId)
}

#[derive(Clone, Debug)]
pub enum GameEvent {
    Message(String),
    SessionEnded
}

pub struct Event {
    pub target: EventTarget,
    pub event: GameEvent
}

pub enum EventBusError {
    InvalidMutex,
    CouldNotSend
}

impl<T> From<PoisonError<T>> for EventBusError {
    fn from(_: PoisonError<T>) -> Self {
        EventBusError::InvalidMutex
    }
}

impl From<SendError<GameEvent>> for EventBusError {
    fn from(_: SendError<GameEvent>) -> Self {
        EventBusError::CouldNotSend
    }
}

pub trait EventTargetResolver<T> {
    fn resolve(&self, target: &EventTarget) -> Result<Vec<EntityId>, T>;
}

pub struct EventBus {
    subscribers: Mutex<HashMap<EntityId, mpsc::Sender<GameEvent>>>
}

impl EventBus {
    const BUFFER_SIZE: usize = 32;

    pub fn new() -> EventBus {
        EventBus { subscribers: Mutex::new(HashMap::new()) }
    }

    pub fn register(&self, id: &EntityId) -> Result<mpsc::Receiver<GameEvent>, EventBusError> {
        let (tx, rx) = mpsc::channel::<GameEvent>(Self::BUFFER_SIZE);
        self.subscribers.lock()?.insert(id.clone(), tx);

        tracing::debug!("Entity '{id:?}' registered on event bus");
        Ok(rx)
    }

    pub fn unregister(&self, id: &EntityId) -> Result<(), EventBusError> {
        self.subscribers.lock()?.remove(id);
        tracing::debug!("Entity '{id:?}' un-registered from event bus");
        Ok(())
    }

    fn resolve_targets(subscribers: &HashMap<EntityId, mpsc::Sender<GameEvent>>, targets: &[EntityId]) -> Vec<mpsc::Sender<GameEvent>> {
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
            let subscribers = self.subscribers.lock()?;
            Self::resolve_targets(&subscribers, targets)
        };

        for sender in senders {
            sender.send(event.clone()).await?;
        };
        Ok(())
    }
}

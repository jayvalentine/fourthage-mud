use std::{collections::HashMap, sync::{Mutex, PoisonError}};

use tokio::sync::mpsc::{self, error::SendError};

pub enum EventTarget {
    Player(String)
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

pub struct EventBus {
    subscribers: Mutex<HashMap<String, mpsc::Sender<GameEvent>>>
}

impl EventBus {
    const BUFFER_SIZE: usize = 32;

    pub fn new() -> EventBus {
        EventBus { subscribers: Mutex::new(HashMap::new()) }
    }

    pub fn register(&self, id: &str) -> Result<mpsc::Receiver<GameEvent>, EventBusError> {
        let (tx, rx) = mpsc::channel::<GameEvent>(Self::BUFFER_SIZE);
        self.subscribers.lock()?.insert(id.into(), tx);
        Ok(rx)
    }

    pub fn unregister(&self, id: &str) -> Result<(), EventBusError> {
        self.subscribers.lock()?.remove(id);
        Ok(())
    }

    fn resolve_targets(subscribers: &HashMap<String, mpsc::Sender<GameEvent>>, event_target: &EventTarget) -> Vec<mpsc::Sender<GameEvent>> {
        let players = match event_target {
            EventTarget::Player(name) => {
                match subscribers.get(name) {
                    Some(p) => vec![p.clone()],
                    None => Vec::new()
                }
            }
        };
        players
    }

    pub async fn publish(&self, event: &Event) -> Result<(), EventBusError> {
        tracing::debug!("Publishing event: {0:?}", event.event);
        let senders: Vec<_> = {
            let subscribers = self.subscribers.lock()?;
            Self::resolve_targets(&subscribers, &event.target)
        };

        for sender in senders {
            sender.send(event.event.clone()).await?;
        };
        Ok(())
    }
}

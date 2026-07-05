use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::{entities::{AiBehavior, EventHandler, Location, Name}, event::{Event, EventTarget, EventTargetResolver, GameEvent}, model::ids::EntityId, system::{System, SystemContext, SystemError}};

pub struct BehaviorContext<'a> {
    entity: &'a EntityId,
    events: Vec<GameEvent>
}

pub enum BehaviorAction {
    Say(String),
    Emote(String)
}

#[derive(Debug)]
pub enum BehaviorError {

}

pub trait Behavior: Send + Sync {
    fn on_tick(&self, ctx: &BehaviorContext) -> Result<Vec<BehaviorAction>, BehaviorError>;
}

pub struct DogBehavior;

impl Behavior for DogBehavior {
    fn on_tick(&self, ctx: &BehaviorContext) -> Result<Vec<BehaviorAction>, BehaviorError> {
        let mut actions = Vec::new();
        for event in &ctx.events {
            match event {
                GameEvent::PlayerSaid(_, msg) => {
                    let msg = msg.to_ascii_lowercase();
                    if msg.contains("woof") || msg.contains("bark") {
                        actions.push(BehaviorAction::Emote("tilts its head in confusion.".to_string()));
                    } else if msg.contains("food") || msg.contains("treat") {
                        actions.push(BehaviorAction::Emote("wags its tail excitedly!".to_string()));
                    } else {
                        actions.push(BehaviorAction::Say("Woof!".to_string()));
                    }
                },
                _ => {}
            }
        }
        Ok(actions)
    }
}

pub struct BehaviorRegistry {
    behaviors: HashMap<String, Box<dyn Behavior>>
}

impl BehaviorRegistry {
    pub fn new() -> Self {
        let mut behaviors: HashMap<String, Box<dyn Behavior>> = HashMap::new();
        behaviors.insert("dog".to_string(), Box::new(DogBehavior));
        BehaviorRegistry { behaviors }
    }
}

pub struct BehaviorSystem {
    registry: BehaviorRegistry,
    receiver: mpsc::Receiver<(EntityId, GameEvent)>
}

impl BehaviorSystem {
    pub fn new(registry: BehaviorRegistry, receiver: mpsc::Receiver<(EntityId, GameEvent)>) -> Self {
        BehaviorSystem { registry, receiver }
    }
}

#[async_trait]
impl System for BehaviorSystem {
    fn name(&self) ->  &str {
        "BehaviorSystem"
    }

    async fn run(&mut self, context: &SystemContext) -> Result<(), SystemError> {
        loop {
            match self.receiver.try_recv() {
                Ok((entity_id, event)) => {
                    tracing::debug!("Received event for entity {}: {:?}", entity_id, event);
                    if let Err(e) = context.entities().push_event(&entity_id, event) {
                        tracing::error!("Failed to push event for entity {}: {:?}", entity_id, e);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    tracing::warn!("NPC event channel disconnected");
                    break;
                }
            }
        }

        let entities_to_process: Vec<(EntityId, Name, Location, AiBehavior)> = context.entities()
                                                                      .query3::<Name, Location, AiBehavior, _, _>(|iter| Ok(iter.map(|(e, (name, loc, ai))| (*e, name.clone(), loc.clone(), ai.clone())).collect()))?;

        for (entity, name, loc, ai_behavior) in entities_to_process {
            if let Some(behavior) = self.registry.behaviors.get(&ai_behavior.template) {
                let events = context.entities().events(&entity)?;

                tracing::debug!("Processing behavior for entity {} (events: {:?})", entity, events);
                let behavior_context = BehaviorContext { entity: &entity, events };
                match behavior.on_tick(&behavior_context) {
                    Ok(actions) => {
                        let mut events = Vec::new();
                        for action in actions {
                            match action {
                                BehaviorAction::Say(message) => {
                                    let message = format!("{} says: {}", name, message);
                                    let event = Event {
                                        target: EventTarget::LocationExcept(loc.clone(), entity),
                                        event: GameEvent::Message(message)
                                    };
                                    events.push(event);
                                },
                                BehaviorAction::Emote(emote) => {
                                    let message = format!("{} {}", name, emote);
                                    let event = Event {
                                        target: EventTarget::LocationExcept(loc.clone(), entity),
                                        event: GameEvent::Message(message)
                                    };
                                    events.push(event);
                                }

                            }
                        }

                        for event in events {
                            let targets = match context.entities().resolve(&event.target) {
                                Ok(targets) => targets,
                                Err(e) => {
                                    tracing::error!("Failed to resolve targets for event {:?}: {:?}", event, e);
                                    continue;
                                }
                            };
                            if let Err(e) = context.event_bus().publish(&event.event, &targets).await {
                                tracing::error!("Failed to publish event for entity {}: {:?} ({:?})", entity, event, e);
                            }
                        }
                    },
                    Err(e) => {
                        tracing::error!("Behavior {} for entity {} returned an error: {:?}", ai_behavior.template, entity, e);
                    }
                }
            } else {
                tracing::error!("No behavior found for template {} for entity {}", ai_behavior.template, entity);
            }
        }

        Ok(())
    }
}

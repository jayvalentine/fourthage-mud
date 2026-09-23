use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::sync::mpsc::{self, error::TryRecvError};

use crate::{entities::{AiBehavior, EntityRegistry, Location, Name}, event::{Event, EventTarget, EventTargetResolver, GameEvent}, model::ids::EntityId, script::{ScriptEngine, ScriptError, ScriptEvent, ScriptNpc}, system::{System, SystemContext, SystemError}};

pub struct BehaviorContext {
    entity: EntityId,
    entity_registry: Arc<EntityRegistry>,
    events: Vec<GameEvent>
}

pub enum BehaviorAction {
    Say(String),
    Emote(String)
}

#[derive(Debug)]
pub enum BehaviorError {
    Script(ScriptError)
}

impl From<ScriptError> for BehaviorError {
    fn from(value: ScriptError) -> Self {
        Self::Script(value)
    }
}

pub trait Behavior: Send + Sync {
    fn on_tick(&self, ctx: BehaviorContext) -> Result<Vec<BehaviorAction>, BehaviorError>;
}

pub struct ScriptBehavior {
    script: String,
    engine: Arc<Mutex<ScriptEngine>>
}

impl Behavior for ScriptBehavior {
    fn on_tick(&self, ctx: BehaviorContext) -> Result<Vec<BehaviorAction>, BehaviorError> {
        let npc = Arc::new(ScriptNpc::new(ctx.entity, ctx.entity_registry));

        let events: Vec<Arc<ScriptEvent>> = ctx.events
            .into_iter()
            .map(|e| Arc::new(ScriptEvent::from(e)))
            .collect();

        self.engine.lock().call::<_, ()>(&self.script, "on_tick", (npc.clone(), events))?;

        Ok(npc.actions())
    }
}

pub struct BehaviorRegistry {
    script_engine: Arc<Mutex<ScriptEngine>>,
    behaviors: HashMap<String, Box<dyn Behavior>>
}

impl BehaviorRegistry {
    pub fn new(script_engine: Arc<Mutex<ScriptEngine>>) -> Self {
        let behaviors: HashMap<String, Box<dyn Behavior>> = HashMap::new();
        BehaviorRegistry { script_engine, behaviors }
    }

    pub fn get<'a>(&'a mut self, template: &str) -> Option<&'a Box<dyn Behavior>> {
        // If the behavior does not exist in the registry,
        // it might be a script.
        // If so, load it and cache for later use.
        if self.behaviors.get(template).is_none() {
            match std::fs::exists(template) {
                Ok(true) => {
                    let b = ScriptBehavior { script: template.into(), engine: self.script_engine.clone() };
                    self.behaviors.insert(template.into(), Box::new(b));
                },
                _ => return None
            }
        }

        self.behaviors.get(template)
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
            if let Some(behavior) = self.registry.get(&ai_behavior.template) {
                let events = context.entities().events(&entity)?;

                tracing::debug!("Processing behavior for entity {} (events: {:?})", entity, events);
                let behavior_context = BehaviorContext { entity, entity_registry: context.entities().clone(), events };
                match behavior.on_tick(behavior_context) {
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

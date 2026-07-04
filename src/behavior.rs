use std::collections::HashMap;

use async_trait::async_trait;

use crate::{entities::{AiBehavior, Location, Name}, event::{Event, EventTarget, EventTargetResolver, GameEvent}, model::ids::EntityId, system::{System, SystemContext, SystemError}};

pub struct BehaviorContext<'a> {
    entity: &'a EntityId
}

pub enum BehaviorAction {
    Say(String)
}

#[derive(Debug)]
pub enum BehaviorError {

}

pub trait Behavior: Send + Sync {
    fn on_tick(&self, ctx: &BehaviorContext) -> Result<Vec<BehaviorAction>, BehaviorError>;
}

pub struct DogBehavior;

impl Behavior for DogBehavior {
    fn on_tick(&self, _: &BehaviorContext) -> Result<Vec<BehaviorAction>, BehaviorError> {
        Ok(vec![BehaviorAction::Say("Woof!".to_string())])
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
    registry: BehaviorRegistry
}

impl BehaviorSystem {
    pub fn new(registry: BehaviorRegistry) -> Self {
        BehaviorSystem { registry }
    }
}

#[async_trait]
impl System for BehaviorSystem {
    fn name(&self) ->  &str {
        "BehaviorSystem"
    }

    async fn run(&self,context: &SystemContext) -> Result<(), SystemError> {
        let entities_to_process: Vec<(EntityId, Name, Location, AiBehavior)> = context.entities()
                                                                      .query3::<Name, Location, AiBehavior, _, _>(|iter| Ok(iter.map(|(e, (name, loc, ai))| (*e, name.clone(), loc.clone(), ai.clone())).collect()))?;

        for (entity, name, loc, ai_behavior) in entities_to_process {
            if let Some(behavior) = self.registry.behaviors.get(&ai_behavior.template) {
                let behavior_context = BehaviorContext { entity: &entity };
                match behavior.on_tick(&behavior_context) {
                    Ok(actions) => {
                        for action in actions {
                            match action {
                                BehaviorAction::Say(message) => {
                                    let message = format!("{} says: {}", name, message);
                                    let event = Event {
                                        target: EventTarget::LocationExcept(loc.clone(), entity),
                                        event: GameEvent::Message(message)
                                    };

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

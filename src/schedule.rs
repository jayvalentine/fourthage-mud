use async_trait::async_trait;
use serde::Deserialize;

use crate::{event::{Event, EventTarget, EventTargetResolver, GameEvent}, model::time::{Tick, TickInterval}, system::System};
use crate::system::{SystemContext, SystemError};

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Schedule {
    Cycle(CycleSchedule)
}

#[derive(Deserialize)]
pub struct CycleSchedule {
    pub name: String,
    pub phases: Vec<CyclePhase>,
    pub ticks_per_phase: TickInterval
}

impl CycleSchedule {
    /// Get the index into the phase array on the given tick.
    /// This is guaranteed to return a value within the bounds of self.phases.
    fn phase_index(&self, tick: Tick) -> usize {
        (tick / self.ticks_per_phase) as usize % self.phases.len()
    }

    /// Get the current phase.
    pub fn current_phase(&self, tick: Tick) -> &CyclePhase {
        let i = self.phase_index(tick);
        &self.phases.get(i).expect(&format!("phase_index returned invalid index '{}' (len: {})", i, self.phases.len()))
    }

    pub fn phase_changed(&self, tick: Tick) -> bool {
        if tick == 0 {
            return false;
        }

        self.current_phase(tick) != self.current_phase(tick - 1.into())
    }
}

#[derive(Deserialize, PartialEq)]
pub struct CyclePhase {
    pub name: String,
    pub announcement: Option<String>
}

pub struct SchedulerSystem {
    schedules: Vec<Schedule>
}

impl SchedulerSystem {
    pub fn new(schedules: Vec<Schedule>) -> SchedulerSystem {
        SchedulerSystem { schedules }
    }
}

#[async_trait]
impl System for SchedulerSystem {
    fn name(&self) -> &str {
        "SchedulerSystem"
    }

    async fn run(&mut self, context: &SystemContext) -> Result<(), SystemError> {
        let current_tick = context.current_tick();

        for s in &self.schedules {
            match s {
                Schedule::Cycle(c) => {
                    if c.phase_changed(current_tick) {
                        let p = c.current_phase(current_tick);
                        let event = Event {
                            target: EventTarget::All,
                            event: GameEvent::CyclePhaseChanged(c.name.clone(), p.name.clone(), p.announcement.clone())
                        };
                        let entities = context.entities().resolve(&event.target)?;
                        context.event_bus().publish(&event.event, &entities).await?;
                    }
                }
            }
        };
        Ok(())
    }
}

use async_trait::async_trait;
use serde::Deserialize;

use crate::{event::{Event, EventTarget, EventTargetResolver, GameEvent}, model::time::{Tick, TickInterval}, system::System};
use crate::system::{SystemContext, SystemError};

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Schedule {
    Cycle(CycleSchedule),
    Loop(LoopSchedule)
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

    /// Returns true if the phase has changed since the last tick.
    pub fn phase_changed(&self, tick: Tick) -> bool {
        if tick == 0 {
            return true;
        }

        self.phase_index(tick) != self.phase_index(tick - 1.into())
    }
}

#[derive(Deserialize, PartialEq)]
pub struct CyclePhase {
    pub name: String,
    pub announcement: Option<String>
}

#[derive(Deserialize)]
pub struct LoopSchedule {
    pub name: String,
    pub stages: Vec<LoopStage>
}

impl LoopSchedule {
    fn stage_index(&self, tick: Tick) -> usize {
        let total_ticks: TickInterval = self.stages.iter().map(|s| s.ticks).fold(0.into(), |acc, e| acc + e);
        let tick_within_loop = tick % total_ticks;

        let mut index = 0;
        let mut acc = TickInterval::default();
        for s in &self.stages {
            acc += s.ticks;
            if tick_within_loop < acc {
                return index;
            }

            index += 1;
        }

        panic!("Found tick interval out of stage range: {tick_within_loop}");
    }

    pub fn current_stage(&self, tick: Tick) -> &LoopStage {
        let i = self.stage_index(tick);
        &self.stages.get(i).expect(&format!("stage_index returned invalid index '{}' (len: {})", i, self.stages.len()))
    }

    /// Returns true if the stage has changed since the last tick.
    pub fn stage_changed(&self, tick: Tick) -> bool {
        if tick == 0 {
            return true;
        }

        self.stage_index(tick) != self.stage_index(tick - 1.into())
    }
}

#[derive(Deserialize)]
pub struct LoopStage {
    pub name: String,
    pub ticks: TickInterval,
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
                        tracing::debug!("Phase changed for {}: {}", c.name, p.name);

                        let event = Event {
                            target: EventTarget::All,
                            event: GameEvent::Scheduler(c.name.clone(), p.name.clone(), p.announcement.clone())
                        };
                        let entities = context.entities().resolve(&event.target)?;
                        context.event_bus().publish(&event.event, &entities).await?;
                    }
                },
                Schedule::Loop(l) => {
                    if l.stage_changed(current_tick) {
                        let s = l.current_stage(current_tick);
                        tracing::debug!("Stage changed for {}: {}", l.name, s.name);

                        let event = Event {
                            target: EventTarget::All,
                            event: GameEvent::Scheduler(l.name.clone(), s.name.clone(), s.announcement.clone())
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

use std::{fmt::Display, ops::{Add, AddAssign, Div, Sub}};

use serde::{Deserialize, Serialize};

type TickType = u64;

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tick(TickType);

impl From<TickType> for Tick {
    fn from(value: TickType) -> Self {
        Tick(value)
    }
}

impl Display for Tick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for Tick {
    fn default() -> Self {
        Tick(0)
    }
}

impl Div<TickInterval> for Tick {
    type Output = TickType;

    fn div(self, rhs: TickInterval) -> Self::Output {
        self.0 / rhs.0
    }
}

impl Sub<TickInterval> for Tick {
    type Output = Tick;

    fn sub(self, rhs: TickInterval) -> Self::Output {
        Self::Output { 0: self.0 - rhs.0 }
    }
}

impl Add<TickInterval> for Tick {
    type Output = Tick;

    fn add(self, rhs: TickInterval) -> Self::Output {
        Self::Output { 0: self.0 + rhs.0 }
    }
}

impl AddAssign<TickInterval> for Tick {
    fn add_assign(&mut self, rhs: TickInterval) {
        self.0 += rhs.0
    }
}

impl PartialEq<TickType> for Tick {
    fn eq(&self, other: &TickType) -> bool {
        self.0 == *other
    }

    fn ne(&self, other: &TickType) -> bool {
        self.0 != *other
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TickInterval(TickType);

impl From<TickType> for TickInterval {
    fn from(value: TickType) -> Self {
        TickInterval(value)
    }
}

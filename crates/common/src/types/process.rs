/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Linux process identifier.

use std::{borrow::Borrow, fmt, num::ParseIntError, str::FromStr};

use serde::{Deserialize, Serialize};

/// Linux process identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Pid(pub u32);

impl From<u32> for Pid {
    fn from(pid: u32) -> Self {
        Self(pid)
    }
}

impl From<Pid> for u32 {
    fn from(pid: Pid) -> Self {
        pid.0
    }
}

impl AsRef<u32> for Pid {
    fn as_ref(&self) -> &u32 {
        &self.0
    }
}

impl Borrow<u32> for Pid {
    fn borrow(&self) -> &u32 {
        &self.0
    }
}

impl FromStr for Pid {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Start time of a process, as reported by `/proc/[pid]/stat` (field 22).
///
/// The value counts clock ticks since boot and identifies one concrete process
/// instance, which is what makes it usable to detect PID reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct StartTime(pub u64);

impl From<u64> for StartTime {
    fn from(start_time: u64) -> Self {
        Self(start_time)
    }
}

impl From<StartTime> for u64 {
    fn from(start_time: StartTime) -> Self {
        start_time.0
    }
}

impl AsRef<u64> for StartTime {
    fn as_ref(&self) -> &u64 {
        &self.0
    }
}

impl Borrow<u64> for StartTime {
    fn borrow(&self) -> &u64 {
        &self.0
    }
}

impl FromStr for StartTime {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl fmt::Display for StartTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

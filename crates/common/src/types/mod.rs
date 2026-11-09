/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Typed identifiers and enumerations shared across `PipeLink` crates.

mod application;
mod global;
mod link;
mod media;
mod node;
mod port;
mod process;
mod routing;

pub use application::{AppDisplay, AppId, SteamAppId};
pub use global::GlobalId;
pub use link::LinkId;
pub use media::MediaClass;
pub use node::{NodeId, NodeName, NodeSerial};
pub use port::{ChannelPosition, PortDirection, PortId};
pub use process::{Pid, StartTime};
pub use routing::{ChannelName, MixName};

#[cfg(test)]
mod tests;

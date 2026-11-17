/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Domain events for audio routing control and state notifications.

mod control_event;
mod control_reply;
mod state_event;

pub use control_event::ControlEvent;
pub use control_reply::{
    ActiveAppInfo, ChannelVolumeInfo, ControlReply, GraphSnapshot, InstalledAppInfo, MixRouteInfo,
    OutputDeviceInfo,
};
pub use state_event::StateEvent;

#[cfg(test)]
mod snapshot_tests;
#[cfg(test)]
mod tests;

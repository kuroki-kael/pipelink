/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Real-time daemon `StateEvent` synchronization against the local `GraphSnapshot`.

use pipelink_common::{
    events::{MixRouteInfo, StateEvent},
    types::{ChannelName, MixName},
};

use super::App;

impl App {
    /// Applies a live [`StateEvent`] onto `self.snapshot` and reports whether a full
    /// snapshot refresh is needed (e.g. when hardware output active flags or app stream
    /// counts changed).
    pub(super) fn apply_state_event(&mut self, event: StateEvent) -> bool {
        match event {
            StateEvent::GainChanged {
                channel,
                mix,
                percent,
                ..
            } => {
                upsert_route(&mut self.snapshot.routes, channel, mix, |r| {
                    r.gain_percent = percent;
                });
                false
            },
            StateEvent::MuteChanged {
                channel,
                mix,
                muted,
            } => {
                upsert_route(&mut self.snapshot.routes, channel, mix, |r| {
                    r.muted = muted;
                });
                false
            },
            StateEvent::GlobalLayerSwitched { mix } => {
                self.snapshot.active_mix = Some(mix);
                false
            },
            StateEvent::ChannelAdded { channel } => {
                self.on_channel_added(&channel);
                false
            },
            StateEvent::ChannelRemoved { channel } => {
                self.snapshot.channels.retain(|c| c != &channel);
                self.snapshot.routes.retain(|r| r.channel != channel);
                self.snapshot
                    .app_routes
                    .retain(|(_, ch)| !ch.eq_ignore_ascii_case(&channel));
                false
            },
            StateEvent::MixAdded { mix } => {
                self.on_mix_added(&mix);
                false
            },
            StateEvent::MixRemoved { mix } => {
                self.snapshot.mixes.retain(|m| m != &mix);
                self.snapshot.routes.retain(|r| r.mix != mix);
                true
            },
            StateEvent::MixChannelAdded { channel, mix } => {
                upsert_route(&mut self.snapshot.routes, channel, mix, |r| {
                    r.active = true;
                });
                false
            },
            StateEvent::MixChannelRemoved { channel, mix } => {
                upsert_route(&mut self.snapshot.routes, channel, mix, |r| {
                    r.active = false;
                });
                false
            },
            StateEvent::OutputChanged { output } => {
                for out in &mut self.snapshot.outputs {
                    out.active = out.name == output;
                }
                true
            },
            StateEvent::DefaultSinkChanged { sink } => {
                self.snapshot.default_sink = Some(sink);
                false
            },
            StateEvent::ChannelCreationFailed { error, .. }
            | StateEvent::MixCreationFailed { error, .. }
            | StateEvent::ChannelRemovalFailed { error, .. }
            | StateEvent::MixRemovalFailed { error, .. }
            | StateEvent::RouteOperationFailed { error, .. }
            | StateEvent::AppRoutingFailed { error, .. }
            | StateEvent::OutputOperationFailed { error }
            | StateEvent::LayerSelectionFailed { error, .. }
            | StateEvent::DefaultSinkOperationFailed { error } => {
                self.last_error = Some(error);
                false
            },
            StateEvent::AppRouted { .. } | StateEvent::AppUnrouted { .. } => true,
            StateEvent::ChannelLayerSwitched { .. } | _ => false,
        }
    }

    fn on_channel_added(&mut self, channel: &ChannelName) {
        if self.snapshot.channels.contains(channel) {
            return;
        }
        self.snapshot.channels.push(channel.clone());
        for mix in &self.snapshot.mixes {
            self.snapshot.routes.push(MixRouteInfo {
                channel: channel.clone(),
                mix: mix.clone(),
                active: false,
                gain_percent: 100.0,
                muted: false,
            });
        }
    }

    fn on_mix_added(&mut self, mix: &MixName) {
        if self.snapshot.mixes.contains(mix) {
            return;
        }
        self.snapshot.mixes.push(mix.clone());
        for channel in &self.snapshot.channels {
            self.snapshot.routes.push(MixRouteInfo {
                channel: channel.clone(),
                mix: mix.clone(),
                active: false,
                gain_percent: 100.0,
                muted: false,
            });
        }
    }
}

fn upsert_route(
    routes: &mut Vec<MixRouteInfo>,
    channel: ChannelName,
    mix: MixName,
    update: impl FnOnce(&mut MixRouteInfo),
) {
    if let Some(existing) = routes
        .iter_mut()
        .find(|r| r.channel == channel && r.mix == mix)
    {
        update(existing);
        return;
    }

    let mut cell = MixRouteInfo {
        channel,
        mix,
        active: false,
        gain_percent: 100.0,
        muted: false,
    };
    update(&mut cell);
    routes.push(cell);
}

/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Real-time daemon `StateEvent` synchronization against the local `GraphSnapshot`.

use iced::Task;
use pipelink_common::{
    events::{ChannelVolumeInfo, MixRouteInfo, StateEvent},
    types::{ChannelName, MixName, NodeName},
};

use super::{App, Message, actions};

impl App {
    /// Applies a live [`StateEvent`] onto `self.snapshot` and returns any follow-up
    /// background task (e.g. snapshot refresh or pending app-to-channel auto-routing).
    pub(super) fn apply_state_event(&mut self, event: StateEvent) -> Task<Message> {
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
            },
            StateEvent::MuteChanged {
                channel,
                mix,
                muted,
            } => {
                upsert_route(&mut self.snapshot.routes, channel, mix, |r| {
                    r.muted = muted;
                });
            },
            StateEvent::ChannelMasterChanged {
                channel,
                percent,
                muted,
            } => self.on_channel_master_changed(channel, percent, muted),
            StateEvent::GlobalLayerSwitched { mix } => {
                self.snapshot.active_mix = Some(mix);
            },
            StateEvent::ChannelAdded { channel } => {
                return self.handle_channel_added_event(channel);
            },
            StateEvent::ChannelRemoved { channel } => self.on_channel_removed(&channel),
            StateEvent::ChannelRenamed { old_name, new_name } => {
                self.on_channel_renamed(&old_name, &new_name);
            },
            StateEvent::ChannelsReordered { channels } => {
                self.snapshot.channels = channels;
            },
            StateEvent::MixAdded { mix } => self.on_mix_added(&mix),
            StateEvent::MixRenamed { old_name, new_name } => {
                self.on_mix_renamed(&old_name, &new_name);
            },
            StateEvent::MixRemoved { mix } => return self.on_mix_removed(&mix),
            StateEvent::MixChannelAdded { channel, mix } => {
                upsert_route(&mut self.snapshot.routes, channel, mix, |r| {
                    r.active = true;
                });
            },
            StateEvent::MixChannelRemoved { channel, mix } => {
                upsert_route(&mut self.snapshot.routes, channel, mix, |r| {
                    r.active = false;
                });
            },
            StateEvent::OutputChanged { output } => return self.on_output_changed(&output),
            StateEvent::DefaultSinkChanged { sink } => {
                self.snapshot.default_sink = Some(sink);
            },
            StateEvent::ChannelOperationFailed { error, channel } => {
                if self
                    .pending_route_after_create
                    .as_ref()
                    .is_some_and(|(ch, _)| ch.eq_ignore_ascii_case(&channel))
                {
                    self.pending_route_after_create = None;
                }
                self.last_error = Some(error);
            },
            StateEvent::RouteOperationFailed { error, .. }
            | StateEvent::AppRoutingFailed { error, .. }
            | StateEvent::OutputOperationFailed { error }
            | StateEvent::LayerSelectionFailed { error, .. }
            | StateEvent::DefaultSinkOperationFailed { error }
            | StateEvent::MixOperationFailed { error, .. } => {
                self.last_error = Some(error);
            },
            StateEvent::AppRouted { app, channel } => {
                self.snapshot
                    .app_routes
                    .retain(|(existing, _)| !existing.eq_ignore_ascii_case(&app));
                self.snapshot.app_routes.push((app, channel));
                return actions::fetch_snapshot(self.client.clone());
            },
            StateEvent::AppUnrouted { app } => {
                self.snapshot
                    .app_routes
                    .retain(|(existing, _)| !existing.eq_ignore_ascii_case(&app));
                return actions::fetch_snapshot(self.client.clone());
            },
            StateEvent::ChannelLayerSwitched { .. } | _ => {},
        }

        Task::none()
    }

    fn handle_channel_added_event(&mut self, channel: ChannelName) -> Task<Message> {
        self.on_channel_added(&channel);
        if let Some((pending_ch, app)) = &self.pending_route_after_create
            && pending_ch.eq_ignore_ascii_case(&channel)
        {
            let app = app.clone();
            self.pending_route_after_create = None;
            return actions::route_app(self.client.clone(), app, channel);
        }
        Task::none()
    }

    fn on_channel_master_changed(&mut self, channel: ChannelName, percent: f32, muted: bool) {
        if let Some(entry) = self
            .snapshot
            .channel_volumes
            .iter_mut()
            .find(|v| v.channel.eq_ignore_ascii_case(&channel))
        {
            entry.volume_percent = percent;
            entry.muted = muted;
        } else {
            self.snapshot.channel_volumes.push(ChannelVolumeInfo {
                channel,
                volume_percent: percent,
                muted,
            });
        }
    }

    fn on_channel_renamed(&mut self, old_name: &ChannelName, new_name: &ChannelName) {
        for ch in &mut self.snapshot.channels {
            if ch.eq_ignore_ascii_case(old_name) {
                *ch = new_name.clone();
            }
        }
        for vol in &mut self.snapshot.channel_volumes {
            if vol.channel.eq_ignore_ascii_case(old_name) {
                vol.channel = new_name.clone();
            }
        }
        for route in &mut self.snapshot.routes {
            if route.channel.eq_ignore_ascii_case(old_name) {
                route.channel = new_name.clone();
            }
        }
        for (_, ch) in &mut self.snapshot.app_routes {
            if ch.eq_ignore_ascii_case(old_name) {
                *ch = new_name.clone();
            }
        }
        let expected_old = format!("pipelink.channel.{old_name}");
        if self
            .snapshot
            .default_sink
            .as_ref()
            .is_some_and(|s| s.eq_ignore_ascii_case(&expected_old))
        {
            self.snapshot.default_sink =
                Some(NodeName::from(format!("pipelink.channel.{new_name}")));
        }
        if let Some(modal) = &mut self.modal
            && modal.channel.eq_ignore_ascii_case(old_name)
        {
            modal.channel = new_name.clone();
            modal.rename_draft = new_name.to_string();
        }
    }

    fn on_channel_removed(&mut self, channel: &ChannelName) {
        self.snapshot.channels.retain(|c| c != channel);
        self.snapshot
            .channel_volumes
            .retain(|v| !v.channel.eq_ignore_ascii_case(channel));
        self.snapshot.routes.retain(|r| r.channel != *channel);
        self.snapshot
            .app_routes
            .retain(|(_, ch)| !ch.eq_ignore_ascii_case(channel));
        if self
            .modal
            .as_ref()
            .is_some_and(|m| m.channel.eq_ignore_ascii_case(channel))
        {
            self.modal = None;
        }
    }

    fn on_channel_added(&mut self, channel: &ChannelName) {
        if self.snapshot.channels.contains(channel) {
            return;
        }
        self.snapshot.channels.push(channel.clone());
        self.snapshot.channel_volumes.push(ChannelVolumeInfo {
            channel: channel.clone(),
            volume_percent: 100.0,
            muted: false,
        });
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

    fn on_mix_renamed(&mut self, old_name: &MixName, new_name: &MixName) {
        for mix in &mut self.snapshot.mixes {
            if mix.eq_ignore_ascii_case(old_name) {
                *mix = new_name.clone();
            }
        }
        for route in &mut self.snapshot.routes {
            if route.mix.eq_ignore_ascii_case(old_name) {
                route.mix = new_name.clone();
            }
        }
        if self
            .snapshot
            .active_mix
            .as_ref()
            .is_some_and(|m| m.eq_ignore_ascii_case(old_name))
        {
            self.snapshot.active_mix = Some(new_name.clone());
        }
        if self
            .renaming_mix
            .as_ref()
            .is_some_and(|(target, _)| target.eq_ignore_ascii_case(old_name))
        {
            self.renaming_mix = None;
        }
    }

    fn on_mix_removed(&mut self, mix: &MixName) -> Task<Message> {
        self.snapshot.mixes.retain(|m| m != mix);
        self.snapshot.routes.retain(|r| r.mix != *mix);
        if self
            .renaming_mix
            .as_ref()
            .is_some_and(|(target, _)| target == mix)
        {
            self.renaming_mix = None;
        }
        actions::fetch_snapshot(self.client.clone())
    }

    fn on_output_changed(&mut self, output: &NodeName) -> Task<Message> {
        for out in &mut self.snapshot.outputs {
            out.active = out.name == *output;
        }
        actions::fetch_snapshot(self.client.clone())
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

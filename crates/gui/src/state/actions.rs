/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Asynchronous IPC actions dispatched from GUI updates.

mod apps;
mod channels;
mod routes;

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub(super) use apps::{route_app, unroute_app};
pub(super) use channels::{
    move_channel, rename_channel, set_channel_master_mute, set_channel_master_volume,
};
use iced::{Task, widget::svg};
use pipelink_common::{
    events::{ControlEvent, ControlReply, InstalledAppInfo, StateEvent},
    types::{ChannelName, MixName},
};
use pipelink_ipc::{ControlClient, UnixSocketClient};
pub(super) use routes::{
    create_channel, create_mix, remove_channel, remove_mix, rename_mix, select_active_mix,
    select_output, set_default_sink, set_gain, set_mute, toggle_route,
};

use super::{App, CreateChannelMode, Message};
use crate::icons;

const GAIN_THROTTLE_INTERVAL: Duration = Duration::from_millis(50);

/// Combined result of fetching installed `.desktop` apps and resolving their XDG icons
/// off the GUI thread.
#[derive(Debug, Clone)]
pub(crate) struct InstalledAppsPayload {
    pub(crate) apps: Vec<InstalledAppInfo>,
    pub(crate) icons: HashMap<String, svg::Handle>,
}

impl App {
    pub(super) fn handle_gain_changed(
        &mut self,
        channel: ChannelName,
        mix: MixName,
        percent: f32,
    ) -> Task<Message> {
        let rounded = percent.round();
        if let Some(route) = self
            .snapshot
            .routes
            .iter_mut()
            .find(|r| r.channel == channel && r.mix == mix)
        {
            route.gain_percent = rounded;
        }

        let key = (channel.clone(), mix.clone());
        let now = Instant::now();
        let should_send = self
            .gain_throttle
            .get(&key)
            .is_none_or(|last| now.duration_since(*last) >= GAIN_THROTTLE_INTERVAL);

        if should_send {
            self.gain_throttle.insert(key, now);
            return set_gain(self.client.clone(), channel, mix, rounded);
        }

        Task::none()
    }

    pub(super) fn handle_gain_released(
        &mut self,
        channel: ChannelName,
        mix: MixName,
    ) -> Task<Message> {
        let percent = self
            .find_route(&channel, &mix)
            .map_or(100.0, |r| r.gain_percent);
        self.gain_throttle
            .insert((channel.clone(), mix.clone()), Instant::now());
        set_gain(self.client.clone(), channel, mix, percent)
    }

    pub(super) fn handle_channel_volume_changed(
        &mut self,
        channel: ChannelName,
        percent: f32,
    ) -> Task<Message> {
        let rounded = percent.round().clamp(0.0, 200.0);
        self.set_local_channel_volume(&channel, rounded);

        let now = Instant::now();
        let should_send = self
            .master_throttle
            .get(&channel)
            .is_none_or(|last| now.duration_since(*last) >= GAIN_THROTTLE_INTERVAL);

        if should_send {
            self.master_throttle.insert(channel.clone(), now);
            return set_channel_master_volume(self.client.clone(), channel, rounded);
        }

        Task::none()
    }

    pub(super) fn handle_channel_volume_released(&mut self, channel: ChannelName) -> Task<Message> {
        let (percent, _) = self.channel_volume(&channel);
        self.master_throttle.insert(channel.clone(), Instant::now());
        set_channel_master_volume(self.client.clone(), channel, percent)
    }

    pub(super) fn submit_create_channel(&mut self) -> Task<Message> {
        let Some(CreateChannelMode::EmptyInput(name)) = &self.create_channel else {
            return Task::none();
        };
        let trimmed = name.trim().to_string();
        if trimmed.is_empty() {
            return Task::none();
        }
        self.create_channel = None;
        create_channel(self.client.clone(), ChannelName::from(trimmed))
    }

    pub(super) fn submit_create_mix(&mut self) -> Task<Message> {
        let trimmed = self.new_mix_name.trim().to_string();
        if trimmed.is_empty() {
            return Task::none();
        }
        self.creating_mix = false;
        self.new_mix_name.clear();
        create_mix(self.client.clone(), MixName::from(trimmed))
    }

    pub(super) fn submit_rename_channel(&mut self) -> Task<Message> {
        let Some(modal) = &self.modal else {
            return Task::none();
        };
        let trimmed = modal.rename_draft.trim().to_string();
        if trimmed.is_empty() || trimmed == modal.channel.as_ref() {
            return Task::none();
        }
        rename_channel(
            self.client.clone(),
            modal.channel.clone(),
            ChannelName::from(trimmed),
        )
    }

    pub(super) fn submit_rename_mix(&mut self) -> Task<Message> {
        let Some((old_mix, draft)) = self.renaming_mix.as_ref() else {
            return Task::none();
        };
        let trimmed = draft.trim().to_string();
        if trimmed.is_empty() || trimmed == old_mix.as_ref() {
            self.renaming_mix = None;
            return Task::none();
        }
        let old_name = old_mix.clone();
        self.renaming_mix = None;
        rename_mix(self.client.clone(), old_name, MixName::from(trimmed))
    }
}

pub(super) fn fetch_snapshot(client: UnixSocketClient) -> Task<Message> {
    Task::perform(
        async move {
            match client.query(ControlEvent::GetGraphSnapshot).await {
                Ok(ControlReply::GraphSnapshot(snap)) => Ok(snap),
                Ok(other) => Err(format!("Unexpected daemon reply: {other:?}")),
                Err(err) => Err(err.to_string()),
            }
        },
        Message::SnapshotLoaded,
    )
}

pub(super) fn fetch_installed_apps(client: UnixSocketClient) -> Task<Message> {
    Task::perform(
        async move {
            let apps = match client.query(ControlEvent::ListInstalledApps).await {
                Ok(ControlReply::InstalledApps { apps }) => apps,
                Ok(other) => return Err(format!("Unexpected daemon reply: {other:?}")),
                Err(err) => return Err(err.to_string()),
            };

            let resolve_input = apps.clone();
            let icons =
                tokio::task::spawn_blocking(move || icons::xdg::resolve_app_icons(&resolve_input))
                    .await
                    .unwrap_or_default();

            Ok(InstalledAppsPayload { apps, icons })
        },
        Message::InstalledAppsLoaded,
    )
}

fn send_mutation(
    client: UnixSocketClient,
    request: ControlEvent,
    matcher: impl Fn(&StateEvent) -> bool + Send + Sync + 'static,
    to_result: impl Fn(StateEvent) -> Result<(), String> + Send + 'static,
) -> Task<Message> {
    Task::perform(
        async move {
            match client.mutate(request, matcher).await {
                Ok(event) => to_result(event),
                Err(err) => Err(err.to_string()),
            }
        },
        Message::MutationFinished,
    )
}

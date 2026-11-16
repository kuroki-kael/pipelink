/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Asynchronous IPC actions dispatched from GUI updates.

mod routes;

use std::time::{Duration, Instant};

use iced::Task;
use pipelink_common::{
    events::{ControlEvent, ControlReply, StateEvent},
    types::{ChannelName, MixName},
};
use pipelink_ipc::{ControlClient, UnixSocketClient};
pub(super) use routes::{
    create_channel, create_mix, remove_channel, remove_mix, select_active_mix, select_output,
    set_default_sink, set_gain, set_mute, toggle_route,
};

use super::{App, Message};

const GAIN_THROTTLE_INTERVAL: Duration = Duration::from_millis(50);

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

    pub(super) fn submit_create_channel(&mut self) -> Task<Message> {
        let trimmed = self.new_channel_name.trim().to_string();
        if trimmed.is_empty() {
            return Task::none();
        }
        self.creating_channel = false;
        self.new_channel_name.clear();
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
            match client.query(ControlEvent::ListInstalledApps).await {
                Ok(ControlReply::InstalledApps { apps }) => Ok(apps),
                Ok(other) => Err(format!("Unexpected daemon reply: {other:?}")),
                Err(err) => Err(err.to_string()),
            }
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

/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! IPC mutations for application-to-channel routing rules.

use iced::Task;
use pipelink_common::{
    events::{ControlEvent, StateEvent},
    types::{AppDisplay, ChannelName},
};
use pipelink_ipc::UnixSocketClient;

use super::send_mutation;
use crate::state::Message;

pub(in crate::state) fn route_app(
    client: UnixSocketClient,
    app: AppDisplay,
    channel: ChannelName,
) -> Task<Message> {
    let target_app = app.clone();
    let target_channel = channel.clone();
    send_mutation(
        client,
        ControlEvent::RouteApp { app, channel },
        move |ev| match ev {
            StateEvent::AppRouted { app: a, channel: c } => {
                c.eq_ignore_ascii_case(&target_channel) || a.eq_ignore_ascii_case(&target_app)
            },
            StateEvent::AppRoutingFailed { app: a, .. } => a.eq_ignore_ascii_case(&target_app),
            _ => false,
        },
        |ev| match ev {
            StateEvent::AppRoutingFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

pub(in crate::state) fn unroute_app(client: UnixSocketClient, app: AppDisplay) -> Task<Message> {
    let target_app = app.clone();
    send_mutation(
        client,
        ControlEvent::UnrouteApp { app },
        move |ev| {
            matches!(
                ev,
                StateEvent::AppUnrouted { app: a } | StateEvent::AppRoutingFailed { app: a, .. }
                    if a.eq_ignore_ascii_case(&target_app)
            )
        },
        |ev| match ev {
            StateEvent::AppRoutingFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

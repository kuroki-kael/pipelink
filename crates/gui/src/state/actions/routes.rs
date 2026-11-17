/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Channel, mix, route, and output mutation IPC tasks.

use iced::Task;
use pipelink_common::{
    events::{ControlEvent, StateEvent},
    types::{ChannelName, MixName},
};
use pipelink_ipc::UnixSocketClient;

use super::send_mutation;
use crate::state::Message;

pub(in crate::state) fn create_channel(
    client: UnixSocketClient,
    name: ChannelName,
) -> Task<Message> {
    let target = name.clone();
    send_mutation(
        client,
        ControlEvent::CreateChannel { name },
        move |ev| {
            matches!(
                ev,
                StateEvent::ChannelAdded { channel }
                | StateEvent::ChannelOperationFailed { channel, .. }
                    if channel == &target
            )
        },
        |ev| match ev {
            StateEvent::ChannelOperationFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

pub(in crate::state) fn remove_channel(
    client: UnixSocketClient,
    name: ChannelName,
) -> Task<Message> {
    let target = name.clone();
    send_mutation(
        client,
        ControlEvent::RemoveChannel { name },
        move |ev| {
            matches!(
                ev,
                StateEvent::ChannelRemoved { channel }
                | StateEvent::ChannelOperationFailed { channel, .. }
                    if channel.eq_ignore_ascii_case(&target)
            )
        },
        |ev| match ev {
            StateEvent::ChannelOperationFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

pub(in crate::state) fn create_mix(client: UnixSocketClient, name: MixName) -> Task<Message> {
    let target = name.clone();
    send_mutation(
        client,
        ControlEvent::CreateMix { name },
        move |ev| {
            matches!(
                ev,
                StateEvent::MixAdded { mix } | StateEvent::MixOperationFailed { mix, .. }
                    if mix == &target
            )
        },
        |ev| match ev {
            StateEvent::MixOperationFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

pub(in crate::state) fn remove_mix(client: UnixSocketClient, name: MixName) -> Task<Message> {
    let target = name.clone();
    send_mutation(
        client,
        ControlEvent::RemoveMix { name },
        move |ev| {
            matches!(
                ev,
                StateEvent::MixRemoved { mix } | StateEvent::MixOperationFailed { mix, .. }
                    if mix.eq_ignore_ascii_case(&target)
            )
        },
        |ev| match ev {
            StateEvent::MixOperationFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

pub(in crate::state) fn select_active_mix(
    client: UnixSocketClient,
    name: MixName,
) -> Task<Message> {
    let target = name.clone();
    send_mutation(
        client,
        ControlEvent::SelectGlobalActiveLayer { mix: name },
        move |ev| {
            matches!(
                ev,
                StateEvent::GlobalLayerSwitched { mix }
                | StateEvent::LayerSelectionFailed { mix, .. }
                    if mix.eq_ignore_ascii_case(&target)
            )
        },
        |ev| match ev {
            StateEvent::LayerSelectionFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

pub(in crate::state) fn toggle_route(
    client: UnixSocketClient,
    channel: ChannelName,
    mix: MixName,
    active: bool,
) -> Task<Message> {
    let req = if active {
        ControlEvent::AddChannelMix {
            channel: channel.clone(),
            mix: mix.clone(),
        }
    } else {
        ControlEvent::RemoveChannelMix {
            channel: channel.clone(),
            mix: mix.clone(),
        }
    };
    send_mutation(
        client,
        req,
        move |ev| {
            matches!(
                ev,
                StateEvent::MixChannelAdded { channel: c, mix: m }
                | StateEvent::MixChannelRemoved { channel: c, mix: m }
                | StateEvent::RouteOperationFailed { channel: c, mix: m, .. }
                    if c.eq_ignore_ascii_case(&channel) && m.eq_ignore_ascii_case(&mix)
            )
        },
        |ev| match ev {
            StateEvent::RouteOperationFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

pub(in crate::state) fn set_gain(
    client: UnixSocketClient,
    channel: ChannelName,
    mix: MixName,
    percent: f32,
) -> Task<Message> {
    send_mutation(
        client,
        ControlEvent::SetChannelGain {
            channel: channel.clone(),
            mix: mix.clone(),
            percent,
        },
        move |ev| {
            matches!(
                ev,
                StateEvent::GainChanged { channel: c, mix: m, .. }
                | StateEvent::RouteOperationFailed { channel: c, mix: m, .. }
                    if c.eq_ignore_ascii_case(&channel) && m.eq_ignore_ascii_case(&mix)
            )
        },
        |ev| match ev {
            StateEvent::RouteOperationFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

pub(in crate::state) fn set_mute(
    client: UnixSocketClient,
    channel: ChannelName,
    mix: MixName,
    muted: bool,
) -> Task<Message> {
    send_mutation(
        client,
        ControlEvent::SetChannelMute {
            channel: channel.clone(),
            mix: mix.clone(),
            muted,
        },
        move |ev| {
            matches!(
                ev,
                StateEvent::MuteChanged { channel: c, mix: m, .. }
                | StateEvent::RouteOperationFailed { channel: c, mix: m, .. }
                    if c.eq_ignore_ascii_case(&channel) && m.eq_ignore_ascii_case(&mix)
            )
        },
        |ev| match ev {
            StateEvent::RouteOperationFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

pub(in crate::state) fn select_output(client: UnixSocketClient, output: String) -> Task<Message> {
    send_mutation(
        client,
        ControlEvent::SetOutput { output },
        |ev| {
            matches!(
                ev,
                StateEvent::OutputChanged { .. } | StateEvent::OutputOperationFailed { .. }
            )
        },
        |ev| match ev {
            StateEvent::OutputOperationFailed { error } => Err(error),
            _ => Ok(()),
        },
    )
}

pub(in crate::state) fn set_default_sink(client: UnixSocketClient, sink: String) -> Task<Message> {
    send_mutation(
        client,
        ControlEvent::SetDefaultSink { sink },
        |ev| {
            matches!(
                ev,
                StateEvent::DefaultSinkChanged { .. }
                    | StateEvent::DefaultSinkOperationFailed { .. }
            )
        },
        |ev| match ev {
            StateEvent::DefaultSinkOperationFailed { error } => Err(error),
            _ => Ok(()),
        },
    )
}

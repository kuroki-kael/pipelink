/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! IPC mutations for channel null-sink volume/mute, renaming, and reordering.

use iced::Task;
use pipelink_common::{
    events::{ControlEvent, StateEvent},
    types::ChannelName,
};
use pipelink_ipc::UnixSocketClient;

use super::send_mutation;
use crate::state::Message;

pub(in crate::state) fn set_channel_master_volume(
    client: UnixSocketClient,
    channel: ChannelName,
    percent: f32,
) -> Task<Message> {
    send_channel_mutation(
        client,
        ControlEvent::SetChannelMasterVolume {
            channel: channel.clone(),
            percent,
        },
        channel,
        |ev, target| {
            matches!(
                ev,
                StateEvent::ChannelMasterChanged { channel: c, .. }
                | StateEvent::ChannelOperationFailed { channel: c, .. }
                    if c.eq_ignore_ascii_case(target)
            )
        },
    )
}

pub(in crate::state) fn set_channel_master_mute(
    client: UnixSocketClient,
    channel: ChannelName,
    muted: bool,
) -> Task<Message> {
    send_channel_mutation(
        client,
        ControlEvent::SetChannelMasterMute {
            channel: channel.clone(),
            muted,
        },
        channel,
        |ev, target| {
            matches!(
                ev,
                StateEvent::ChannelMasterChanged { channel: c, .. }
                | StateEvent::ChannelOperationFailed { channel: c, .. }
                    if c.eq_ignore_ascii_case(target)
            )
        },
    )
}

pub(in crate::state) fn rename_channel(
    client: UnixSocketClient,
    old_name: ChannelName,
    new_name: ChannelName,
) -> Task<Message> {
    send_channel_mutation(
        client,
        ControlEvent::RenameChannel {
            old_name: old_name.clone(),
            new_name,
        },
        old_name,
        |ev, target| match ev {
            StateEvent::ChannelRenamed { old_name: o, .. } => o.eq_ignore_ascii_case(target),
            StateEvent::ChannelOperationFailed { channel: c, .. } => c.eq_ignore_ascii_case(target),
            _ => false,
        },
    )
}

pub(in crate::state) fn move_channel(
    client: UnixSocketClient,
    channel: ChannelName,
    to_index: usize,
) -> Task<Message> {
    send_channel_mutation(
        client,
        ControlEvent::MoveChannel {
            channel: channel.clone(),
            to_index,
        },
        channel,
        |ev, target| {
            matches!(ev, StateEvent::ChannelsReordered { .. })
                || matches!(ev, StateEvent::ChannelOperationFailed { channel: c, .. } if c.eq_ignore_ascii_case(target))
        },
    )
}

fn send_channel_mutation(
    client: UnixSocketClient,
    request: ControlEvent,
    channel: ChannelName,
    matcher: impl Fn(&StateEvent, &ChannelName) -> bool + Send + Sync + 'static,
) -> Task<Message> {
    send_mutation(
        client,
        request,
        move |ev| matcher(ev, &channel),
        |ev| match ev {
            StateEvent::ChannelOperationFailed { error, .. } => Err(error),
            _ => Ok(()),
        },
    )
}

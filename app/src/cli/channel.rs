/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The channel commands of the cli.

use clap::Subcommand;
use pipelink_common::{
    events::{ControlEvent, ControlReply, StateEvent},
    types::ChannelName,
};

use super::fail;

/// The channels to manage.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum ChannelAction {
    /// List all configured channels
    List,
    /// Create a new channel
    Add { name: String },
    /// Remove a channel
    Remove { name: String },
}

/// Runs a channel command against the daemon.
pub(super) async fn run(action: ChannelAction) {
    match action {
        ChannelAction::List => match crate::ipc::send_query(ControlEvent::ListChannels).await {
            Ok(ControlReply::Channels { names }) => {
                for name in names {
                    println!("{name}");
                }
            },
            Ok(other) => fail(format!("unexpected answer: {other:?}")),
            Err(e) => fail(e),
        },
        ChannelAction::Add { name } => {
            let target = ChannelName::from(name.clone());
            let request = ControlEvent::CreateChannel {
                name: target.clone(),
            };
            match crate::ipc::send_mutation(request, |event| confirms_created(event, &target)).await
            {
                Ok(StateEvent::ChannelCreationFailed { error, .. }) => {
                    fail(format!("failed to create channel '{name}': {error}"));
                },
                Ok(_) => println!("Channel '{name}' created successfully"),
                Err(e) => fail(e),
            }
        },
        ChannelAction::Remove { name } => {
            let target = ChannelName::from(name.clone());
            let request = ControlEvent::RemoveChannel {
                name: target.clone(),
            };
            match crate::ipc::send_mutation(request, |event| confirms_removed(event, &target)).await
            {
                Ok(StateEvent::ChannelRemovalFailed { error, .. }) => {
                    fail(format!("failed to remove channel '{name}': {error}"));
                },
                Ok(StateEvent::ChannelRemoved { channel }) => {
                    println!("Channel '{channel}' removed");
                },
                Ok(_) => println!("Channel '{name}' removed"),
                Err(e) => fail(e),
            }
        },
    }
}

/// Whether the state event confirms that the channel was created.
fn confirms_created(event: &StateEvent, target: &ChannelName) -> bool {
    matches!(
        event,
        StateEvent::ChannelAdded { channel } | StateEvent::ChannelCreationFailed { channel, .. }
        if channel == target
    )
}

/// Whether the state event confirms that the channel was removed.
fn confirms_removed(event: &StateEvent, target: &ChannelName) -> bool {
    matches!(
        event,
        StateEvent::ChannelRemoved { channel } | StateEvent::ChannelRemovalFailed { channel, .. }
            if channel.eq_ignore_ascii_case(target)
    )
}

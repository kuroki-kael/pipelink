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
    /// Rename an existing channel
    Rename { old_name: String, new_name: String },
    /// Remove a channel
    Remove { name: String },
}

/// Runs a channel command against the daemon.
pub(super) async fn run(action: ChannelAction) {
    match action {
        ChannelAction::List => {
            match pipelink_ipc::client::send_query(ControlEvent::ListChannels).await {
                Ok(ControlReply::Channels { names }) => {
                    for name in names {
                        println!("{name}");
                    }
                },
                Ok(other) => fail(format!("unexpected answer: {other:?}")),
                Err(e) => fail(e),
            }
        },
        ChannelAction::Add { name } => {
            let target = ChannelName::from(name.clone());
            let request = ControlEvent::CreateChannel {
                name: target.clone(),
            };
            match pipelink_ipc::client::send_mutation(request, |event| {
                confirms_created(event, &target)
            })
            .await
            {
                Ok(StateEvent::ChannelOperationFailed { error, .. }) => {
                    fail(format!("failed to create channel '{name}': {error}"));
                },
                Ok(_) => println!("Channel '{name}' created successfully"),
                Err(e) => fail(e),
            }
        },
        ChannelAction::Rename { old_name, new_name } => rename_channel(&old_name, &new_name).await,
        ChannelAction::Remove { name } => {
            let target = ChannelName::from(name.clone());
            let request = ControlEvent::RemoveChannel {
                name: target.clone(),
            };
            match pipelink_ipc::client::send_mutation(request, |event| {
                confirms_removed(event, &target)
            })
            .await
            {
                Ok(StateEvent::ChannelOperationFailed { error, .. }) => {
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
        StateEvent::ChannelAdded { channel } | StateEvent::ChannelOperationFailed { channel, .. }
            if channel.eq_ignore_ascii_case(target)
    )
}

/// Whether the state event confirms that the channel was removed.
fn confirms_removed(event: &StateEvent, target: &ChannelName) -> bool {
    matches!(
        event,
        StateEvent::ChannelRemoved { channel } | StateEvent::ChannelOperationFailed { channel, .. }
            if channel.eq_ignore_ascii_case(target)
    )
}

/// Whether the state event confirms or rejects a channel rename.
fn confirms_renamed(event: &StateEvent, target: &ChannelName) -> bool {
    matches!(
        event,
        StateEvent::ChannelRenamed { old_name, .. }
            if old_name.eq_ignore_ascii_case(target)
    ) || matches!(
        event,
        StateEvent::ChannelOperationFailed { channel, .. }
            if channel.eq_ignore_ascii_case(target)
    )
}

async fn rename_channel(old_name: &str, new_name: &str) {
    let old_target = ChannelName::from(old_name);
    let new_target = ChannelName::from(new_name);
    let request = ControlEvent::RenameChannel {
        old_name: old_target.clone(),
        new_name: new_target,
    };
    match pipelink_ipc::client::send_mutation(request, |event| confirms_renamed(event, &old_target))
        .await
    {
        Ok(StateEvent::ChannelOperationFailed { error, .. }) => {
            fail(format!(
                "failed to rename channel '{old_name}' to '{new_name}': {error}"
            ));
        },
        Ok(StateEvent::ChannelRenamed { old_name, new_name }) => {
            println!("Channel '{old_name}' renamed to '{new_name}'");
        },
        Ok(_) => println!("Channel '{old_name}' renamed to '{new_name}'"),
        Err(e) => fail(e),
    }
}

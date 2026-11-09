/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The mix commands of the cli.

use clap::Subcommand;
use pipelink_common::{
    events::{ControlEvent, ControlReply, StateEvent},
    types::MixName,
};

use super::fail;

/// The mixes to manage.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum MixAction {
    /// List all configured mixes
    List,
    /// Create a new mix
    Add { name: String },
    /// Remove a mix
    Remove { name: String },
    /// Set mix volume
    Volume { mix: String, db: f32 },
}

/// Runs a mix command against the daemon.
pub(super) async fn run(action: MixAction) {
    match action {
        MixAction::List => match crate::ipc::send_query(ControlEvent::ListMixes).await {
            Ok(ControlReply::Mixes { names }) => {
                for name in names {
                    println!("{name}");
                }
            },
            Ok(other) => fail(format!("unexpected answer: {other:?}")),
            Err(e) => fail(e),
        },
        MixAction::Add { name } => {
            let target = MixName::from(name.clone());
            let request = ControlEvent::CreateMix {
                name: target.clone(),
            };
            match crate::ipc::send_mutation(request, |event| confirms_created(event, &target)).await
            {
                Ok(StateEvent::MixCreationFailed { error, .. }) => {
                    fail(format!("failed to create mix '{name}': {error}"));
                },
                Ok(_) => println!("Mix '{name}' created successfully"),
                Err(e) => fail(e),
            }
        },
        MixAction::Remove { name } => {
            let target = MixName::from(name.clone());
            let request = ControlEvent::RemoveMix {
                name: target.clone(),
            };
            match crate::ipc::send_mutation(request, |event| confirms_removed(event, &target)).await
            {
                Ok(StateEvent::MixRemovalFailed { error, .. }) => {
                    fail(format!("failed to remove mix '{name}': {error}"));
                },
                Ok(_) => println!("Mix '{name}' removed"),
                Err(e) => fail(e),
            }
        },
        MixAction::Volume { mix, db } => {
            fail(format!(
                "setting the volume of mix '{mix}' to {db} dB is not implemented yet"
            ));
        },
    }
}

/// Whether the state event confirms that the mix was created.
fn confirms_created(event: &StateEvent, target: &MixName) -> bool {
    matches!(
        event,
        StateEvent::MixAdded { mix } | StateEvent::MixCreationFailed { mix, .. } if mix == target
    )
}

/// Whether the state event confirms that the mix was removed.
fn confirms_removed(event: &StateEvent, target: &MixName) -> bool {
    matches!(
        event,
        StateEvent::MixRemoved { mix } | StateEvent::MixRemovalFailed { mix, .. } if mix == target
    )
}

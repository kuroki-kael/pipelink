/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! The physical/virtual audio output commands of the cli.

use clap::Subcommand;
use pipelink_common::events::{ControlEvent, ControlReply, StateEvent};

use super::fail;

/// Output sink discovery and selection actions.
#[derive(Subcommand, Debug, Clone)]
pub(crate) enum OutputAction {
    /// List all available audio output sinks
    List,
    /// Select the output sink for the active mix (by node name, description, or nick)
    Set { device: String },
}

/// Runs an output command against the daemon.
pub(super) async fn run(action: OutputAction) {
    match action {
        OutputAction::List => {
            match pipelink_ipc::client::send_query(ControlEvent::ListOutputs).await {
                Ok(ControlReply::Outputs { outputs }) => {
                    if outputs.is_empty() {
                        println!("(none)");
                        return;
                    }
                    for out in outputs {
                        let marker = if out.active { "*" } else { " " };
                        let kind = if out.is_hardware { "hw" } else { "sw" };
                        println!("{marker} [{kind}] {}", format_output_display(&out));
                    }
                },
                Ok(other) => fail(format!("unexpected answer: {other:?}")),
                Err(e) => fail(e),
            }
        },
        OutputAction::Set { device } => {
            let request = ControlEvent::SetOutput {
                output: device.clone(),
            };
            let reply = pipelink_ipc::client::send_mutation(request, |event| {
                matches!(
                    event,
                    StateEvent::OutputChanged { .. } | StateEvent::OutputOperationFailed { .. }
                )
            })
            .await;

            match reply {
                Ok(StateEvent::OutputOperationFailed { error }) => {
                    fail(format!("failed to select output '{device}': {error}"));
                },
                Ok(StateEvent::OutputChanged { output }) => {
                    println!("Selected output '{output}'");
                },
                Ok(other) => fail(format!("unexpected answer: {other:?}")),
                Err(e) => fail(e),
            }
        },
    }
}

/// Formats an output device as `Label (node.name)`, or just `node.name` when equal.
pub(super) fn format_output_display(out: &pipelink_common::events::OutputDeviceInfo) -> String {
    let label = out.display_name();
    if label == &*out.name {
        out.name.to_string()
    } else {
        format!("{label} ({})", out.name)
    }
}

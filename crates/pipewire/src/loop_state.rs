/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
mod registry;

use std::{any::Any, cell::RefCell, collections::HashMap};

use pipelink_common::{
    graph::link::StereoLink,
    types::{ChannelName, GlobalId, MixName, NodeId, NodeName},
};
use pipewire::{
    context::Context, core::Core, metadata::Metadata, node::Node, registry::RegistryRc,
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, warn};

use crate::{Command, PipewireEvent, factory, module_handle::ModuleHandle};

type StreamListenerEntry = (Node, Box<dyn Any>);

#[derive(Default)]
pub(crate) struct LoopState {
    active_nodes: RefCell<HashMap<ChannelName, Node>>,
    active_modules: RefCell<HashMap<MixName, ModuleHandle>>,
    default_metadata: RefCell<Option<Metadata>>,
    stream_listeners: RefCell<HashMap<NodeId, StreamListenerEntry>>,
}

impl LoopState {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn handle_pipewire_command(
        &self,
        cmd: Command,
        core: &Core,
        context: &Context,
        registry: &RegistryRc,
        event_tx: &UnboundedSender<PipewireEvent>,
    ) {
        match cmd {
            Command::CoreSync => {
                let _ = core.sync(0);
            },
            Command::CreateChannel { name } => match factory::create_null_sink(core, &name) {
                Ok(node) => {
                    self.active_nodes.borrow_mut().insert(name, node);
                },
                Err(e) => {
                    error!("Failed to create null sink: {e:?}");
                    let _ = event_tx.send(PipewireEvent::ChannelCreationFailed {
                        name,
                        reason: e.to_string(),
                    });
                },
            },
            Command::DeleteChannel { channel } => {
                if let Some(id) = channel.node.id {
                    self.destroy_channel(&channel.name, id, registry, event_tx);
                }
            },
            Command::DestroyNode { node_id } => {
                if let Err(e) = registry.destroy_global(node_id.0).into_result() {
                    error!("Failed to destroy node {}: {e}", node_id.0);
                    let _ = event_tx.send(PipewireEvent::GlobalRemovalFailed {
                        id: GlobalId::from(node_id.0),
                        reason: e.to_string(),
                    });
                }
            },
            Command::SetAppRoute {
                node_id,
                target_serial,
                label,
            } => {
                // set target.object to target channel serial on the stream node
                if let Some(metadata) = self.default_metadata.borrow().as_ref() {
                    let serial_str = target_serial.to_string();
                    metadata.set_property(
                        node_id.0,
                        "target.object",
                        Some("Spa:Id"),
                        Some(&serial_str),
                    );
                    let _ = core.sync(0);
                    debug!(
                        "target.object set to {} for {} (node {})",
                        serial_str, label, node_id
                    );
                }
            },
            Command::ClearNodeRoute { node_id } => {
                if let Some(metadata) = self.default_metadata.borrow().as_ref() {
                    metadata.set_property(node_id.0, "target.object", None, None);
                    let _ = core.sync(0);
                    debug!("target.object cleared for {}", node_id);
                }
            },
            Command::SetDefaultSink { node_name } => {
                self.set_default_sink(core, node_name, event_tx);
            },
            Command::CreateMix { name } => match factory::create_loopback(context, &name) {
                Ok(module) => {
                    self.active_modules.borrow_mut().insert(name, module);
                },
                Err(e) => {
                    error!("Failed to create loopback: {e:?}");
                    let _ = event_tx.send(PipewireEvent::MixCreationFailed {
                        name,
                        reason: e.to_string(),
                    });
                },
            },
            Command::DeleteMix { mix } => {
                let module = self.active_modules.borrow_mut().remove(&mix.name);
                if let Some(module) = module {
                    drop(module);
                } else {
                    // Without the loopback there is nothing that could vanish,
                    // so the removal would never be confirmed.
                    error!("No loopback to destroy for mix '{}'", mix.name);
                    let _ = event_tx.send(PipewireEvent::MixRemovalFailed {
                        name: mix.name,
                        reason: "the loopback of the mix does not exist".to_string(),
                    });
                }
            },
            Command::CreateLink { stereo_link } => Self::create_stereo_link(core, stereo_link),
            Command::DestroyLink { link_id } => {
                if let Err(e) = registry.destroy_global(link_id.0).into_result() {
                    error!("Failed to destroy link {link_id}: {e}");
                } else {
                    let _ = core.sync(0);
                }
            },
            Command::SetMixRoute { .. } | Command::LoadInput { .. } => {},
        }
    }

    fn set_default_sink(
        &self,
        core: &Core,
        node_name: NodeName,
        event_tx: &UnboundedSender<PipewireEvent>,
    ) {
        if let Some(metadata) = self.default_metadata.borrow().as_ref() {
            let payload = serde_json::json!({ "name": node_name }).to_string();
            metadata.set_property(
                0,
                "default.configured.audio.sink",
                Some("Spa:String:JSON"),
                Some(&payload),
            );
            let _ = core.sync(0);
            debug!("default.configured.audio.sink set to {}", payload);
            let _ = event_tx.send(PipewireEvent::DefaultSinkSet { node_name });
        } else {
            warn!("Cannot set default sink '{node_name}': default metadata not bound");
            let _ = event_tx.send(PipewireEvent::DefaultSinkFailed {
                reason: "PipeWire 'default' metadata is not bound yet".to_string(),
            });
        }
    }

    fn create_stereo_link(core: &Core, stereo_link: StereoLink) {
        for half in [stereo_link.fl, stereo_link.fr] {
            if let Err(e) = factory::create_link(core, half) {
                error!(
                    "Failed to create link {}:{} -> {}:{}: {e:?}",
                    half.out_port.node_id,
                    half.out_port.port_id,
                    half.in_port.node_id,
                    half.in_port.port_id
                );
            }
        }
        let _ = core.sync(0);
    }

    /// Destroys the sink of a channel and reports why that did not work.
    ///
    /// The removal is only confirmed when `PipeWire` reports the node as
    /// gone. If there is nothing to destroy, or if the destruction fails,
    /// the removal is reported as failed, so that nobody waits for a
    /// confirmation that cannot arrive.
    fn destroy_channel(
        &self,
        name: &ChannelName,
        id: NodeId,
        registry: &RegistryRc,
        event_tx: &UnboundedSender<PipewireEvent>,
    ) {
        let is_active = self.active_nodes.borrow().contains_key(name);

        if is_active {
            match registry.destroy_global(id.0).into_result() {
                Ok(_) => {
                    self.active_nodes.borrow_mut().remove(name);
                },
                Err(e) => {
                    error!("Failed to destroy the sink of channel '{name}': {e}");
                    let _ = event_tx.send(PipewireEvent::GlobalRemovalFailed {
                        id: GlobalId::from(id.0),
                        reason: e.to_string(),
                    });
                },
            }
        } else {
            // The sink is not there, so nothing can vanish.
            error!("No sink to destroy for channel '{name}'");
            let _ = event_tx.send(PipewireEvent::GlobalRemovalFailed {
                id: GlobalId::from(id.0),
                reason: "the sink of the channel does not exist".to_string(),
            });
        }
    }
}

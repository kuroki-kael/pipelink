/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::{any::Any, cell::RefCell, collections::HashMap};

use pipelink_common::types::{
    ChannelName, ChannelPosition, GlobalId, MediaClass, MixName, NodeId, NodeName, NodeSerial, Pid,
    PortDirection, PortId,
};
use pipewire::{
    context::Context,
    core::Core,
    metadata::Metadata,
    node::Node,
    registry::{GlobalObject, RegistryRc},
    spa::utils::dict::DictRef,
    types::ObjectType,
};
use tokio::sync::mpsc::Sender;
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
        event_tx: &Sender<PipewireEvent>,
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
                    let _ = event_tx.blocking_send(PipewireEvent::ChannelCreationFailed {
                        name,
                        reason: e.to_string(),
                    });
                },
            },
            Command::DeleteChannel { channel } => {
                if let Some(id) = channel.node.id {
                    let _ = registry.destroy_global(id.0);
                    self.active_nodes.borrow_mut().remove(&channel.name);
                }
            },
            Command::DestroyNode { node_id } => {
                let _ = registry.destroy_global(node_id.0);
            },
            Command::SetAppRoute { app, channel } => {
                // set target.object to target channel serial on the stream node
                if let Some(channel_serial) = channel.node.serial
                    && let Some(node_id) = app.node.id
                    && let Some(metadata) = self.default_metadata.borrow().as_ref()
                {
                    let serial_str = channel_serial.to_string();
                    metadata.set_property(
                        node_id.0,
                        "target.object",
                        Some("Spa:Id"),
                        Some(&serial_str),
                    );
                    let _ = core.sync(0);
                    debug!(
                        "target.object set to {} for {} (node {})",
                        serial_str, app.app.app_name, node_id
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
            Command::CreateMix { name } => match factory::create_loopback(context, &name) {
                Ok(module) => {
                    self.active_modules.borrow_mut().insert(name, module);
                },
                Err(e) => {
                    error!("Failed to create loopback: {e:?}");
                    let _ = event_tx.blocking_send(PipewireEvent::MixCreationFailed {
                        name,
                        reason: e.to_string(),
                    });
                },
            },
            Command::DeleteMix { mix } => {
                let module = self.active_modules.borrow_mut().remove(&mix.name);
                drop(module);
            },
            Command::CreateLink { .. }
            | Command::DestroyLink { .. }
            | Command::SetMixRoute { .. }
            | Command::LoadInput { .. } => {},
        }
    }

    pub(crate) fn handle_pipewire_event(
        &self,
        obj: &GlobalObject<&DictRef>,
        event_tx: &Sender<PipewireEvent>,
        registry: &RegistryRc,
    ) {
        let Some(dict) = &obj.props else { return };

        match obj.type_ {
            ObjectType::Node => {
                let media_class =
                    MediaClass::from_pw_str(dict.get("media.class").unwrap_or_default());
                let id = NodeId::from(obj.id);
                let serial = dict
                    .get("object.serial")
                    .and_then(|s| s.parse().ok())
                    .map(NodeSerial);
                let pid = dict
                    .get("application.process.id")
                    .and_then(|p| p.parse().ok())
                    .map(Pid);

                let is_pipelink_node = dict
                    .get("node.name")
                    .is_some_and(|n| n.starts_with("pipelink."));

                if let Some(name) = dict.get("node.name") {
                    let _ = event_tx.blocking_send(PipewireEvent::NodeAppeared {
                        id,
                        name: NodeName::from(name),
                        media_class: media_class.clone(),
                        app: None,
                        pid,
                        serial,
                        nick: dict.get("node.nick").map(ToString::to_string),
                    });
                }

                if !is_pipelink_node
                    && media_class.is_stream_output()
                    && let Ok(node_proxy) = registry.bind::<Node, _>(obj)
                {
                    let tx = event_tx.clone();

                    let listener = node_proxy
                        .add_listener_local()
                        .info(move |info| {
                            if let Some(props) = info.props() {
                                let _ = tx.blocking_send(PipewireEvent::NodeInfo {
                                    id,
                                    node_name: props.get("node.name").map(NodeName::from),
                                    app: props.get("application.name").map(ToString::to_string),
                                    pid: props
                                        .get("application.process.id")
                                        .and_then(|p| p.parse().ok())
                                        .map(Pid),
                                    nick: props.get("node.nick").map(ToString::to_string),
                                });
                            }
                        })
                        .register();

                    self.stream_listeners
                        .borrow_mut()
                        .insert(id, (node_proxy, Box::new(listener)));
                }
            },
            ObjectType::Port => {
                if let (Some(node_id_str), Some(direction), Some(channel)) = (
                    dict.get("node.id"),
                    dict.get("port.direction"),
                    dict.get("audio.channel"),
                ) && let Ok(node_id) = node_id_str.parse::<NodeId>()
                {
                    let _ = event_tx.blocking_send(PipewireEvent::PortAppeared {
                        id: PortId::from(obj.id),
                        node_id,
                        direction: PortDirection::from_pw_str(direction),
                        channel: ChannelPosition::from_pw_str(channel),
                    });
                }
            },
            ObjectType::Metadata => {
                let is_default = dict.get("metadata.name") == Some("default");
                if is_default {
                    match registry.bind::<Metadata, _>(obj) {
                        Ok(metadata) => {
                            debug!("Default metadata bound");
                            *self.default_metadata.borrow_mut() = Some(metadata);
                            let _ = event_tx.blocking_send(PipewireEvent::MetadataAppeared {
                                id: GlobalId::from(obj.id),
                            });
                        },
                        Err(e) => {
                            warn!("Failed to load metadata: {e:?}");
                        },
                    }
                }
            },
            _ => {},
        }
    }

    pub(crate) fn global_removed(&self, id: GlobalId, event_tx: &Sender<PipewireEvent>) {
        self.stream_listeners.borrow_mut().remove(&id.0);
        let _ = event_tx.blocking_send(PipewireEvent::GlobalRemoved { id });
    }
}

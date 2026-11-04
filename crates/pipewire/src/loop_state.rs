/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::{any::Any, cell::RefCell, collections::HashMap};

use pipelink_common::graph::node::NativeNode;
use pipewire::{
    core::Core,
    metadata::Metadata,
    node::Node,
    registry::{GlobalObject, RegistryRc},
    spa::utils::dict::DictRef,
    types::ObjectType,
};
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, warn};

use crate::{Command, PipewireEvent, factory};

type StreamListenerEntry = (Node, Box<dyn Any>);

#[derive(Default)]
pub(crate) struct LoopState {
    active_nodes: RefCell<HashMap<String, Node>>,
    default_metadata: RefCell<Option<Metadata>>,
    stream_listeners: RefCell<HashMap<u32, StreamListenerEntry>>,
}

impl LoopState {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn handle_pipewire_command(&self, cmd: Command, core: &Core, registry: &RegistryRc) {
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
                },
            },
            Command::DeleteChannel { channel } => {
                if let Some(id) = channel.get_id() {
                    let _ = registry.destroy_global(id);
                    self.active_nodes.borrow_mut().remove(channel.get_name());
                }
            },
            Command::DestroyNode { node_id } => {
                let _ = registry.destroy_global(node_id);
            },
            Command::SetAppRoute { app, channel } => {
                // set target.object to target channel serial on the stream node
                if let Some(channel_serial) = channel.get_node().get_serial()
                    && let Some(node_id) = app.node.get_id()
                    && let Some(metadata) = self.default_metadata.borrow().as_ref()
                {
                    let serial_str = channel_serial.to_string();
                    metadata.set_property(
                        node_id,
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
                    metadata.set_property(node_id, "target.object", None, None);
                    let _ = core.sync(0);
                    debug!("target.object cleared for {}", node_id);
                }
            },
            Command::CreateLink { .. }
            | Command::DestroyLink { .. }
            | Command::SetMixRoute { .. }
            | Command::CreateMix { .. }
            | Command::LoadInput { .. }
            | Command::DeleteMix { .. } => {},
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
                let media_class = dict.get("media.class").unwrap_or_default();
                let id = obj.id;

                if let Some(name) = dict.get("node.name") {
                    let _ = event_tx.blocking_send(PipewireEvent::NodeAppeared {
                        id,
                        name: name.to_string(),
                        media_class: dict.get("media.class").unwrap_or_default().to_string(),
                        app: None,
                        pid: None,
                        serial: dict.get("object.serial").and_then(|s| s.parse().ok()),
                        nick: dict.get("node.nick").map(ToString::to_string),
                    });
                }

                if media_class.starts_with("Stream/Output/")
                    && let Ok(node_proxy) = registry.bind::<Node, _>(obj)
                {
                    let tx = event_tx.clone();

                    let listener = node_proxy
                        .add_listener_local()
                        .info(move |info| {
                            if let Some(props) = info.props() {
                                let _ = tx.blocking_send(PipewireEvent::NodeInfo {
                                    id,
                                    app: props.get("application.name").map(ToString::to_string),
                                    pid: props
                                        .get("application.process.id")
                                        .and_then(|p| p.parse().ok()),
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
                ) && let Ok(node_id) = node_id_str.parse::<u32>()
                {
                    let _ = event_tx.blocking_send(PipewireEvent::PortAppeared {
                        id: obj.id,
                        node_id,
                        direction: if direction == "in" {
                            "Input".into()
                        } else {
                            "Output".into()
                        },
                        channel: channel.to_string(),
                        node_name: dict.get("node.name").unwrap_or_default().to_string(),
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
                            let _ = event_tx
                                .blocking_send(PipewireEvent::MetadataAppeared { id: obj.id });
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

    pub(crate) fn global_removed(&self, id: u32, event_tx: &Sender<PipewireEvent>) {
        self.stream_listeners.borrow_mut().remove(&id);
        let _ = event_tx.blocking_send(PipewireEvent::GlobalRemoved { id });
    }
}

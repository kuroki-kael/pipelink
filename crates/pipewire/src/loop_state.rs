/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use std::{any::Any, cell::RefCell, collections::HashMap};

use pipelink_common::{
    JACK_CLIENT_NAME,
    graph::link::StereoLink,
    types::{
        ChannelName, ChannelPosition, GlobalId, LinkId, MediaClass, MixName, NodeId, NodeName,
        NodeSerial, Pid, PortDirection, PortId,
    },
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
                    self.destroy_channel(&channel.name, id, registry, event_tx);
                }
            },
            Command::DestroyNode { node_id } => {
                if let Err(e) = registry.destroy_global(node_id.0).into_result() {
                    error!("Failed to destroy node {}: {e}", node_id.0);
                    let _ = event_tx.blocking_send(PipewireEvent::GlobalRemovalFailed {
                        id: GlobalId::from(node_id.0),
                        reason: e.to_string(),
                    });
                }
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
                if let Some(module) = module {
                    drop(module);
                } else {
                    // Without the loopback there is nothing that could vanish,
                    // so the removal would never be confirmed.
                    error!("No loopback to destroy for mix '{}'", mix.name);
                    let _ = event_tx.blocking_send(PipewireEvent::MixRemovalFailed {
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
        event_tx: &Sender<PipewireEvent>,
    ) {
        let is_active = self.active_nodes.borrow().contains_key(name);

        if is_active {
            match registry.destroy_global(id.0).into_result() {
                Ok(_) => {
                    self.active_nodes.borrow_mut().remove(name);
                },
                Err(e) => {
                    error!("Failed to destroy the sink of channel '{name}': {e}");
                    let _ = event_tx.blocking_send(PipewireEvent::GlobalRemovalFailed {
                        id: GlobalId::from(id.0),
                        reason: e.to_string(),
                    });
                },
            }
        } else {
            // The sink is not there, so nothing can vanish.
            error!("No sink to destroy for channel '{name}'");
            let _ = event_tx.blocking_send(PipewireEvent::GlobalRemovalFailed {
                id: GlobalId::from(id.0),
                reason: "the sink of the channel does not exist".to_string(),
            });
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
            ObjectType::Node => self.handle_node_appeared(obj, dict, event_tx, registry),
            ObjectType::Port => Self::handle_port_appeared(obj.id, dict, event_tx),
            ObjectType::Link => Self::handle_link_appeared(obj.id, dict, event_tx),
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

    fn handle_node_appeared(
        &self,
        obj: &GlobalObject<&DictRef>,
        dict: &DictRef,
        event_tx: &Sender<PipewireEvent>,
        registry: &RegistryRc,
    ) {
        let media_class = MediaClass::from_pw_str(dict.get("media.class").unwrap_or_default());
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
            .is_some_and(|n| n.starts_with("pipelink.") || n == JACK_CLIENT_NAME);

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
    }

    fn handle_port_appeared(obj_id: u32, dict: &DictRef, event_tx: &Sender<PipewireEvent>) {
        if let (Some(node_id_str), Some(direction)) =
            (dict.get("node.id"), dict.get("port.direction"))
            && let Ok(node_id) = node_id_str.parse::<NodeId>()
        {
            let channel = dict
                .get("audio.channel")
                .map_or(ChannelPosition::Other, ChannelPosition::from_pw_str);
            let _ = event_tx.blocking_send(PipewireEvent::PortAppeared {
                id: PortId::from(obj_id),
                node_id,
                name: dict.get("port.name").map(ToString::to_string),
                direction: PortDirection::from_pw_str(direction),
                channel,
            });
        }
    }

    fn handle_link_appeared(obj_id: u32, dict: &DictRef, event_tx: &Sender<PipewireEvent>) {
        if let (Some(out_node), Some(in_node), Some(out_port), Some(in_port)) = (
            dict.get("link.output.node")
                .and_then(|s| s.parse::<NodeId>().ok()),
            dict.get("link.input.node")
                .and_then(|s| s.parse::<NodeId>().ok()),
            dict.get("link.output.port")
                .and_then(|s| s.parse::<PortId>().ok()),
            dict.get("link.input.port")
                .and_then(|s| s.parse::<PortId>().ok()),
        ) {
            let _ = event_tx.blocking_send(PipewireEvent::LinkAppeared {
                id: LinkId::from(obj_id),
                out_node,
                in_node,
                out_port,
                in_port,
            });
        }
    }

    pub(crate) fn global_removed(&self, id: GlobalId, event_tx: &Sender<PipewireEvent>) {
        self.stream_listeners.borrow_mut().remove(&id.0);
        let _ = event_tx.blocking_send(PipewireEvent::GlobalRemoved { id });
    }
}

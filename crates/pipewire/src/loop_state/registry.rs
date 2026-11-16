/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use pipelink_common::{
    JACK_CLIENT_NAME,
    types::{
        ChannelPosition, GlobalId, LinkId, MediaClass, NodeId, NodeName, NodeSerial, Pid,
        PortDirection, PortId,
    },
};
use pipewire::{
    metadata::Metadata,
    node::Node,
    registry::{GlobalObject, RegistryRc},
    spa::utils::dict::DictRef,
    types::ObjectType,
};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

use super::LoopState;
use crate::PipewireEvent;

impl LoopState {
    pub(crate) fn handle_pipewire_event(
        &self,
        obj: &GlobalObject<&DictRef>,
        event_tx: &UnboundedSender<PipewireEvent>,
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
                            let _ = event_tx.send(PipewireEvent::MetadataAppeared {
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
        event_tx: &UnboundedSender<PipewireEvent>,
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
            let _ = event_tx.send(PipewireEvent::NodeAppeared {
                id,
                name: NodeName::from(name),
                media_class: media_class.clone(),
                app: None,
                pid,
                serial,
                description: dict.get("node.description").map(ToString::to_string),
                nick: dict.get("node.nick").map(ToString::to_string),
                is_hardware: dict.get("device.id").is_some(),
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
                        let _ = tx.send(PipewireEvent::NodeInfo {
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

    fn handle_port_appeared(
        obj_id: u32,
        dict: &DictRef,
        event_tx: &UnboundedSender<PipewireEvent>,
    ) {
        if let (Some(node_id_str), Some(direction)) =
            (dict.get("node.id"), dict.get("port.direction"))
            && let Ok(node_id) = node_id_str.parse::<NodeId>()
        {
            let channel = dict
                .get("audio.channel")
                .map_or(ChannelPosition::Other, ChannelPosition::from_pw_str);
            let _ = event_tx.send(PipewireEvent::PortAppeared {
                id: PortId::from(obj_id),
                node_id,
                name: dict.get("port.name").map(ToString::to_string),
                direction: PortDirection::from_pw_str(direction),
                channel,
            });
        }
    }

    fn handle_link_appeared(
        obj_id: u32,
        dict: &DictRef,
        event_tx: &UnboundedSender<PipewireEvent>,
    ) {
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
            let _ = event_tx.send(PipewireEvent::LinkAppeared {
                id: LinkId::from(obj_id),
                out_node,
                in_node,
                out_port,
                in_port,
            });
        }
    }

    pub(crate) fn global_removed(&self, id: GlobalId, event_tx: &UnboundedSender<PipewireEvent>) {
        self.stream_listeners.borrow_mut().remove(&id.0);
        let _ = event_tx.send(PipewireEvent::GlobalRemoved { id });
    }
}

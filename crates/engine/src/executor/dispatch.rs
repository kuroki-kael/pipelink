/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Event dispatch routing incoming control and `PipeWire` messages to executor handlers.

use pipelink_common::{
    error::ExecutorError,
    events::{ControlEvent, StateEvent},
};
use pipelink_pw::PipewireEvent;

use crate::executor::Executor;

impl Executor {
    pub(super) fn handle_control_event(&mut self, ctrl: ControlEvent) -> Result<(), ExecutorError> {
        match ctrl {
            ControlEvent::CreateChannel { name } => self.on_create_channel(name)?,
            ControlEvent::RemoveChannel { name } => self.on_remove_channel(name)?,
            ControlEvent::CreateMix { name } => self.on_create_mix(name)?,
            ControlEvent::RemoveMix { name } => self.on_remove_mix(name)?,
            ControlEvent::AddChannelMix { channel, mix } => {
                self.on_add_channel_mix(channel, mix)?;
            },
            ControlEvent::RemoveChannelMix { channel, mix } => {
                self.on_remove_channel_mix(channel, mix)?;
            },
            ControlEvent::SetChannelGain {
                channel,
                mix,
                percent,
            } => {
                self.on_set_channel_gain(channel, mix, percent)?;
            },
            ControlEvent::SetChannelMute {
                channel,
                mix,
                muted,
            } => {
                self.on_set_channel_mute(channel, mix, muted)?;
            },
            ControlEvent::SetChannelMasterVolume { channel, percent } => {
                self.on_set_channel_master_volume(&channel, percent);
            },
            ControlEvent::SetChannelMasterMute { channel, muted } => {
                self.on_set_channel_master_mute(&channel, muted);
            },
            ControlEvent::RenameChannel { old_name, new_name } => {
                self.on_rename_channel(&old_name, &new_name)?;
            },
            ControlEvent::RenameMix { old_name, new_name } => {
                self.on_rename_mix(&old_name, &new_name)?;
            },
            ControlEvent::MoveChannel { channel, to_index } => {
                self.on_move_channel(&channel, to_index)?;
            },
            ControlEvent::RouteApp { app, channel } => {
                self.on_route_app(app, &channel);
            },
            ControlEvent::UnrouteApp { app } => {
                self.on_unroute_app(app);
            },
            ControlEvent::SelectGlobalActiveLayer { mix } => {
                self.on_select_active_mix(mix);
            },
            ControlEvent::SetOutput { output } => {
                self.on_set_output(&output);
            },
            ControlEvent::SetDefaultSink { sink } => {
                self.on_set_default_sink(&sink);
            },
            ControlEvent::SelectChannelActiveLayer { .. } | _ => {},
        }

        Ok(())
    }

    pub(super) fn handle_pipewire_event(&mut self, event: PipewireEvent) {
        match event {
            PipewireEvent::NodeAppeared {
                id,
                name,
                media_class,
                serial,
                description,
                nick,
                is_hardware,
                ..
            } => {
                self.on_node_appeared(
                    id,
                    &name,
                    &media_class,
                    serial,
                    description,
                    nick,
                    is_hardware,
                );
            },
            PipewireEvent::MetadataAppeared { .. } => {
                self.on_metadata_appeared();
            },
            PipewireEvent::PortAppeared {
                id,
                node_id,
                name,
                direction,
                channel,
            } => {
                self.on_port_appeared(id, node_id, name.as_deref(), direction, channel);
            },
            PipewireEvent::LinkAppeared {
                id,
                out_node,
                in_node,
                out_port,
                in_port,
            } => {
                self.on_link_appeared(id, out_node, in_node, out_port, in_port);
            },
            PipewireEvent::LinkRemoved { id } => {
                self.on_link_removed(id);
            },
            PipewireEvent::NodeInfo {
                id, node_name, pid, ..
            } => {
                self.on_node_info(id, node_name, pid);
            },
            PipewireEvent::GlobalRemoved { id } => {
                self.on_global_removed(id);
            },
            PipewireEvent::GlobalRemovalFailed { id, reason } => {
                self.on_global_removal_failed(id, &reason);
            },
            PipewireEvent::ChannelCreationFailed { name, reason } => {
                self.on_channel_creation_failed(name, &reason);
            },
            PipewireEvent::MixCreationFailed { name, reason } => {
                self.on_mix_creation_failed(name, &reason);
            },
            PipewireEvent::MixRemovalFailed { name, reason } => {
                self.on_mix_removal_failed(name, &reason);
            },
            PipewireEvent::DefaultSinkSet { node_name } => {
                self.graph.default_sink = Some(node_name.clone());
                let _ = self
                    .state_tx
                    .send(StateEvent::DefaultSinkChanged { sink: node_name });
            },
            PipewireEvent::DefaultSinkFailed { reason } => {
                let _ = self
                    .state_tx
                    .send(StateEvent::DefaultSinkOperationFailed { error: reason });
            },
            PipewireEvent::ChannelPropsChanged {
                node_id,
                linear_volume,
                muted,
            } => {
                self.on_channel_props_changed(node_id, linear_volume, muted);
            },
            _ => {},
        }
    }
}

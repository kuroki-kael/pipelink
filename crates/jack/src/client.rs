/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Control-thread handle managing the active JACK client and port lifecycle.

use jack::{
    AsyncClient, Client, ClientOptions, ClientStatus, Control, Frames, NotificationHandler, PortId,
};
use pipelink_common::{
    JACK_CLIENT_NAME,
    types::{ChannelName, MixName},
};
use rtrb::{Consumer, Producer, PushError, RingBuffer};
use tracing::{debug, info, warn};

use crate::{
    Command, JackError, RetiredResource,
    ports::{ChannelPorts, MixPorts},
    process::JackProcess,
};

const COMMAND_RING_CAPACITY: usize = 256;
const RETIRED_RING_CAPACITY: usize = 1024;

/// Control-side handle for the active real-time JACK mixer client.
///
/// Maintains ordered channel and mix name lists mirroring the audio-thread
/// port tables so callers can address routes by name without risking index
/// mismatches.
#[derive(Debug)]
pub struct JackClient {
    client: AsyncClient<Notification, JackProcess>,
    channel_names: Vec<ChannelName>,
    mix_names: Vec<MixName>,
    cmd_tx: Producer<Command>,
    retired_rx: Consumer<RetiredResource>,
    pending_reclaims: usize,
}

impl JackClient {
    /// Opens and activates a new JACK client named [`JACK_CLIENT_NAME`].
    ///
    /// # Errors
    /// Returns [`JackError::Jack`] if no JACK/PipeWire-JACK server is available
    /// or activation fails.
    pub fn start() -> Result<Self, JackError> {
        let (client, _status) = Client::new(JACK_CLIENT_NAME, ClientOptions::NO_START_SERVER)?;
        let (cmd_tx, cmd_rx) = RingBuffer::new(COMMAND_RING_CAPACITY);
        let (retired_tx, retired_rx) = RingBuffer::new(RETIRED_RING_CAPACITY);

        let process = JackProcess::new(cmd_rx, retired_tx);
        let async_client = client.activate_async(Notification, process)?;

        Ok(Self {
            client: async_client,
            channel_names: Vec::new(),
            mix_names: Vec::new(),
            cmd_tx,
            retired_rx,
            pending_reclaims: 0,
        })
    }

    /// Returns the actual client name assigned by the JACK server.
    pub fn client_name(&self) -> &str {
        self.client.as_client().name()
    }

    /// Returns the number of retired port sets awaiting cleanup from the audio thread.
    #[must_use]
    pub const fn pending_reclaims(&self) -> usize {
        self.pending_reclaims
    }

    /// Unregisters any ports and frees any gain buffers retired by the audio thread.
    pub fn reclaim(&mut self) {
        while let Ok(resource) = self.retired_rx.pop() {
            match resource {
                RetiredResource::Channel(ports) => {
                    Self::unregister_channel_ports(self.client.as_client(), ports);
                    self.pending_reclaims = self.pending_reclaims.saturating_sub(1);
                },
                RetiredResource::Mix(ports) => {
                    Self::unregister_mix_ports(self.client.as_client(), ports);
                    self.pending_reclaims = self.pending_reclaims.saturating_sub(1);
                },
                RetiredResource::Gains(old_gains) => {
                    drop(old_gains);
                },
            }
        }
    }

    /// Registers stereo input ports for a new channel and installs the updated gains
    /// matrix.
    ///
    /// # Errors
    /// Returns [`JackError::ChannelExists`] if `name` is already registered,
    /// [`JackError::Jack`] if port registration fails, [`JackError::ChannelClosed`]
    /// if the audio thread stopped, or [`JackError::ChannelFull`] if the command
    /// ringbuffer is full.
    pub fn create_channel(&mut self, name: &ChannelName, gains: Vec<f32>) -> Result<(), JackError> {
        self.reclaim();

        if self.channel_names.iter().any(|c| c == name) {
            return Err(JackError::ChannelExists(name.clone()));
        }

        let ports = ChannelPorts::register(self.client.as_client(), name)?;
        self.send_with_rollback(Command::CreateChannel { ports, gains }, |client, cmd| {
            if let Command::CreateChannel { ports, .. } = cmd {
                Self::unregister_channel_ports(client, ports);
            }
        })?;

        self.channel_names.push(name.clone());
        Ok(())
    }

    /// Removes the channel `name` from the audio thread, installs `gains`, and schedules
    /// port cleanup.
    ///
    /// # Errors
    /// Returns [`JackError::ChannelNotFound`] if `name` is not registered,
    /// [`JackError::ChannelClosed`] if the audio thread stopped, or
    /// [`JackError::ChannelFull`] if the command ringbuffer is full.
    pub fn remove_channel(&mut self, name: &ChannelName, gains: Vec<f32>) -> Result<(), JackError> {
        self.reclaim();

        let idx = self
            .channel_names
            .iter()
            .position(|c| c == name)
            .ok_or_else(|| JackError::ChannelNotFound(name.clone()))?;

        self.send_command(Command::RemoveChannel { idx, gains })?;
        self.channel_names.remove(idx);
        self.pending_reclaims += 1;
        Ok(())
    }

    /// Registers stereo output ports for a new mix and installs the updated gains matrix.
    ///
    /// # Errors
    /// Returns [`JackError::MixExists`] if `name` is already registered,
    /// [`JackError::Jack`] if port registration fails, [`JackError::ChannelClosed`]
    /// if the audio thread stopped, or [`JackError::ChannelFull`] if the command
    /// ringbuffer is full.
    pub fn create_mix(&mut self, name: &MixName, gains: Vec<f32>) -> Result<(), JackError> {
        self.reclaim();

        if self.mix_names.iter().any(|m| m == name) {
            return Err(JackError::MixExists(name.clone()));
        }

        let ports = MixPorts::register(self.client.as_client(), name)?;
        self.send_with_rollback(Command::CreateMix { ports, gains }, |client, cmd| {
            if let Command::CreateMix { ports, .. } = cmd {
                Self::unregister_mix_ports(client, ports);
            }
        })?;

        self.mix_names.push(name.clone());
        Ok(())
    }

    /// Removes the mix `name` from the audio thread, installs `gains`, and schedules port
    /// cleanup.
    ///
    /// # Errors
    /// Returns [`JackError::MixNotFound`] if `name` is not registered,
    /// [`JackError::ChannelClosed`] if the audio thread stopped, or
    /// [`JackError::ChannelFull`] if the command ringbuffer is full.
    pub fn remove_mix(&mut self, name: &MixName, gains: Vec<f32>) -> Result<(), JackError> {
        self.reclaim();

        let idx = self
            .mix_names
            .iter()
            .position(|m| m == name)
            .ok_or_else(|| JackError::MixNotFound(name.clone()))?;

        self.send_command(Command::RemoveMix { idx, gains })?;
        self.mix_names.remove(idx);
        self.pending_reclaims += 1;
        Ok(())
    }

    /// Pushes a pre-built row-major (`channels × mixes`) gains matrix to the audio
    /// thread.
    ///
    /// # Errors
    /// Returns [`JackError::ChannelClosed`] if the audio thread stopped or
    /// [`JackError::ChannelFull`] if the command ringbuffer is full.
    pub fn sync_gains(&mut self, gains: Vec<f32>) -> Result<(), JackError> {
        self.reclaim();
        self.send_command(Command::SetGains(gains))
    }

    /// Deactivates the JACK client and unregisters all remaining ports cleanly.
    ///
    /// # Errors
    /// Returns [`JackError::Jack`] if deactivating the JACK client fails.
    pub fn shutdown(mut self) -> Result<(), JackError> {
        self.reclaim();
        let (client, _, mut process) = self.client.deactivate()?;

        while let Ok(resource) = self.retired_rx.pop() {
            Self::cleanup_retired(&client, resource);
        }

        let (channels, mixes) = process.drain_all_ports();
        for ch in channels {
            Self::unregister_channel_ports(&client, ch);
        }
        for mix in mixes {
            Self::unregister_mix_ports(&client, mix);
        }

        Ok(())
    }

    fn send_command(&mut self, cmd: Command) -> Result<(), JackError> {
        self.send_with_rollback(cmd, |_, _| {})
    }

    fn send_with_rollback<F>(&mut self, cmd: Command, rollback: F) -> Result<(), JackError>
    where
        F: FnOnce(&Client, Command),
    {
        if self.cmd_tx.is_abandoned() {
            rollback(self.client.as_client(), cmd);
            return Err(JackError::ChannelClosed);
        }

        self.cmd_tx.push(cmd).map_err(|PushError::Full(cmd)| {
            rollback(self.client.as_client(), cmd);
            JackError::ChannelFull
        })
    }

    fn cleanup_retired(client: &Client, resource: RetiredResource) {
        match resource {
            RetiredResource::Channel(ports) => Self::unregister_channel_ports(client, ports),
            RetiredResource::Mix(ports) => Self::unregister_mix_ports(client, ports),
            RetiredResource::Gains(old_gains) => drop(old_gains),
        }
    }

    fn unregister_channel_ports(client: &Client, ports: ChannelPorts) {
        let _ = client.unregister_port(ports.in_l);
        let _ = client.unregister_port(ports.in_r);
    }

    fn unregister_mix_ports(client: &Client, ports: MixPorts) {
        let _ = client.unregister_port(ports.out_l);
        let _ = client.unregister_port(ports.out_r);
    }
}

struct Notification;

impl NotificationHandler for Notification {
    fn thread_init(&self, _: &Client) {
        debug!("JACK audio thread initialized");
    }

    #[expect(
        unsafe_code,
        reason = "NotificationHandler::shutdown is marked unsafe by the jack crate"
    )]
    unsafe fn shutdown(&mut self, status: ClientStatus, reason: &str) {
        warn!("JACK client shut down ({status:?}): {reason}");
    }

    fn freewheel(&mut self, _: &Client, is_freewheel_enabled: bool) {
        debug!(
            "JACK freewheel mode {}",
            if is_freewheel_enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    fn sample_rate(&mut self, _: &Client, srate: Frames) -> Control {
        info!("JACK sample rate changed to {srate} Hz");
        Control::Continue
    }

    fn client_registration(&mut self, _: &Client, name: &str, is_registered: bool) {
        debug!(
            "JACK client '{name}' {}",
            if is_registered {
                "registered"
            } else {
                "unregistered"
            }
        );
    }

    fn port_registration(&mut self, _: &Client, port_id: PortId, is_registered: bool) {
        debug!(
            "JACK port {port_id} {}",
            if is_registered {
                "registered"
            } else {
                "unregistered"
            }
        );
    }

    fn port_rename(
        &mut self,
        _: &Client,
        port_id: PortId,
        old_name: &str,
        new_name: &str,
    ) -> Control {
        debug!("JACK port {port_id} renamed: '{old_name}' -> '{new_name}'");
        Control::Continue
    }

    fn xrun(&mut self, _: &Client) -> Control {
        warn!("JACK xrun occurred");
        Control::Continue
    }
}

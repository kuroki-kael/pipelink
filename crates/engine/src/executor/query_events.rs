/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use pipelink_common::events::{ControlEvent, ControlReply};

use crate::executor::{Envelope, Executor};

impl Executor {
    /// Answers a query from the graph and returns the event if it was no query.
    pub(super) fn handle_query(&mut self, envelope: Envelope) -> Option<ControlEvent> {
        let Envelope { event, reply } = envelope;
        match event {
            ControlEvent::ListChannels => {
                Self::reply(
                    reply,
                    ControlReply::Channels {
                        names: self
                            .graph
                            .channels()
                            .iter()
                            .map(|c| c.name.clone())
                            .collect(),
                    },
                );
                None
            },
            ControlEvent::ListMixes => {
                Self::reply(
                    reply,
                    ControlReply::Mixes {
                        names: self.graph.mixes().iter().map(|m| m.name.clone()).collect(),
                    },
                );
                None
            },
            // The remaining queries report on data models that do not exist yet.
            // TODO: answer ListAppChannels and ListMixChannels once the graph
            //   knows which app is routed to which channel.
            // TODO: answer GetActiveMix once a mix can be selected as active.
            other => Some(other),
        }
    }
}

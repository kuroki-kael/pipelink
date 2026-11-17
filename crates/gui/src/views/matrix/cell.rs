/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Single `(channel, mix)` routing cell inside the mixer matrix.

use iced::{
    Alignment, Element, Length,
    widget::{button, container, row, slider, text},
};
use pipelink_common::types::{ChannelName, MixName};

use crate::{
    state::{App, Message},
    theme,
};

impl App {
    pub(super) fn view_matrix_cell<'a>(
        &'a self,
        channel: &'a ChannelName,
        mix: &'a MixName,
        col_width: f32,
    ) -> Element<'a, Message> {
        let route = self.find_route(channel, mix);
        let is_active = route.is_some_and(|r| r.active);

        if !is_active {
            return container(
                button(
                    text("+ Route")
                        .size(12)
                        .color(theme::TEXT_MUTED)
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Center),
                )
                .on_press(Message::ToggleRoute {
                    channel: channel.clone(),
                    mix: mix.clone(),
                    active: true,
                })
                .width(Length::Fill)
                .padding([12, 10])
                .style(theme::nav_button(false)),
            )
            .width(col_width)
            .padding(4)
            .style(theme::inactive_cell_style)
            .into();
        }

        let (gain_percent, muted) = route.map_or((100.0, false), |r| (r.gain_percent, r.muted));
        let mute_label = if muted { "Muted" } else { "Mute" };

        let mute_btn = button(text(mute_label).size(11))
            .on_press(Message::ToggleMute {
                channel: channel.clone(),
                mix: mix.clone(),
                muted: !muted,
            })
            .padding([4, 8])
            .style(theme::mute_button(muted));

        let ch_change = channel.clone();
        let mix_change = mix.clone();
        let gain_slider = slider(0.0_f32..=200.0_f32, gain_percent, move |percent| {
            Message::GainChanged {
                channel: ch_change.clone(),
                mix: mix_change.clone(),
                percent,
            }
        })
        .step(1.0_f32)
        .on_release(Message::GainReleased {
            channel: channel.clone(),
            mix: mix.clone(),
        })
        .style(theme::accent_slider)
        .width(Length::Fill);

        let disconnect_btn = button(text("x").size(11))
            .on_press(Message::ToggleRoute {
                channel: channel.clone(),
                mix: mix.clone(),
                active: false,
            })
            .padding([4, 7])
            .style(theme::subtle_button);

        let gain_label = text(format!("{gain_percent:.0}%"))
            .size(11)
            .color(theme::TEXT_MUTED);

        container(
            row![mute_btn, gain_slider, gain_label, disconnect_btn]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .width(col_width)
        .padding([10, 12])
        .style(theme::cell_style)
        .into()
    }
}

/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! WaveLink-inspired `Channels × Mixes` routing matrix view.

mod cell;

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, space, text, text_input},
};
use pipelink_common::types::ChannelName;

use crate::{
    icons,
    state::{App, Message},
    theme,
};

const CHANNEL_COL_WIDTH: f32 = 270.0;
pub(super) const MIX_COL_WIDTH: f32 = 230.0;

impl App {
    /// Renders the interactive `Channels × Mixes` matrix.
    pub(super) fn view_matrix(&self) -> Element<'_, Message> {
        let mut rows = column![self.view_matrix_header()].spacing(10);

        for channel in &self.snapshot.channels {
            rows = rows.push(self.view_channel_row(channel));
        }

        rows = rows.push(self.view_create_channel_row());

        scrollable(rows)
            .direction(scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::default(),
                horizontal: scrollable::Scrollbar::default(),
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_matrix_header(&self) -> Element<'_, Message> {
        let mut header = row![
            container(text("Channels").size(13).color(theme::TEXT_MUTED))
                .width(CHANNEL_COL_WIDTH)
                .padding([8, 12]),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        for mix in &self.snapshot.mixes {
            let is_active = self.snapshot.active_mix.as_ref() == Some(mix);
            let style = if is_active {
                theme::active_mix_header_style
            } else {
                theme::card_style
            };

            let mix_tip = if is_active {
                "Active monitor mix"
            } else {
                "Monitor this mix"
            };
            let activate_btn = icons::hint(
                button(icons::monitor_mix(is_active, 15.0))
                    .on_press(Message::SelectActiveMix(mix.clone()))
                    .padding([5, 7])
                    .style(theme::active_mix_button(is_active)),
                mix_tip,
            );

            let remove_btn = button(text("x").size(11))
                .on_press(Message::RemoveMix(mix.clone()))
                .padding([4, 8])
                .style(theme::subtle_button);

            let card = container(
                row![
                    text(mix.to_string()).size(14).color(theme::TEXT_PRIMARY),
                    space().width(Length::Fill),
                    activate_btn,
                    remove_btn,
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .width(MIX_COL_WIDTH)
            .padding([10, 12])
            .style(style);

            header = header.push(card);
        }

        header = header.push(self.view_create_mix_header());
        header.into()
    }

    fn view_create_mix_header(&self) -> Element<'_, Message> {
        if !self.creating_mix {
            return button(text("+ Mix").size(13))
                .on_press(Message::OpenCreateMix)
                .padding([10, 14])
                .style(theme::subtle_button)
                .into();
        }

        let input = text_input("Mix name", &self.new_mix_name)
            .on_input(Message::NewMixNameChanged)
            .on_submit(Message::SubmitCreateMix)
            .padding([6, 8])
            .width(120);

        container(
            row![
                input,
                button(text("Add").size(12))
                    .on_press(Message::SubmitCreateMix)
                    .padding([6, 10])
                    .style(theme::subtle_button),
                button(text("x").size(12))
                    .on_press(Message::CancelCreateMix)
                    .padding([6, 8])
                    .style(theme::subtle_button),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(6)
        .style(theme::card_style)
        .into()
    }

    fn view_channel_row<'a>(&'a self, channel: &'a ChannelName) -> Element<'a, Message> {
        let app_count = self.routed_app_count(channel);
        let subtitle = if app_count == 1 {
            "1 app routed".to_string()
        } else {
            format!("{app_count} apps routed")
        };

        let is_default = self.is_default_channel(channel);
        let default_tip = if is_default {
            "Current default audio sink"
        } else {
            "Set as default audio sink"
        };
        let default_btn = icons::hint(
            button(icons::default_sink(is_default, 14.0))
                .on_press(Message::SetDefaultSink(channel.to_string()))
                .padding([5, 7])
                .style(theme::default_sink_button(is_default)),
            default_tip,
        );

        let channel_badge = container(
            row![
                column![
                    text(channel.to_string())
                        .size(14)
                        .color(theme::TEXT_PRIMARY),
                    text(subtitle).size(11).color(theme::TEXT_MUTED),
                ]
                .spacing(2),
                space().width(Length::Fill),
                default_btn,
                button(text("x").size(11))
                    .on_press(Message::RemoveChannel(channel.clone()))
                    .padding([4, 8])
                    .style(theme::subtle_button),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .width(CHANNEL_COL_WIDTH)
        .padding([12, 14])
        .style(theme::card_style);

        let mut line = row![channel_badge].spacing(10).align_y(Alignment::Center);

        for mix in &self.snapshot.mixes {
            line = line.push(self.view_matrix_cell(channel, mix));
        }

        line.into()
    }

    fn view_create_channel_row(&self) -> Element<'_, Message> {
        if !self.creating_channel {
            return button(text("+ Create channel").size(13))
                .on_press(Message::OpenCreateChannel)
                .padding([12, 16])
                .width(CHANNEL_COL_WIDTH)
                .style(theme::subtle_button)
                .into();
        }

        let input = text_input("Channel name", &self.new_channel_name)
            .on_input(Message::NewChannelNameChanged)
            .on_submit(Message::SubmitCreateChannel)
            .padding([8, 10])
            .width(150);

        container(
            row![
                input,
                button(text("Create").size(12))
                    .on_press(Message::SubmitCreateChannel)
                    .padding([8, 12])
                    .style(theme::subtle_button),
                button(text("Cancel").size(12))
                    .on_press(Message::CancelCreateChannel)
                    .padding([8, 10])
                    .style(theme::subtle_button),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding(10)
        .style(theme::card_style)
        .into()
    }
}

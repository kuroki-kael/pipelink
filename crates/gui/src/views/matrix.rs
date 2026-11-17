/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! WaveLink-inspired `Channels × Mixes` routing matrix view.

mod cell;
mod channel_card;

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, space, text, text_input},
};

use crate::{
    icons,
    state::{App, Message},
    theme,
};

pub(super) const CHANNEL_COL_WIDTH: f32 = 290.0;
pub(super) const MIX_COL_WIDTH: f32 = 230.0;

impl App {
    /// Renders the interactive `Channels × Mixes` matrix.
    pub(super) fn view_matrix(&self) -> Element<'_, Message> {
        let mut rows = column![self.view_matrix_header()].spacing(10);
        let total = self.snapshot.channels.len();

        for (idx, channel) in self.snapshot.channels.iter().enumerate() {
            rows = rows.push(self.view_channel_row(idx, total, channel));
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

            let header_content: Element<'_, Message> = if let Some((_, draft)) = self
                .renaming_mix
                .as_ref()
                .filter(|(target, _)| target == mix)
            {
                let trimmed = draft.trim();
                let valid = !trimmed.is_empty()
                    && trimmed != mix.as_ref()
                    && trimmed
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                    && !self
                        .snapshot
                        .mixes
                        .iter()
                        .any(|m| m.eq_ignore_ascii_case(trimmed));
                let mut save_btn = button(text("OK").size(11))
                    .padding([5, 8])
                    .style(theme::subtle_button);
                if valid {
                    save_btn = save_btn.on_press(Message::SubmitRenameMix);
                }
                row![
                    text_input("Mix name", draft)
                        .on_input(Message::MixRenameDraftChanged)
                        .on_submit(Message::SubmitRenameMix)
                        .padding([4, 6])
                        .size(13)
                        .width(Length::Fill),
                    save_btn,
                    button(text("x").size(11))
                        .on_press(Message::CancelRenameMix)
                        .padding([5, 8])
                        .style(theme::subtle_button),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .into()
            } else {
                let title_btn = icons::hint(
                    button(text(mix.to_string()).size(14).color(theme::TEXT_PRIMARY))
                        .on_press(Message::StartRenameMix(mix.clone()))
                        .padding([2, 4])
                        .style(theme::channel_title_button),
                    "Click to rename mix",
                );
                row![
                    title_btn,
                    space().width(Length::Fill),
                    activate_btn,
                    remove_btn,
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .into()
            };

            let card = container(header_content)
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
}

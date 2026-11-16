/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Top header bar with output sink selection and inline error notifications.

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, pick_list, row, space, text},
};

use crate::{
    state::{App, Message, NavPage, OutputOption},
    theme,
};

impl App {
    /// Renders the page title, output sink selector, and any active error banner.
    pub(super) fn view_topbar(&self) -> Element<'_, Message> {
        let title = match self.page {
            NavPage::Mixes => "Mixes",
            NavPage::Settings => "Settings",
        };

        let options: Vec<OutputOption> = self
            .snapshot
            .outputs
            .iter()
            .cloned()
            .map(OutputOption)
            .collect();
        let selected = options.iter().find(|opt| opt.0.active).cloned();

        let output_picker = pick_list(options, selected, Message::SelectOutput)
            .placeholder("No audio output")
            .padding([8, 12])
            .width(300);

        let controls = row![
            text(title).size(26).color(theme::TEXT_PRIMARY),
            space().width(Length::Fill),
            output_picker,
            button(text("Refresh").size(12))
                .on_press(Message::RefreshSnapshot)
                .padding([8, 12])
                .style(theme::subtle_button),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let mut wrapper = column![controls].spacing(10).width(Length::Fill);

        if let Some(err) = &self.last_error {
            let banner = container(
                row![
                    text(err).size(13).color(theme::ACCENT_RED),
                    space().width(Length::Fill),
                    button(text("Dismiss").size(12))
                        .on_press(Message::DismissError)
                        .padding([4, 8])
                        .style(theme::subtle_button),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
            .width(Length::Fill)
            .style(theme::error_banner_style);

            wrapper = wrapper.push(banner);
        }

        wrapper.into()
    }
}

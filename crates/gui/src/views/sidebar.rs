/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Left navigation sidebar and daemon connection status indicator.

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, space, text},
};

use crate::{
    icons,
    state::{App, Message, NavPage},
    theme,
};

impl App {
    /// Renders the compact left sidebar.
    pub(super) fn view_sidebar(&self) -> Element<'_, Message> {
        let mixes_selected = self.page == NavPage::Mixes;
        let mixes_btn = button(
            row![
                icons::nav_mixes(mixes_selected, 15.0),
                text("Mixes").size(14)
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .on_press(Message::SelectPage(NavPage::Mixes))
        .padding([10, 12])
        .width(Length::Fill)
        .style(theme::nav_button(mixes_selected));

        let settings_selected = self.page == NavPage::Settings;
        let settings_btn = button(
            row![
                icons::nav_settings(settings_selected, 15.0),
                text("Settings").size(14)
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .on_press(Message::SelectPage(NavPage::Settings))
        .padding([10, 12])
        .width(Length::Fill)
        .style(theme::nav_button(settings_selected));

        let (status_color, status_label) = if self.connected {
            (theme::ACCENT_GREEN, "Daemon online")
        } else {
            (theme::ACCENT_RED, "Daemon offline")
        };

        let status_dot = container(space().width(8).height(8)).style(move |_| container::Style {
            background: Some(iced::Background::Color(status_color)),
            border: iced::Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let status_row = row![
            status_dot,
            text(status_label).size(12).color(theme::TEXT_MUTED)
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let retry_btn = button(text("Reconnect").size(12))
            .on_press(Message::RefreshSnapshot)
            .padding([6, 10])
            .width(Length::Fill)
            .style(theme::subtle_button);

        let mut footer = column![status_row].spacing(8);
        if !self.connected {
            footer = footer.push(retry_btn);
        }

        let brand = column![
            icons::brand_logo(140.0),
            space().height(4),
            text("PipeLink").size(20).color(theme::TEXT_PRIMARY),
            text("Audio Routing").size(12).color(theme::TEXT_MUTED),
        ]
        .width(Length::Fill)
        .spacing(2)
        .align_x(Alignment::Center);

        container(
            column![
                brand,
                space().height(12),
                mixes_btn,
                settings_btn,
                space().height(Length::Fill),
                footer,
            ]
            .spacing(6)
            .height(Length::Fill),
        )
        .width(200)
        .height(Length::Fill)
        .padding(16)
        .style(theme::sidebar_style)
        .into()
    }
}

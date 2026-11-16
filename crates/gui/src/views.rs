/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Top-level UI view composition implemented on [`App`].

mod matrix;
mod settings;
mod sidebar;
mod topbar;

use iced::{
    Element, Length,
    widget::{column, container, row},
};

use crate::{
    state::{App, Message, NavPage},
    theme,
};

impl App {
    /// Renders the complete `PipeLink` window layout.
    pub(crate) fn render_window(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = match self.page {
            NavPage::Mixes => self.view_matrix(),
            NavPage::Settings => self.view_settings(),
        };

        let main_pane = container(
            column![self.view_topbar(), body]
                .spacing(16)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(theme::BG_APP)),
            ..Default::default()
        });

        row![self.view_sidebar(), main_pane]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

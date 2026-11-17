/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Searchable `+ Add app` picker listing active streams and installed `.desktop` apps.

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, space, text, text_input},
};
use pipelink_common::types::ChannelName;

use crate::{
    icons,
    state::{App, Message, PickerAppEntry},
    theme,
};

impl App {
    pub(super) fn view_app_picker<'a>(
        &'a self,
        channel: &'a ChannelName,
        query: &'a str,
    ) -> Element<'a, Message> {
        let search_row = container(
            row![
                icons::search(14.0, theme::TEXT_MUTED),
                text_input("Search active or installed apps...", query)
                    .on_input(Message::AppSearchChanged)
                    .padding([6, 8])
                    .width(Length::Fill),
                button(text("Cancel").size(11))
                    .on_press(Message::ToggleAppPicker)
                    .padding([5, 9])
                    .style(theme::subtle_button),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([6, 8])
        .style(theme::card_style);

        let groups = self.picker_apps(Some(channel), query);
        let has_active = !groups.active.is_empty();
        let has_installed = !groups.installed.is_empty();
        let mut list = column![].spacing(6).width(Length::Fill);

        if has_active {
            list = list.push(text("Active apps").size(11).color(theme::TEXT_MUTED));
            for entry in groups.active {
                list = list.push(self.view_picker_entry(channel, entry));
            }
        }

        if has_installed {
            if has_active {
                list = list.push(space().height(6));
            }
            list = list.push(text("Installed apps").size(11).color(theme::TEXT_MUTED));
            for entry in groups.installed {
                list = list.push(self.view_picker_entry(channel, entry));
            }
        }

        if !has_active && !has_installed {
            list = list.push(
                container(
                    text("No matching applications found.")
                        .size(12)
                        .color(theme::TEXT_MUTED),
                )
                .padding([16, 8]),
            );
        }

        column![
            search_row,
            scrollable(list).height(Length::Fill).width(Length::Fill),
        ]
        .spacing(10)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    fn view_picker_entry<'a>(
        &'a self,
        target_channel: &'a ChannelName,
        entry: PickerAppEntry,
    ) -> Element<'a, Message> {
        let resolved = self.lookup_app_icon(&entry.display_name, entry.icon_key.as_deref());
        let badge = icons::app_badge(&entry.display_name, resolved, 22.0);

        let status = match (&entry.current_channel, entry.active_streams) {
            (Some(ch), _) => text(format!("-> {ch}")).size(11).color(theme::ACCENT_AMBER),
            (None, Some(n)) => text(format!("{n} active"))
                .size(11)
                .color(theme::ACCENT_GREEN),
            (None, None) => text("Installed").size(11).color(theme::TEXT_MUTED),
        };

        button(
            row![
                badge,
                text(entry.display_name.to_string())
                    .size(13)
                    .color(theme::TEXT_PRIMARY),
                space().width(Length::Fill),
                status,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .on_press(Message::RouteApp {
            app: entry.display_name,
            channel: target_channel.clone(),
        })
        .width(Length::Fill)
        .padding([7, 10])
        .style(theme::channel_title_button)
        .into()
    }
}

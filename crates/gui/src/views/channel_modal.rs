/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Channel detail modal overlay for managing routed applications.

mod picker;

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, opaque, row, scrollable, space, stack, text, text_input},
};
use pipelink_common::types::ChannelName;

use crate::{
    icons,
    state::{App, ChannelModalState, Message, RoutedChannelApp},
    theme,
};

impl App {
    /// Renders the centered Channel Detail modal over a dimmed backdrop.
    pub(super) fn view_channel_modal<'a>(
        &'a self,
        modal: &'a ChannelModalState,
    ) -> Element<'a, Message> {
        let channel = &modal.channel;
        let routed = self.routed_apps_for_channel(channel);
        let active_count = routed.iter().filter(|a| a.active_streams.is_some()).count();

        let can_rename = {
            let trimmed = modal.rename_draft.trim();
            !trimmed.is_empty() && trimmed != channel.as_ref()
        };

        let mut rename_row = row![
            text_input("Channel name", &modal.rename_draft)
                .on_input(Message::ChannelRenameDraftChanged)
                .on_submit(Message::SubmitRenameChannel)
                .padding([6, 10])
                .width(Length::Fixed(220.0)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if can_rename {
            rename_row = rename_row.push(
                button(text("Rename").size(12))
                    .on_press(Message::SubmitRenameChannel)
                    .padding([6, 12])
                    .style(theme::subtle_button),
            );
        }

        let header = row![
            rename_row,
            if self.is_default_channel(channel) {
                container(text("Default sink").size(11).color(theme::ACCENT_GREEN))
                    .padding([3, 8])
                    .style(theme::cell_style)
            } else {
                container(space().width(0).height(0))
            },
            space().width(Length::Fill),
            button(icons::close(14.0, theme::TEXT_MUTED))
                .on_press(Message::CloseChannelDetails)
                .padding([6, 8])
                .style(theme::nav_button(false)),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let left_summary = self.view_channel_summary(channel, &routed, active_count);
        let right_panel = self.view_apps_panel(modal, &routed);

        let body = row![left_summary, right_panel]
            .spacing(16)
            .height(Length::Fill)
            .width(Length::Fill);

        let footer = row![
            button(
                row![
                    icons::trash(13.0, theme::ACCENT_RED),
                    text("Remove channel").size(12).color(theme::ACCENT_RED),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .on_press(Message::RemoveChannel(channel.clone()))
            .padding([8, 12])
            .style(theme::danger_button),
            space().width(Length::Fill),
            button(text("Close window").size(12).color(theme::BG_APP))
                .on_press(Message::CloseChannelDetails)
                .padding([8, 16])
                .style(theme::primary_button),
        ]
        .align_y(Alignment::Center);

        let card = opaque(
            container(
                column![header, body, footer]
                    .spacing(16)
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .width(680)
            .height(470)
            .padding(20)
            .style(theme::modal_card_style),
        );

        let backdrop = button(space().width(Length::Fill).height(Length::Fill))
            .on_press(Message::CloseChannelDetails)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::backdrop_button);

        let centered_card = container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

        stack![backdrop, centered_card].into()
    }

    fn view_channel_summary<'a>(
        &'a self,
        channel: &'a ChannelName,
        routed: &[RoutedChannelApp],
        active_count: usize,
    ) -> Element<'a, Message> {
        let mut preview_icons = row![].spacing(8).align_y(Alignment::Center);
        for app in routed.iter().take(4) {
            let resolved = self.lookup_app_icon(&app.display_name, app.icon_key.as_deref());
            preview_icons = preview_icons.push(icons::app_badge(&app.display_name, resolved, 36.0));
        }

        let preview_tile = if routed.is_empty() {
            container(
                text(channel.chars().next().unwrap_or('#').to_string())
                    .size(44)
                    .color(theme::ACCENT_PINK),
            )
            .width(Length::Fill)
            .height(150)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(theme::cell_style)
        } else {
            container(preview_icons)
                .width(Length::Fill)
                .height(150)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center)
                .style(theme::cell_style)
        };

        let routed_mixes = self
            .snapshot
            .routes
            .iter()
            .filter(|r| r.channel.eq_ignore_ascii_case(channel) && r.active)
            .count();

        container(
            column![
                preview_tile,
                space().height(8),
                text(format!("{} routed app(s)", routed.len()))
                    .size(13)
                    .color(theme::TEXT_PRIMARY),
                text(format!("{active_count} active · {routed_mixes} mix(es)"))
                    .size(12)
                    .color(theme::TEXT_MUTED),
            ]
            .spacing(4),
        )
        .width(205)
        .height(Length::Fill)
        .padding(14)
        .style(theme::card_style)
        .into()
    }

    fn view_apps_panel<'a>(
        &'a self,
        modal: &'a ChannelModalState,
        routed: &[RoutedChannelApp],
    ) -> Element<'a, Message> {
        let content: Element<'a, Message> = if let Some(query) = &modal.picker_query {
            self.view_app_picker(&modal.channel, query)
        } else {
            let mut items = column![].spacing(6).width(Length::Fill);
            if routed.is_empty() {
                items = items.push(
                    container(
                        text("No apps routed to this channel yet.")
                            .size(12)
                            .color(theme::TEXT_MUTED),
                    )
                    .padding([18, 6]),
                );
            } else {
                for app in routed {
                    items = items.push(self.view_routed_app_row(app));
                }
            }

            let add_btn = button(
                row![
                    icons::plus(13.0, theme::TEXT_PRIMARY),
                    text("Add app").size(13).color(theme::TEXT_PRIMARY),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            )
            .on_press(Message::ToggleAppPicker)
            .padding([9, 12])
            .width(Length::Fill)
            .style(theme::subtle_button);

            column![
                scrollable(items).height(Length::Fill).width(Length::Fill),
                add_btn,
            ]
            .spacing(10)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
        };

        container(
            column![
                text("Apps").size(13).color(theme::TEXT_PRIMARY),
                space().height(4),
                content,
            ]
            .spacing(6)
            .height(Length::Fill)
            .width(Length::Fill),
        )
        .padding(14)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::card_style)
        .into()
    }

    fn view_routed_app_row<'a>(&'a self, app: &RoutedChannelApp) -> Element<'a, Message> {
        let resolved = self.lookup_app_icon(&app.display_name, app.icon_key.as_deref());
        let badge = icons::app_badge(&app.display_name, resolved, 22.0);
        let status = match app.active_streams {
            Some(1) => text("Active").size(11).color(theme::ACCENT_GREEN),
            Some(n) => text(format!("Active ({n})"))
                .size(11)
                .color(theme::ACCENT_GREEN),
            None => text("(Unavailable)").size(11).color(theme::TEXT_MUTED),
        };

        let remove_btn = icons::hint(
            button(icons::trash(13.0, theme::TEXT_MUTED))
                .on_press(Message::UnrouteApp(app.display_name.clone()))
                .padding([5, 7])
                .style(theme::nav_button(false)),
            "Remove app route",
        );

        container(
            row![
                badge,
                text(app.display_name.to_string())
                    .size(13)
                    .color(theme::TEXT_PRIMARY),
                status,
                space().width(Length::Fill),
                remove_btn,
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([6, 8])
        .width(Length::Fill)
        .style(theme::cell_style)
        .into()
    }
}

/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Channel badge card (null-sink volume/mute + ordering) and `+ Create channel` UI.

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, slider, space, text, text_input},
};
use pipelink_common::types::ChannelName;

use super::{CHANNEL_COL_WIDTH, MIX_COL_WIDTH};
use crate::{
    icons,
    state::{App, CreateChannelMode, Message, PickerAppEntry},
    theme,
};

impl App {
    pub(super) fn view_channel_row<'a>(
        &'a self,
        idx: usize,
        total: usize,
        channel: &'a ChannelName,
    ) -> Element<'a, Message> {
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

        let title_btn = icons::hint(
            button(
                column![
                    text(channel.to_string())
                        .size(14)
                        .color(theme::TEXT_PRIMARY),
                    text(subtitle).size(11).color(theme::TEXT_MUTED),
                ]
                .spacing(2),
            )
            .on_press(Message::OpenChannelDetails(channel.clone()))
            .padding([2, 4])
            .width(Length::Fill)
            .style(theme::channel_title_button),
            "Rename or manage routed apps",
        );

        let mut header_row = row![title_btn].spacing(4).align_y(Alignment::Center);

        if total > 1 {
            let mut up = button(text("↑").size(11))
                .padding([3, 6])
                .style(theme::subtle_button);
            if idx > 0 {
                up = up.on_press(Message::MoveChannel {
                    channel: channel.clone(),
                    to_index: idx - 1,
                });
            }
            let mut down = button(text("↓").size(11))
                .padding([3, 6])
                .style(theme::subtle_button);
            if idx + 1 < total {
                down = down.on_press(Message::MoveChannel {
                    channel: channel.clone(),
                    to_index: idx + 1,
                });
            }
            header_row = header_row.push(up).push(down);
        }

        header_row = header_row.push(default_btn).push(
            button(text("x").size(11))
                .on_press(Message::RemoveChannel(channel.clone()))
                .padding([4, 7])
                .style(theme::subtle_button),
        );

        let master_row = self.view_channel_master_row(channel);

        let channel_badge = container(column![header_row, master_row].spacing(8))
            .width(CHANNEL_COL_WIDTH)
            .padding([10, 12])
            .style(theme::card_style);

        let mut line = row![channel_badge].spacing(10).align_y(Alignment::Center);
        for mix in &self.snapshot.mixes {
            line = line.push(self.view_matrix_cell(channel, mix, MIX_COL_WIDTH));
        }

        line.into()
    }

    fn view_channel_master_row<'a>(&'a self, channel: &'a ChannelName) -> Element<'a, Message> {
        let (master_vol, master_muted) = self.channel_volume(channel);
        let mute_tip = if master_muted {
            "Unmute channel"
        } else {
            "Mute channel"
        };
        let mute_btn = icons::hint(
            button(icons::mute_speaker(master_muted, 14.0))
                .on_press(Message::ToggleChannelMute {
                    channel: channel.clone(),
                    muted: !master_muted,
                })
                .padding([5, 7])
                .style(theme::mute_button(master_muted)),
            mute_tip,
        );

        let ch_slider = channel.clone();
        let master_slider = slider(0.0_f32..=200.0_f32, master_vol, move |percent| {
            Message::ChannelVolumeChanged {
                channel: ch_slider.clone(),
                percent,
            }
        })
        .step(1.0_f32)
        .on_release(Message::ChannelVolumeReleased {
            channel: channel.clone(),
        })
        .style(theme::accent_slider)
        .width(Length::Fill);

        let vol_label = text(format!("{master_vol:.0}%"))
            .size(11)
            .color(theme::TEXT_MUTED);

        row![mute_btn, master_slider, vol_label]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
    }

    pub(super) fn view_create_channel_row(&self) -> Element<'_, Message> {
        match &self.create_channel {
            None => button(text("+ Create channel").size(13))
                .on_press(Message::OpenCreateChannel)
                .padding([12, 16])
                .width(CHANNEL_COL_WIDTH)
                .style(theme::subtle_button)
                .into(),
            Some(CreateChannelMode::EmptyInput(name)) => {
                let input = text_input("Channel name", name)
                    .on_input(Message::NewChannelNameChanged)
                    .on_submit(Message::SubmitCreateChannel)
                    .padding([8, 10])
                    .width(Length::Fill);

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
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .width(CHANNEL_COL_WIDTH)
                .padding(10)
                .style(theme::card_style)
                .into()
            },
            Some(CreateChannelMode::AppPicker(query)) => self.view_create_channel_picker(query),
        }
    }

    fn view_create_channel_picker<'a>(&'a self, query: &'a str) -> Element<'a, Message> {
        let search_row = row![
            icons::search(14.0, theme::TEXT_MUTED),
            text_input("Search app for new channel...", query)
                .on_input(Message::ChannelCreateSearchChanged)
                .padding([6, 8])
                .width(Length::Fill),
            button(icons::close(13.0, theme::TEXT_MUTED))
                .on_press(Message::CancelCreateChannel)
                .padding([5, 7])
                .style(theme::nav_button(false)),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let groups = self.picker_apps(None, query);
        let mut list = column![].spacing(4).width(Length::Fill);

        if !groups.active.is_empty() {
            list = list.push(text("Active apps").size(11).color(theme::TEXT_MUTED));
            for entry in groups.active {
                list = list.push(self.view_create_app_entry(entry));
            }
        }

        if !groups.installed.is_empty() {
            list = list.push(text("Installed apps").size(11).color(theme::TEXT_MUTED));
            for entry in groups.installed {
                list = list.push(self.view_create_app_entry(entry));
            }
        }

        let empty_btn = button(
            row![
                text("Add empty channel")
                    .size(12)
                    .color(theme::TEXT_PRIMARY),
                space().width(Length::Fill),
                text(">").size(12).color(theme::TEXT_MUTED),
            ]
            .align_y(Alignment::Center),
        )
        .on_press(Message::OpenEmptyChannelInput)
        .padding([7, 10])
        .width(Length::Fill)
        .style(theme::subtle_button);

        container(
            column![
                search_row,
                scrollable(list).height(190).width(Length::Fill),
                empty_btn,
            ]
            .spacing(8),
        )
        .width(CHANNEL_COL_WIDTH)
        .padding(10)
        .style(theme::card_style)
        .into()
    }

    fn view_create_app_entry(&self, entry: PickerAppEntry) -> Element<'_, Message> {
        let resolved = self.lookup_app_icon(&entry.display_name, entry.icon_key.as_deref());
        let badge = icons::app_badge(&entry.display_name, resolved, 20.0);
        let channel_name = ChannelName::from(entry.display_name.as_ref());

        button(
            row![
                badge,
                text(entry.display_name.to_string())
                    .size(12)
                    .color(theme::TEXT_PRIMARY),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .on_press(Message::CreateChannelForApp {
            channel: channel_name,
            app: entry.display_name,
        })
        .width(Length::Fill)
        .padding([6, 8])
        .style(theme::channel_title_button)
        .into()
    }
}

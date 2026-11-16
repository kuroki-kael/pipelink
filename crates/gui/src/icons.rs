/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Embedded vector SVG icons and window branding assets for `pipelink-gui`.

use iced::{
    Color, Length, Theme,
    widget::{
        self,
        svg::{Handle, Style as SvgStyle, Svg},
        text,
        tooltip::{Position, Tooltip},
    },
    window::{Icon, icon},
};

use crate::theme;

const PIPELINK_LOGO: &[u8] = include_bytes!("../../../assets/icons/ui/pipelink-brand.svg");
const PIPELINK_WINDOW_RGBA: &[u8] = include_bytes!("../../../assets/pipelink-64.rgba");
const STAR_OUTLINE: &[u8] = include_bytes!("../../../assets/icons/ui/star-outline.svg");
const STAR_FILLED: &[u8] = include_bytes!("../../../assets/icons/ui/star-filled.svg");
const HEADPHONES_OUTLINE: &[u8] = include_bytes!("../../../assets/icons/ui/headphones-outline.svg");
const HEADPHONES_FILLED: &[u8] = include_bytes!("../../../assets/icons/ui/headphones-filled.svg");
const SLIDERS: &[u8] = include_bytes!("../../../assets/icons/ui/sliders.svg");
const SETTINGS: &[u8] = include_bytes!("../../../assets/icons/ui/settings.svg");

/// Decodes the embedded 64×64 RGBA application icon for the OS window manager.
#[must_use]
pub(crate) fn window_icon() -> Option<Icon> {
    icon::from_rgba(PIPELINK_WINDOW_RGBA.to_vec(), 64, 64).ok()
}

/// Renders the tinted `PipeLink` symbolic brand icon for the sidebar header at
/// tight aspect bounds (`1024×524`).
#[must_use]
pub(crate) fn brand_logo(width: f32) -> Svg<'static, Theme> {
    let height = (width * (524.0 / 1024.0)).round();
    tinted_svg_sized(PIPELINK_LOGO, width, height, theme::BRAND_PINK)
}

/// Renders the system default audio sink star icon.
#[must_use]
pub(crate) fn default_sink(active: bool, size: f32) -> Svg<'static, Theme> {
    let bytes = if active { STAR_FILLED } else { STAR_OUTLINE };
    let color = if active {
        theme::ACCENT_GREEN
    } else {
        theme::TEXT_MUTED
    };
    tinted_svg(bytes, size, color)
}

/// Renders the monitor headphones icon for an active or inactive mix header.
#[must_use]
pub(crate) fn monitor_mix(active: bool, size: f32) -> Svg<'static, Theme> {
    let bytes = if active {
        HEADPHONES_FILLED
    } else {
        HEADPHONES_OUTLINE
    };
    let color = if active {
        theme::ACCENT_PINK
    } else {
        theme::TEXT_MUTED
    };
    tinted_svg(bytes, size, color)
}

/// Renders the Mixes navigation icon.
#[must_use]
pub(crate) fn nav_mixes(selected: bool, size: f32) -> Svg<'static, Theme> {
    tinted_svg(SLIDERS, size, nav_color(selected))
}

/// Renders the Settings navigation icon.
#[must_use]
pub(crate) fn nav_settings(selected: bool, size: f32) -> Svg<'static, Theme> {
    tinted_svg(SETTINGS, size, nav_color(selected))
}

/// Wraps `content` in a themed hover tooltip styled consistently with the dark UI.
pub(crate) fn hint<'a, Message: 'a>(
    content: impl Into<iced::Element<'a, Message>>,
    label: &'a str,
) -> Tooltip<'a, Message> {
    widget::tooltip(
        content,
        text(label).size(11).color(theme::TEXT_PRIMARY),
        Position::Top,
    )
    .padding(6)
    .style(theme::card_style)
}

fn nav_color(selected: bool) -> Color {
    if selected {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    }
}

fn tinted_svg(bytes: &'static [u8], size: f32, color: Color) -> Svg<'static, Theme> {
    tinted_svg_sized(bytes, size, size, color)
}

fn tinted_svg_sized(
    bytes: &'static [u8],
    width: f32,
    height: f32,
    color: Color,
) -> Svg<'static, Theme> {
    widget::svg(Handle::from_memory(bytes))
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(move |_theme, _status| SvgStyle { color: Some(color) })
}

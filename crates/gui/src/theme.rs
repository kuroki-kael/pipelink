/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Dark WaveLink-inspired color palette and widget styles for `pipelink-gui`.

use iced::{
    Background, Border, Color, Theme,
    theme::Palette,
    widget::{button, container, slider},
};

pub(crate) const BG_APP: Color = Color::from_rgb(0.078, 0.078, 0.094);
pub(crate) const BG_SIDEBAR: Color = Color::from_rgb(0.102, 0.102, 0.125);
pub(crate) const BG_CARD: Color = Color::from_rgb(0.129, 0.129, 0.157);
pub(crate) const BG_CELL: Color = Color::from_rgb(0.157, 0.157, 0.192);
pub(crate) const BG_CELL_DIM: Color = Color::from_rgb(0.114, 0.114, 0.137);
pub(crate) const BG_HOVER: Color = Color::from_rgb(0.204, 0.204, 0.247);
pub(crate) const BORDER_SUBTLE: Color = Color::from_rgb(0.220, 0.220, 0.267);

pub(crate) const TEXT_PRIMARY: Color = Color::from_rgb(0.941, 0.941, 0.961);
pub(crate) const TEXT_MUTED: Color = Color::from_rgb(0.604, 0.604, 0.659);

pub(crate) const ACCENT_PINK: Color = Color::from_rgb(0.878, 0.569, 0.788);
pub(crate) const BRAND_PINK: Color = Color::from_rgb8(0xF1, 0x71, 0x8A);
pub(crate) const ACCENT_GREEN: Color = Color::from_rgb(0.239, 0.839, 0.549);
pub(crate) const ACCENT_AMBER: Color = Color::from_rgb(0.910, 0.690, 0.290);
pub(crate) const ACCENT_RED: Color = Color::from_rgb(0.878, 0.424, 0.459);

/// Builds the custom dark `iced` theme.
#[must_use]
pub(crate) fn app_theme() -> Theme {
    Theme::custom(
        "PipeLink Dark".to_string(),
        Palette {
            background: BG_APP,
            text: TEXT_PRIMARY,
            primary: ACCENT_PINK,
            success: ACCENT_GREEN,
            warning: ACCENT_AMBER,
            danger: ACCENT_RED,
        },
    )
}

pub(crate) fn sidebar_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_SIDEBAR)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_CARD)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn active_mix_header_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_CELL)),
        border: Border {
            color: ACCENT_PINK,
            width: 1.5,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn cell_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_CELL)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn inactive_cell_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_CELL_DIM)),
        border: Border {
            color: Color::from_rgba(0.220, 0.220, 0.267, 0.55),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn error_banner_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(
            0.878, 0.424, 0.459, 0.16,
        ))),
        border: Border {
            color: ACCENT_RED,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn subtle_button(theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Background::Color(BG_CELL)),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    };
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(Background::Color(BG_HOVER)),
            ..base
        },
        button::Status::Disabled => button::secondary(theme, status),
        button::Status::Active => base,
    }
}

pub(crate) fn nav_button(selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = if selected {
            BG_CELL
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            BG_HOVER
        } else {
            Color::TRANSPARENT
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if selected { TEXT_PRIMARY } else { TEXT_MUTED },
            border: Border {
                color: if selected {
                    BORDER_SUBTLE
                } else {
                    Color::TRANSPARENT
                },
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }
}

pub(crate) fn mute_button(muted: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    toggle_accent_button(muted, ACCENT_RED, BG_CARD, TEXT_PRIMARY)
}

pub(crate) fn default_sink_button(
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    toggle_accent_button(active, ACCENT_GREEN, BG_CELL, TEXT_MUTED)
}

pub(crate) fn active_mix_button(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    toggle_accent_button(active, ACCENT_PINK, BG_CELL, TEXT_MUTED)
}

fn toggle_accent_button(
    active: bool,
    accent: Color,
    inactive_bg: Color,
    inactive_text: Color,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let active_bg = Color::from_rgba(accent.r, accent.g, accent.b, 0.22);
        button::Style {
            background: Some(Background::Color(if active {
                active_bg
            } else if hovered {
                BG_HOVER
            } else {
                inactive_bg
            })),
            text_color: if active { accent } else { inactive_text },
            border: Border {
                color: if active { accent } else { BORDER_SUBTLE },
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }
}

pub(crate) fn accent_slider(_theme: &Theme, _status: slider::Status) -> slider::Style {
    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(ACCENT_PINK),
                Background::Color(Color::from_rgb(0.27, 0.27, 0.33)),
            ),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 6.0 },
            background: Background::Color(ACCENT_PINK),
            border_width: 1.5,
            border_color: TEXT_PRIMARY,
        },
    }
}

pub(crate) fn modal_card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(BG_SIDEBAR)),
        border: Border {
            color: BORDER_SUBTLE,
            width: 1.0,
            radius: 12.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn backdrop_button(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.62))),
        ..Default::default()
    }
}

pub(crate) fn channel_title_button(_theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if hovered {
            BG_HOVER
        } else {
            Color::TRANSPARENT
        })),
        text_color: TEXT_PRIMARY,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    pill_button(
        Color::from_rgba(
            ACCENT_RED.r,
            ACCENT_RED.g,
            ACCENT_RED.b,
            if hovered { 0.28 } else { 0.16 },
        ),
        ACCENT_RED,
        ACCENT_RED,
    )
}

pub(crate) fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    pill_button(
        if hovered { BRAND_PINK } else { ACCENT_PINK },
        BG_APP,
        ACCENT_PINK,
    )
}

fn pill_button(bg: Color, text_color: Color, border_color: Color) -> button::Style {
    button::Style {
        background: Some(Background::Color(bg)),
        text_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

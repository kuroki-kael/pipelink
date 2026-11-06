/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Desktop and Flatpak application identifiers.

use std::{borrow::Borrow, fmt, num::ParseIntError, ops::Deref, str::FromStr};

use serde::{Deserialize, Serialize};

/// Desktop or Flatpak application identifier (e.g. "com.discordapp.Discord").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct AppId(pub String);

impl From<String> for AppId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for AppId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<AppId> for String {
    fn from(app_id: AppId) -> Self {
        app_id.0
    }
}

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for AppId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for AppId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for AppId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for AppId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for AppId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<AppId> for &str {
    fn eq(&self, other: &AppId) -> bool {
        *self == other.0
    }
}

impl PartialEq<AppId> for str {
    fn eq(&self, other: &AppId) -> bool {
        *self == other.0
    }
}

/// Valve Steam application identifier.
///
/// Steam app ids are numeric (e.g. `620` for Portal 2) and reach the game
/// through the `STEAM_COMPAT_APP_ID` and `SteamAppId` environment variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct SteamAppId(pub u32);

impl From<u32> for SteamAppId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<SteamAppId> for u32 {
    fn from(id: SteamAppId) -> Self {
        id.0
    }
}

impl AsRef<u32> for SteamAppId {
    fn as_ref(&self) -> &u32 {
        &self.0
    }
}

impl Borrow<u32> for SteamAppId {
    fn borrow(&self) -> &u32 {
        &self.0
    }
}

impl FromStr for SteamAppId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl fmt::Display for SteamAppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SteamAppId {
    /// Returns this Steam app id in the `steam_{id}` form used as [`AppId`].
    #[must_use]
    pub fn to_app_id(self) -> AppId {
        AppId::from(format!("steam_{self}"))
    }
}

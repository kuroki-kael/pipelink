/*
 * Copyright (c) 2026 Kuroki-Kael / PipeLink Contributors.
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

//! Daemon-side `StatusNotifierItem` tray icon (`org.kde.StatusNotifierItem` + `dbusmenu`)
//! compiled when the `gui` feature is enabled.

mod launcher;

use std::{
    collections::HashMap,
    process::Child,
    sync::{Arc, Mutex},
};

use tokio::sync::watch;
use tracing::{info, warn};
use zbus::{
    connection::Builder,
    interface,
    zvariant::{ObjectPath, OwnedValue, Value},
};

const ICON_RGBA_64: &[u8] = include_bytes!("../../assets/pipelink-64.rgba");
const MENU_OPEN_GUI_ID: i32 = 1;
const MENU_SEPARATOR_ID: i32 = 2;
const MENU_QUIT_DAEMON_ID: i32 = 3;

type MenuProps = HashMap<String, OwnedValue>;
type MenuLayoutNode = (i32, MenuProps, Vec<OwnedValue>);

#[derive(Clone)]
struct TrayController {
    gui_child: Arc<Mutex<Option<Child>>>,
    shutdown_tx: watch::Sender<bool>,
}

impl TrayController {
    fn open_gui(&self) {
        launcher::launch_independent_gui(&self.gui_child);
    }

    fn quit_daemon(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

struct StatusNotifierItem {
    controller: TrayController,
    icon_argb: Vec<u8>,
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl StatusNotifierItem {
    #[zbus(property)]
    #[expect(clippy::unused_self, reason = "zbus interface macro requires &self")]
    fn category(&self) -> &'static str {
        "ApplicationStatus"
    }

    #[zbus(property)]
    #[expect(clippy::unused_self, reason = "zbus interface macro requires &self")]
    fn id(&self) -> &'static str {
        "pipelink"
    }

    #[zbus(property)]
    #[expect(clippy::unused_self, reason = "zbus interface macro requires &self")]
    fn title(&self) -> &'static str {
        "PipeLink Audio Routing"
    }

    #[zbus(property)]
    #[expect(clippy::unused_self, reason = "zbus interface macro requires &self")]
    fn status(&self) -> &'static str {
        "Active"
    }

    #[zbus(property)]
    #[expect(clippy::unused_self, reason = "zbus interface macro requires &self")]
    fn icon_name(&self) -> &'static str {
        "pipelink"
    }

    #[zbus(property)]
    fn icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        vec![(64, 64, self.icon_argb.clone())]
    }

    #[zbus(property)]
    #[expect(clippy::unused_self, reason = "zbus interface macro requires &self")]
    fn item_is_menu(&self) -> bool {
        false
    }

    #[zbus(property)]
    #[expect(clippy::unused_self, reason = "zbus interface macro requires &self")]
    fn menu(&self) -> ObjectPath<'static> {
        ObjectPath::from_static_str_unchecked("/MenuBar")
    }

    fn activate(&self, x: i32, y: i32) {
        let _ = (x, y);
        self.controller.open_gui();
    }

    fn secondary_activate(&self, x: i32, y: i32) {
        let _ = (x, y);
        self.controller.open_gui();
    }
}

struct TrayMenu {
    controller: TrayController,
}

#[interface(name = "com.canonical.dbusmenu")]
impl TrayMenu {
    #[expect(clippy::unused_self, reason = "zbus interface macro requires &self")]
    fn version(&self) -> u32 {
        3
    }

    #[expect(clippy::unused_self, reason = "zbus interface macro requires &self")]
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
    ) -> (u32, MenuLayoutNode) {
        let _ = (recursion_depth, property_names);
        let open_item = menu_item_value(MENU_OPEN_GUI_ID, Some("Open PipeLink Mixer"), None);
        let separator = menu_item_value(MENU_SEPARATOR_ID, None, Some("separator"));
        let quit_item = menu_item_value(MENU_QUIT_DAEMON_ID, Some("Quit Daemon"), None);

        let root_children = if parent_id == 0 {
            vec![open_item, separator, quit_item]
        } else {
            Vec::new()
        };

        let mut root_props = HashMap::new();
        if let Ok(val) = OwnedValue::try_from(Value::from("submenu")) {
            root_props.insert("children-display".to_string(), val);
        }

        (1, (parent_id, root_props, root_children))
    }

    #[expect(clippy::unused_self, reason = "zbus interface macro requires &self")]
    fn about_to_show(&self, id: i32) -> bool {
        let _ = id;
        false
    }

    fn event(&self, id: i32, event_id: &str, data: OwnedValue, timestamp: u32) {
        let _ = (data, timestamp);
        if event_id != "clicked" {
            return;
        }
        match id {
            MENU_OPEN_GUI_ID => self.controller.open_gui(),
            MENU_QUIT_DAEMON_ID => self.controller.quit_daemon(),
            _ => {},
        }
    }
}

/// Runs the daemon's `StatusNotifierItem` tray service until `shutdown_rx` becomes
/// `true`, logging a non-fatal warning if D-Bus or a tray watcher is unavailable.
pub(crate) async fn run(shutdown_tx: watch::Sender<bool>, mut shutdown_rx: watch::Receiver<bool>) {
    let controller = TrayController {
        gui_child: Arc::new(Mutex::new(None)),
        shutdown_tx,
    };

    let service_name = format!("org.kde.StatusNotifierItem-{}-1", std::process::id());
    let conn = match Builder::session() {
        Ok(builder) => match builder
            .name(service_name.as_str())
            .and_then(|b| {
                b.serve_at(
                    "/StatusNotifierItem",
                    StatusNotifierItem {
                        controller: controller.clone(),
                        icon_argb: rgba_to_argb(ICON_RGBA_64),
                    },
                )
            })
            .and_then(|b| b.serve_at("/MenuBar", TrayMenu { controller }))
        {
            Ok(ready) => match ready.build().await {
                Ok(c) => c,
                Err(err) => {
                    warn!("Tray icon disabled (D-Bus connection failed): {err}");
                    return;
                },
            },
            Err(err) => {
                warn!("Tray icon disabled (D-Bus interface setup failed): {err}");
                return;
            },
        },
        Err(err) => {
            warn!("Tray icon disabled (no session D-Bus): {err}");
            return;
        },
    };

    match conn
        .call_method(
            Some("org.kde.StatusNotifierWatcher"),
            "/StatusNotifierWatcher",
            Some("org.kde.StatusNotifierWatcher"),
            "RegisterStatusNotifierItem",
            &service_name,
        )
        .await
    {
        Ok(_) => info!("Registered StatusNotifierItem tray icon ({service_name})"),
        Err(err) => warn!("StatusNotifierWatcher registration unavailable: {err}"),
    }

    while !*shutdown_rx.borrow_and_update() {
        if shutdown_rx.changed().await.is_err() {
            break;
        }
    }
}

fn rgba_to_argb(rgba: &[u8]) -> Vec<u8> {
    let mut argb = Vec::with_capacity(rgba.len());
    let (chunks, _) = rgba.as_chunks::<4>();
    for px in chunks {
        argb.push(px[3]);
        argb.push(px[0]);
        argb.push(px[1]);
        argb.push(px[2]);
    }
    argb
}

fn menu_item_value(id: i32, label: Option<&str>, item_type: Option<&str>) -> OwnedValue {
    let mut props: MenuProps = HashMap::new();
    if let Some(text) = label
        && let Ok(v) = OwnedValue::try_from(Value::from(text))
    {
        props.insert("label".to_string(), v);
    }
    if let Some(kind) = item_type
        && let Ok(v) = OwnedValue::try_from(Value::from(kind))
    {
        props.insert("type".to_string(), v);
    }
    if let Ok(v) = OwnedValue::try_from(Value::from(true)) {
        props.insert("enabled".to_string(), v.clone());
        props.insert("visible".to_string(), v);
    }

    let tuple: MenuLayoutNode = (id, props, Vec::new());
    OwnedValue::try_from(Value::from(tuple))
        .unwrap_or_else(|_| OwnedValue::from(zbus::zvariant::Str::from("")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba_to_argb_preserves_dimensions_and_reorders_channels() {
        let argb = rgba_to_argb(ICON_RGBA_64);
        assert_eq!(argb.len(), 64 * 64 * 4);
        assert_eq!(rgba_to_argb(&[10, 20, 30, 255]), vec![255, 10, 20, 30]);
    }
}

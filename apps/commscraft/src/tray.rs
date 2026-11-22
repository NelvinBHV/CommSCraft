//! The notification area icon: a StatusNotifierItem (`org.kde.StatusNotifierItem`, the
//! freedesktop StatusNotifierItem specification) with its menu over DBusMenu
//! (`com.canonical.dbusmenu`), registered at the desktop's `org.kde.StatusNotifierWatcher`.
//! KDE Plasma, Xfce, Cinnamon, MATE, Budgie, LXQt and waybar host it; GNOME with the extension
//! "AppIndicator and KStatusNotifierItem Support". Without a host there is no icon and closing
//! the window quits as before.
//!
//! The menu: open CommSCraft, the own status, notifications off, quit. The icon is the owner's
//! tray icon (`assets/branding/tray`), symbolic so the host colours it for its panel; the own
//! status is an overlay dot where the host shows overlays (the tool tip names it everywhere).
//! Hosts that cannot read the icon file get the brand icon with the dot as pixmaps.
//!
//! Security by design: the item is registered with the connection's unique name (no
//! well-known name to own, so the Flatpak needs only `--talk-name=org.kde.StatusNotifierWatcher`);
//! only fixed menu IDs are acted on, anything else from the bus is ignored; the menu carries no
//! contact data (the tool tip shows the own status only).

use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use commscraft_ui::model::Presence;
use futures_lite::StreamExt;
use zbus::object_server::SignalEmitter;
use zbus::{OwnedObjectPath, OwnedVariant as OwnedValue, Variant as Value};

const ITEM_PATH: &str = "/StatusNotifierItem";
const MENU_PATH: &str = "/MenuBar";
const WATCHER: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
/// The tray icon's name; `-symbolic`: hosts colour it like their panel.
const ICON_NAME: &str = "commscraft-tray-symbolic";
const ICON_SVG: &str = include_str!("../../../assets/branding/tray/CommSCraft-tray-symbolic.svg");

/// Writes the tray icon to `$XDG_DATA_HOME/CommSCraft/tray` (where the host, outside a sandbox
/// too, can read it) and gives that folder as the icon theme path; `None` if it cannot.
fn icon_dir() -> Option<String> {
    let dir = commscraft_client::xdg::dir(commscraft_client::xdg::Kind::Data)?.join("tray");
    std::fs::create_dir_all(&dir).ok()?;
    let file = dir.join(format!("{ICON_NAME}.svg"));
    if std::fs::read_to_string(&file).ok().as_deref() != Some(ICON_SVG) {
        std::fs::write(&file, ICON_SVG).ok()?;
    }
    dir.to_str().map(str::to_string)
}

/// What the user chose in the icon's menu (or by clicking the icon).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    Open,
    Presence(Presence),
    Mute(bool),
    Quit,
}

/// The statuses the menu offers, as the Windows client's status menu.
pub const STATUSES: [Presence; 6] = [
    Presence::Available,
    Presence::Busy,
    Presence::DoNotDisturb,
    Presence::BeRightBack,
    Presence::Away,
    Presence::Offline,
];

/// What the icon and its menu show.
#[derive(Debug, Clone, PartialEq)]
pub struct Shown {
    pub presence: Presence,
    pub muted: bool,
    pub signed_in: bool,
    /// Labels: open, the statuses (in [`STATUSES`] order), notifications off, quit.
    pub open: String,
    pub statuses: Vec<String>,
    pub mute: String,
    pub quit: String,
    /// The tool tip's text (the own status).
    pub status_text: String,
}

/// A running icon: what the user chose, and updates of what it shows.
pub struct Tray {
    events: Mutex<Option<Receiver<TrayEvent>>>,
    updates: Mutex<Sender<Shown>>,
}

impl Tray {
    /// Registers the icon; `None` without a host (no watcher on the session bus).
    pub fn start(shown: Shown, icon: Option<(u32, u32, Vec<u8>)>) -> Option<Tray> {
        let (ev_tx, events) = mpsc::channel();
        let (updates, up_rx) = mpsc::channel::<Shown>();
        let base = icon.map(|(w, h, rgba)| Icon { w, h, rgba });
        let conn = futures_lite::future::block_on(register(shown.clone(), base.clone(), ev_tx))?;
        let c1 = conn.clone();
        std::thread::Builder::new()
            .name("tray".into())
            .spawn(move || {
                for s in up_rx {
                    futures_lite::future::block_on(update(&c1, s, base.as_ref()));
                }
            })
            .ok()?;
        std::thread::Builder::new()
            .name("tray-watcher".into())
            .spawn(move || futures_lite::future::block_on(watch(&conn)))
            .ok()?;
        Some(Tray {
            events: Mutex::new(Some(events)),
            updates: Mutex::new(updates),
        })
    }

    /// What the user chooses, for a thread of its own (the window may draw no frames while it
    /// is covered or closed); `None` after the first call.
    pub fn take_events(&self) -> Option<Receiver<TrayEvent>> {
        let mut slot = self.events.lock().ok()?;
        slot.take()
    }

    pub fn show(&self, s: Shown) {
        if let Ok(tx) = self.updates.lock() {
            let _ = tx.send(s);
        }
    }
}

/// The brand icon as RGBA.
#[derive(Clone)]
struct Icon {
    w: u32,
    h: u32,
    rgba: Vec<u8>,
}

/// The colour of the status dot (RGB): green available, red busy, yellow away, grey otherwise.
fn dot_colour(p: Presence) -> [u8; 3] {
    match p {
        Presence::Available => [0x6b, 0xb7, 0x00],
        Presence::Busy | Presence::InACall | Presence::DoNotDisturb => [0xc4, 0x31, 0x4b],
        Presence::BeRightBack | Presence::Inactive | Presence::Away | Presence::OffWork => {
            [0xf8, 0xc4, 0x00]
        }
        Presence::Offline | Presence::Unknown => [0x8a, 0x88, 0x86],
    }
}

/// The icon in the sizes hosts ask for, as `IconPixmap` wants it: ARGB32 in network byte
/// order, with the status dot at the lower right (none while signed out).
fn pixmaps(base: &Icon, presence: Option<Presence>) -> Vec<(i32, i32, Vec<u8>)> {
    [22u32, 32, 48, 64]
        .iter()
        .filter_map(|&size| {
            let mut rgba = scale(base, size)?;
            if let Some(p) = presence {
                dot(&mut rgba, size, dot_colour(p), p == Presence::DoNotDisturb);
            }
            let argb: Vec<u8> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|&[r, g, b, a]| [a, r, g, b])
                .collect();
            let s = i32::try_from(size).ok()?;
            Some((s, s, argb))
        })
        .collect()
}

/// The status dot alone, on a transparent square (the overlay hosts draw over the icon).
fn dots(p: Presence) -> Vec<Pixmap> {
    [16u32, 22, 32]
        .iter()
        .filter_map(|&size| {
            let mut rgba = vec![0u8; (size * size * 4) as usize];
            dot(&mut rgba, size, dot_colour(p), p == Presence::DoNotDisturb);
            let argb: Vec<u8> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .flat_map(|&[r, g, b, a]| [a, r, g, b])
                .collect();
            let s = i32::try_from(size).ok()?;
            Some((s, s, argb))
        })
        .collect()
}

/// Box-filtered downscale of the square brand icon to `size` × `size`.
fn scale(base: &Icon, size: u32) -> Option<Vec<u8>> {
    if base.w == 0 || base.h == 0 || size == 0 || base.rgba.len() != (base.w * base.h * 4) as usize
    {
        return None;
    }
    let mut out = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let (x0, x1) = (
                x * base.w / size,
                ((x + 1) * base.w / size).max(x * base.w / size + 1),
            );
            let (y0, y1) = (
                y * base.h / size,
                ((y + 1) * base.h / size).max(y * base.h / size + 1),
            );
            let mut acc = [0u32; 4];
            let mut n = 0u32;
            for sy in y0..y1.min(base.h) {
                for sx in x0..x1.min(base.w) {
                    let i = ((sy * base.w + sx) * 4) as usize;
                    let a = u32::from(*base.rgba.get(i + 3)?);
                    for (c, v) in acc.iter_mut().take(3).enumerate() {
                        *v += u32::from(*base.rgba.get(i + c)?) * a;
                    }
                    acc[3] += a;
                    n += 1;
                }
            }
            let o = ((y * size + x) * 4) as usize;
            let [r, g, b, a] = acc;
            let channel = |v: u32| {
                v.checked_div(a)
                    .and_then(|c| u8::try_from(c).ok())
                    .unwrap_or(0)
            };
            let alpha = u8::try_from(a / n.max(1)).unwrap_or(255);
            out.get_mut(o..o + 4)?
                .copy_from_slice(&[channel(r), channel(g), channel(b), alpha]);
        }
    }
    Some(out)
}

/// A filled dot with a white ring at the lower right; Do Not Disturb gets a white bar.
fn dot(rgba: &mut [u8], size: u32, rgb: [u8; 3], bar: bool) {
    let r = size as f32 * 0.22;
    let (cx, cy) = (size as f32 - r - 0.5, size as f32 - r - 0.5);
    for y in 0..size {
        for x in 0..size {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let d = (dx * dx + dy * dy).sqrt();
            let colour = if d <= r - 1.2 {
                if bar && dy.abs() <= r * 0.18 && dx.abs() <= r * 0.55 {
                    [255, 255, 255]
                } else {
                    rgb
                }
            } else if d <= r {
                [255, 255, 255]
            } else {
                continue;
            };
            let o = ((y * size + x) * 4) as usize;
            if let Some(px) = rgba.get_mut(o..o + 4) {
                let [r, g, b] = colour;
                px.copy_from_slice(&[r, g, b, 255]);
            }
        }
    }
}

/// A pixmap of `IconPixmap`: width, height, ARGB32 bytes.
type Pixmap = (i32, i32, Vec<u8>);
/// `ToolTip`: icon name, icon pixmaps, title, text.
type ToolTip = (String, Vec<Pixmap>, String, String);

struct Item {
    icon: Vec<(i32, i32, Vec<u8>)>,
    /// The status dot alone (overlay).
    overlay: Vec<Pixmap>,
    /// The folder holding the tray icon; empty: pixmaps only.
    theme_path: String,
    tip: String,
    events: Sender<TrayEvent>,
}

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    #[zbus(property)]
    fn category(&self) -> &str {
        "Communications"
    }
    #[zbus(property)]
    fn id(&self) -> &str {
        "commscraft"
    }
    #[zbus(property)]
    fn title(&self) -> &str {
        "CommSCraft"
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "Active"
    }
    #[zbus(property)]
    fn window_id(&self) -> i32 {
        0
    }
    #[zbus(property)]
    fn icon_name(&self) -> &str {
        if self.theme_path.is_empty() {
            ""
        } else {
            ICON_NAME
        }
    }
    #[zbus(property)]
    fn icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        self.icon.clone()
    }
    #[zbus(property)]
    fn overlay_icon_name(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn attention_icon_name(&self) -> &str {
        ""
    }
    #[zbus(property)]
    fn icon_theme_path(&self) -> &str {
        &self.theme_path
    }
    #[zbus(property)]
    fn overlay_icon_pixmap(&self) -> Vec<Pixmap> {
        self.overlay.clone()
    }
    #[zbus(property)]
    fn tool_tip(&self) -> ToolTip {
        (
            String::new(),
            Vec::new(),
            "CommSCraft".into(),
            self.tip.clone(),
        )
    }
    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn menu(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from(MENU_PATH).unwrap_or_default()
    }

    /// A click on the icon opens the main window.
    fn activate(&self, _x: i32, _y: i32) {
        let _ = self.events.send(TrayEvent::Open);
    }
    fn secondary_activate(&self, _x: i32, _y: i32) {
        let _ = self.events.send(TrayEvent::Open);
    }
    fn context_menu(&self, _x: i32, _y: i32) {}
    fn scroll(&self, _delta: i32, _orientation: &str) {}

    #[zbus(signal)]
    async fn new_icon(e: &SignalEmitter<'_>) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn new_tool_tip(e: &SignalEmitter<'_>) -> zbus::Result<()>;
    #[zbus(signal)]
    async fn new_overlay_icon(e: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Menu item IDs (fixed; nothing else is acted on).
const ID_OPEN: i32 = 1;
const ID_MUTE: i32 = 2;
const ID_QUIT: i32 = 3;
const ID_STATUS: i32 = 10;
const ID_SEP: i32 = 100;

/// One menu entry: its ID and DBusMenu properties.
type Entry = (i32, HashMap<String, OwnedValue>);

fn value<'a>(v: impl Into<Value<'a>>) -> Option<OwnedValue> {
    OwnedValue::try_from(v.into()).ok()
}

/// The entries of the menu, in order.
fn entries(s: &Shown) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut push = |id: i32, props: Vec<(&str, Option<OwnedValue>)>| {
        let map = props
            .into_iter()
            .filter_map(|(k, v)| Some((k.to_string(), v?)))
            .collect();
        out.push((id, map));
    };
    push(ID_OPEN, vec![("label", value(s.open.as_str()))]);
    push(ID_SEP, vec![("type", value("separator"))]);
    for (i, (p, label)) in STATUSES.iter().zip(&s.statuses).enumerate() {
        let id = ID_STATUS + i32::try_from(i).unwrap_or(0);
        push(
            id,
            vec![
                ("label", value(label.as_str())),
                ("enabled", value(s.signed_in)),
                ("toggle-type", value("radio")),
                (
                    "toggle-state",
                    value(i32::from(s.signed_in && *p == s.presence)),
                ),
            ],
        );
    }
    push(ID_SEP + 1, vec![("type", value("separator"))]);
    push(
        ID_MUTE,
        vec![
            ("label", value(s.mute.as_str())),
            ("toggle-type", value("checkmark")),
            ("toggle-state", value(i32::from(s.muted))),
        ],
    );
    push(ID_SEP + 2, vec![("type", value("separator"))]);
    push(ID_QUIT, vec![("label", value(s.quit.as_str()))]);
    out
}

/// What a click on a menu entry means.
fn chosen(id: i32, s: &Shown) -> Option<TrayEvent> {
    match id {
        ID_OPEN => Some(TrayEvent::Open),
        ID_MUTE => Some(TrayEvent::Mute(!s.muted)),
        ID_QUIT => Some(TrayEvent::Quit),
        _ => {
            let i = usize::try_from(id.checked_sub(ID_STATUS)?).ok()?;
            STATUSES
                .get(i)
                .filter(|_| s.signed_in)
                .map(|p| TrayEvent::Presence(*p))
        }
    }
}

struct Menu {
    shown: Shown,
    revision: u32,
    events: Sender<TrayEvent>,
}

type Layout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

#[zbus::interface(name = "com.canonical.dbusmenu")]
impl Menu {
    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }
    #[zbus(property)]
    fn text_direction(&self) -> &str {
        "ltr"
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "normal"
    }
    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }

    /// The whole menu (it is one level): the root with its entries as children.
    fn get_layout(
        &self,
        parent_id: i32,
        _recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> (u32, Layout) {
        let children: Vec<OwnedValue> = if parent_id == 0 {
            entries(&self.shown)
                .into_iter()
                .filter_map(|(id, props)| value((id, props, Vec::<OwnedValue>::new())))
                .collect()
        } else {
            Vec::new()
        };
        let mut root = HashMap::new();
        if let Some(v) = value("submenu") {
            root.insert("children-display".to_string(), v);
        }
        (self.revision, (parent_id, root, children))
    }

    fn get_group_properties(
        &self,
        ids: Vec<i32>,
        _property_names: Vec<String>,
    ) -> Vec<(i32, HashMap<String, OwnedValue>)> {
        entries(&self.shown)
            .into_iter()
            .filter(|(id, _)| ids.is_empty() || ids.contains(id))
            .collect()
    }

    fn get_property(&self, id: i32, name: &str) -> zbus::fdo::Result<OwnedValue> {
        entries(&self.shown)
            .into_iter()
            .find(|(i, _)| *i == id)
            .and_then(|(_, mut p)| p.remove(name))
            .ok_or_else(|| zbus::fdo::Error::InvalidArgs("no such property".into()))
    }

    fn event(&self, id: i32, event_id: &str, _data: OwnedValue, _timestamp: u32) {
        if event_id == "clicked" {
            if let Some(e) = chosen(id, &self.shown) {
                let _ = self.events.send(e);
            }
        }
    }

    fn event_group(&self, events: Vec<(i32, String, OwnedValue, u32)>) -> Vec<i32> {
        for (id, event_id, _, _) in events.iter().take(16) {
            if event_id == "clicked" {
                if let Some(e) = chosen(*id, &self.shown) {
                    let _ = self.events.send(e);
                }
            }
        }
        Vec::new()
    }

    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }

    #[zbus(signal)]
    async fn layout_updated(e: &SignalEmitter<'_>, revision: u32, parent: i32) -> zbus::Result<()>;
}

async fn register(
    shown: Shown,
    base: Option<Icon>,
    events: Sender<TrayEvent>,
) -> Option<zbus::Connection> {
    let presence = shown.signed_in.then_some(shown.presence);
    let item = Item {
        icon: base
            .as_ref()
            .map(|b| pixmaps(b, presence))
            .unwrap_or_default(),
        tip: shown.status_text.clone(),
        overlay: presence.map(dots).unwrap_or_default(),
        theme_path: icon_dir().unwrap_or_default(),
        events: events.clone(),
    };
    let menu = Menu {
        shown,
        revision: 1,
        events,
    };
    let conn = zbus::connection::Builder::session()
        .serve_at(ITEM_PATH, item)
        .serve_at(MENU_PATH, menu)
        .build()
        .await
        .ok()?;
    announce(&conn).await.then_some(conn)
}

/// Registers the item at the watcher with the connection's unique name.
async fn announce(conn: &zbus::Connection) -> bool {
    let Some(name) = conn.unique_name().map(|n| n.to_string()) else {
        return false;
    };
    conn.call_method(
        Some(WATCHER),
        WATCHER_PATH,
        Some(WATCHER),
        "RegisterStatusNotifierItem",
        &(name.as_str(),),
    )
    .await
    .is_ok()
}

/// A new watcher (the desktop shell restarted) gets the item again.
async fn watch(conn: &zbus::Connection) {
    let Ok(dbus) = zbus::fdo::DBusProxy::new(conn).await else {
        return;
    };
    let Ok(mut changes) = dbus.receive_name_owner_changed().await else {
        return;
    };
    while let Some(c) = changes.next().await {
        let Ok(args) = c.args() else { continue };
        if args.name().as_str() == WATCHER && args.new_owner().is_some() {
            let _ = announce(conn).await;
        }
    }
}

async fn update(conn: &zbus::Connection, s: Shown, base: Option<&Icon>) {
    let server = conn.object_server();
    if let Ok(item) = server.interface::<_, Item>(ITEM_PATH).await {
        {
            let mut i = item.get_mut().await;
            i.icon = base
                .map(|b| pixmaps(b, s.signed_in.then_some(s.presence)))
                .unwrap_or_default();
            i.overlay = if s.signed_in {
                dots(s.presence)
            } else {
                Vec::new()
            };
            i.tip = s.status_text.clone();
        }
        let _ = Item::new_icon(item.signal_emitter()).await;
        let _ = Item::new_overlay_icon(item.signal_emitter()).await;
        let _ = Item::new_tool_tip(item.signal_emitter()).await;
    }
    if let Ok(menu) = server.interface::<_, Menu>(MENU_PATH).await {
        let revision = {
            let mut m = menu.get_mut().await;
            if m.shown == s {
                return;
            }
            m.shown = s;
            m.revision = m.revision.wrapping_add(1);
            m.revision
        };
        let _ = Menu::layout_updated(menu.signal_emitter(), revision, 0).await;
    }
}

/// Shared handle as the app keeps it.
pub type Shared = Arc<Tray>;

#[cfg(test)]
mod tests {
    use super::*;

    fn shown() -> Shown {
        Shown {
            presence: Presence::Busy,
            muted: false,
            signed_in: true,
            open: "Open".into(),
            statuses: STATUSES.iter().map(|p| format!("{p:?}")).collect(),
            mute: "Mute".into(),
            quit: "Quit".into(),
            status_text: "Busy".into(),
        }
    }

    #[test]
    fn menu_ids_map_to_choices() {
        let s = shown();
        assert_eq!(chosen(ID_OPEN, &s), Some(TrayEvent::Open));
        assert_eq!(chosen(ID_MUTE, &s), Some(TrayEvent::Mute(true)));
        assert_eq!(chosen(ID_QUIT, &s), Some(TrayEvent::Quit));
        assert_eq!(
            chosen(ID_STATUS + 2, &s),
            Some(TrayEvent::Presence(Presence::DoNotDisturb))
        );
        assert_eq!(chosen(ID_STATUS + 6, &s), None);
        assert_eq!(chosen(-5, &s), None);
        let out = Shown {
            signed_in: false,
            ..s
        };
        assert_eq!(chosen(ID_STATUS, &out), None);
    }

    #[test]
    fn entries_mark_the_own_status() {
        let e = entries(&shown());
        let busy = e.iter().find(|(id, _)| *id == ID_STATUS + 1).unwrap();
        let state = busy.1.get("toggle-state").unwrap();
        assert_eq!(i32::try_from(state).unwrap(), 1);
        assert_eq!(e.len(), 12);
    }

    #[test]
    fn the_tray_icon_is_symbolic_and_the_dot_overlay_has_the_status_colour() {
        assert!(ICON_SVG.contains("fill=\"currentColor\""));
        assert!(ICON_NAME.ends_with("-symbolic"));
        let d = dots(Presence::Busy);
        assert_eq!(d.len(), 3);
        let (_, _, px) = &d[1];
        // Upper left transparent, lower right the red dot.
        assert_eq!(px[0], 0);
        let c = ((22 - 5) * 22 + (22 - 5)) * 4;
        assert_eq!(&px[c..c + 4], &[255, 0xc4, 0x31, 0x4b]);
    }

    #[test]
    fn pixmaps_are_argb_in_four_sizes() {
        let base = Icon {
            w: 128,
            h: 128,
            rgba: [10u8, 20, 30, 255].repeat(128 * 128),
        };
        let p = pixmaps(&base, Some(Presence::Available));
        assert_eq!(
            p.iter().map(|x| x.0).collect::<Vec<_>>(),
            vec![22, 32, 48, 64]
        );
        let (_, _, px) = &p[0];
        assert_eq!(px.len(), 22 * 22 * 4);
        // Upper left: the icon (alpha first); lower right: the green dot.
        assert_eq!(&px[..4], &[255, 10, 20, 30]);
        let last = 11 * 22 * 4;
        assert_eq!(px[last], 255);
        let c = ((22 - 5) * 22 + (22 - 5)) * 4;
        assert_eq!(&px[c..c + 4], &[255, 0x6b, 0xb7, 0x00]);
    }
}

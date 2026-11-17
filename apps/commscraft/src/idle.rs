//! How long the user has not touched keyboard or mouse in the desktop session, for the status
//! automation (inactive, then away – [MS-PRES] §2.2.2.7.1: the client watches keyboard and mouse
//! for its machine state). Asked over D-Bus of the session: GNOME's
//! `org.gnome.Mutter.IdleMonitor.GetIdletime`, else `org.freedesktop.ScreenSaver.
//! GetSessionIdleTime` (KDE and others); both answer milliseconds. Without either the
//! automation stays off (nothing is guessed).
//!
//! Security by design: a thread of its own asks every [`EVERY`] seconds and keeps only a
//! number; nothing is sent anywhere else.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Seconds between two questions.
const EVERY: u64 = 10;
/// No answer: the idle time is not known.
pub const UNKNOWN: u64 = u64::MAX;

/// The idle seconds as last asked, shared with the interface thread.
#[derive(Clone)]
pub struct Idle(Arc<AtomicU64>);

impl Idle {
    pub fn start() -> Idle {
        let idle = Idle(Arc::new(AtomicU64::new(UNKNOWN)));
        let shared = idle.0.clone();
        let _ = std::thread::Builder::new()
            .name("idle".into())
            .spawn(move || {
                let conn = futures_lite::future::block_on(zbus::Connection::session()).ok();
                loop {
                    let secs = conn
                        .as_ref()
                        .and_then(|c| futures_lite::future::block_on(idle_ms(c)))
                        .map_or(UNKNOWN, |ms| ms / 1000);
                    shared.store(secs, Ordering::Relaxed);
                    std::thread::sleep(Duration::from_secs(EVERY));
                }
            });
        idle
    }

    /// Idle seconds, if the desktop tells them.
    pub fn seconds(&self) -> Option<u64> {
        Some(self.0.load(Ordering::Relaxed)).filter(|s| *s != UNKNOWN)
    }
}

async fn idle_ms(c: &zbus::Connection) -> Option<u64> {
    if let Ok(m) = c
        .call_method(
            Some("org.gnome.Mutter.IdleMonitor"),
            "/org/gnome/Mutter/IdleMonitor/Core",
            Some("org.gnome.Mutter.IdleMonitor"),
            "GetIdletime",
            &(),
        )
        .await
    {
        if let Ok(ms) = m.body().deserialize::<u64>() {
            return Some(ms);
        }
    }
    let m = c
        .call_method(
            Some("org.freedesktop.ScreenSaver"),
            "/org/freedesktop/ScreenSaver",
            Some("org.freedesktop.ScreenSaver"),
            "GetSessionIdleTime",
            &(),
        )
        .await
        .ok()?;
    m.body().deserialize::<u32>().ok().map(u64::from)
}

/// The activity for an idle time and the user's limits in minutes (0: that step is off).
pub fn activity(
    idle_secs: Option<u64>,
    inactive_min: u32,
    away_min: u32,
) -> commscraft_presence::MachineActivity {
    use commscraft_presence::MachineActivity as A;
    let Some(s) = idle_secs else {
        return A::Active;
    };
    let inactive = u64::from(inactive_min) * 60;
    let away = inactive + u64::from(away_min) * 60;
    if inactive_min > 0 && away_min > 0 && s >= away {
        A::Away
    } else if inactive_min > 0 && s >= inactive {
        A::Idle
    } else {
        A::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use commscraft_presence::MachineActivity as A;

    #[test]
    fn idle_limits() {
        assert_eq!(activity(None, 5, 5), A::Active);
        assert_eq!(activity(Some(299), 5, 5), A::Active);
        assert_eq!(activity(Some(300), 5, 5), A::Idle);
        assert_eq!(activity(Some(600), 5, 5), A::Away);
        // Off: never idle; no away step: idle stays idle.
        assert_eq!(activity(Some(10_000), 0, 5), A::Active);
        assert_eq!(activity(Some(10_000), 5, 0), A::Idle);
    }
}

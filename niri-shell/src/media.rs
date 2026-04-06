// MPRIS media backend — polls the active MPRIS2 player via D-Bus.
// Zero GTK4 imports. Call `poll_media()` from the panel's 2-second timer.

use thiserror::Error;

#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum MediaError {
    #[error("D-Bus error: {0}")]
    Dbus(String),
    #[error("no MPRIS player active")]
    NoPlayer,
}

/// Current media playback state, ready for display.
#[derive(Debug, Clone, Default)]
pub struct MediaSnapshot {
    pub artist: String,
    pub title: String,
    pub playing: bool,
}

/// Synchronously poll the first active MPRIS2 player.
/// Returns `None` when no player is found (treated as "no media").
pub fn poll_media() -> Option<MediaSnapshot> {
    // Use a blocking zbus connection.
    let conn = zbus::blocking::Connection::session().ok()?;

    // List all names on the session bus and find MPRIS players.
    let dbus = zbus::blocking::fdo::DBusProxy::new(&conn).ok()?;
    let names = dbus.list_names().ok()?;

    let player_name = names
        .iter()
        .find(|n| n.starts_with("org.mpris.MediaPlayer2."))?
        .clone();

    let proxy = zbus::blocking::Proxy::new(
        &conn,
        player_name,
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    )
    .ok()?;

    // PlaybackStatus: "Playing", "Paused", "Stopped"
    let status: String = proxy.get_property("PlaybackStatus").ok()?;
    let playing = status == "Playing";

    // Metadata is a dict — extract xesam:title and xesam:artist.
    let metadata: std::collections::HashMap<String, zbus::zvariant::OwnedValue> =
        proxy.get_property("Metadata").ok()?;

    let title = extract_string(&metadata, "xesam:title").unwrap_or_default();
    let artist = extract_artist(&metadata).unwrap_or_default();

    if title.is_empty() && artist.is_empty() {
        return None;
    }

    Some(MediaSnapshot { artist, title, playing })
}

fn extract_string(
    map: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    key: &str,
) -> Option<String> {
    let val = map.get(key)?;
    // The value is a Str variant.
    if let Ok(s) = <&str>::try_from(&**val) {
        return Some(s.to_owned());
    }
    None
}

fn extract_artist(
    map: &std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
) -> Option<String> {
    let val = map.get("xesam:artist")?;
    // xesam:artist is an array of strings; take the first.
    if let Ok(arr) = <zbus::zvariant::Array>::try_from(&**val) {
        if let Some(first) = arr.iter().next() {
            if let Ok(s) = <&str>::try_from(first) {
                return Some(s.to_owned());
            }
        }
    }
    // Fallback: try as plain string.
    extract_string(map, "xesam:artist")
}

// ── Playback control ───────────────────────────────────────────────────────────

/// Send a playback command to the first active MPRIS2 player.
/// Called from a background thread — all D-Bus work is blocking.
fn send_mpris_command(method: &str) {
    let conn = match zbus::blocking::Connection::session() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("media: D-Bus session failed: {e}");
            return;
        }
    };
    let dbus = match zbus::blocking::fdo::DBusProxy::new(&conn) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("media: DBusProxy failed: {e}");
            return;
        }
    };
    let names = match dbus.list_names() {
        Ok(n) => n,
        Err(e) => {
            log::warn!("media: list_names failed: {e}");
            return;
        }
    };
    let Some(player_name) = names.iter().find(|n| n.starts_with("org.mpris.MediaPlayer2.")) else {
        log::warn!("media: {method} — no MPRIS player active");
        return;
    };
    let proxy = match zbus::blocking::Proxy::new(
        &conn,
        player_name.clone(),
        "/org/mpris/MediaPlayer2",
        "org.mpris.MediaPlayer2.Player",
    ) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("media: proxy create failed: {e}");
            return;
        }
    };
    if let Err(e) = proxy.call::<_, _, ()>(method, &()) {
        log::warn!("media: {method} call failed: {e}");
    }
}

pub fn send_play_pause() {
    send_mpris_command("PlayPause");
}

pub fn send_previous() {
    send_mpris_command("Previous");
}

pub fn send_next() {
    send_mpris_command("Next");
}

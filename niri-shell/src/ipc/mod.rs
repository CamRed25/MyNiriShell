// IPC client for the niri compositor.
// Connects to $NIRI_SOCKET, subscribes to the event stream, and provides
// a one-shot action sender for dock/launcher interactions.

pub mod types;

use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc;
use std::time::Duration;

use types::{NiriEvent, NiriReply, NiriRequest};

use crate::error::IpcError;

// ── Event stream ──────────────────────────────────────────────────────────────

/// Handle to a running niri event stream subscription.
/// Call [`IpcEventStream::attach`] to start receiving events on the GTK main thread.
pub struct IpcEventStream {
    receiver: mpsc::Receiver<NiriEvent>,
}

impl IpcEventStream {
    /// Open the niri socket, issue an `EventStream` subscription, and spawn a
    /// background reader thread that forwards events via an mpsc channel.
    pub fn connect() -> Result<Self, IpcError> {
        let socket_path = env::var("NIRI_SOCKET").map_err(|_| IpcError::SocketEnvMissing)?;
        let stream = UnixStream::connect(&socket_path).map_err(IpcError::Connect)?;

        // Clone before moving into BufReader so we can write the subscription request.
        let write_half = stream.try_clone().map_err(IpcError::Connect)?;

        // Send the EventStream subscription request.
        let req = serde_json::to_string(&NiriRequest::EventStream)
            .map_err(|e| IpcError::Send(e.to_string()))?;
        {
            let mut w = write_half;
            writeln!(w, "{req}").map_err(|e| IpcError::Send(e.to_string()))?;
        }

        // Read and validate the initial reply.
        let mut reader = BufReader::new(stream);
        let mut reply_line = String::new();
        reader
            .read_line(&mut reply_line)
            .map_err(|e| IpcError::Recv(e.to_string()))?;

        let reply: NiriReply = serde_json::from_str(reply_line.trim())
            .map_err(|e| IpcError::Parse(e.to_string()))?;
        if let NiriReply::Err(msg) = reply {
            return Err(IpcError::Recv(msg));
        }

        // Spawn a background reader; forward events over an mpsc channel.
        let (sender, receiver) = mpsc::channel::<NiriEvent>();

        std::thread::spawn(move || {
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(e) => {
                        log::error!("niri IPC read error: {e}");
                        break;
                    }
                };

                match serde_json::from_str::<NiriEvent>(&line) {
                    Ok(event) => {
                        if sender.send(event).is_err() {
                            // Receiver dropped — shell is shutting down.
                            break;
                        }
                    }
                    Err(e) => {
                        log::debug!("niri IPC: unrecognised event skipped ({e}): {line}");
                    }
                }
            }
            log::info!("niri IPC event stream closed.");
        });

        Ok(Self { receiver })
    }

    /// Attach a callback that runs on the GTK main thread for every incoming event.
    /// Events are flushed every 50 ms via a `glib::timeout_add_local` source.
    pub fn attach<F>(self, callback: F)
    where
        F: Fn(NiriEvent) -> glib::ControlFlow + 'static,
    {
        let receiver = self.receiver;
        glib::timeout_add_local(Duration::from_millis(50), move || {
            loop {
                match receiver.try_recv() {
                    Ok(event) => {
                        if callback(event) == glib::ControlFlow::Break {
                            return glib::ControlFlow::Break;
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        log::warn!("niri IPC channel disconnected.");
                        return glib::ControlFlow::Break;
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }
}



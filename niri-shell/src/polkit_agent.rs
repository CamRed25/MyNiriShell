//! Polkit Authentication Agent backend
// Registers org.freedesktop.PolicyKit1.AuthenticationAgent on the session bus
// and forwards InitiateAuthentication to the GTK dialog via a oneshot channel.

use std::sync::Arc;
use std::sync::mpsc;
use tokio::sync::oneshot;
use zbus::Connection;
use zbus::names::WellKnownName;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PolkitAgentError {
    #[error("Zbus error: {0}")]
    Zbus(#[from] zbus::Error),
}

/// Authentication request forwarded to the GTK dialog.
/// Send `Some(password)` through `response_tx` to authenticate, `None` to cancel.
#[derive(Debug)]
#[allow(dead_code)] // all fields are part of the D-Bus protocol; UI consumes a subset
pub struct PolkitRequest {
    pub action_id: String,
    pub message: String,
    pub details: Vec<(String, String)>,
    pub cookie: String,
    pub identities: Vec<String>,
    pub response_tx: oneshot::Sender<Option<String>>,
}

pub struct PolkitAgent {
    tx: mpsc::Sender<PolkitRequest>,
}

impl PolkitAgent {
    pub fn new(tx: mpsc::Sender<PolkitRequest>) -> Self {
        Self { tx }
    }

    pub async fn run(self: Arc<Self>) -> Result<(), PolkitAgentError> {
        let conn = Connection::session().await?;
        conn.object_server()
            .at(
                "/org/freedesktop/PolicyKit1/AuthenticationAgent",
                PolkitAgentIface::new(self.tx.clone()),
            )
            .await?;
        let name = WellKnownName::try_from("org.freedesktop.PolicyKit1.AuthenticationAgent")
            .map_err(|e| zbus::Error::Failure(e.to_string()))?;
        conn.request_name(name).await?;
        futures::future::pending::<()>().await;
        Ok(())
    }
}

/// Notify PolicyKit that authentication succeeded for the given cookie.
/// TODO: verify password via PAM before calling this.
async fn send_auth_response(cookie: &str) -> Result<(), zbus::Error> {
    let conn = Connection::system().await?;
    let uid = unsafe { libc::getuid() };
    // Identity: ("unix-user", {"uid": variant<u32>})
    let mut details: std::collections::HashMap<String, zbus::zvariant::Value<'static>> =
        std::collections::HashMap::new();
    details.insert("uid".to_string(), zbus::zvariant::Value::U32(uid));
    conn.call_method(
        Some("org.freedesktop.PolicyKit1"),
        "/org/freedesktop/PolicyKit1/Authority",
        Some("org.freedesktop.PolicyKit1.Authority"),
        "AuthenticationAgentResponse2",
        &(uid, cookie, ("unix-user".to_string(), details)),
    )
    .await?;
    Ok(())
}

pub struct PolkitAgentIface {
    tx: mpsc::Sender<PolkitRequest>,
}

impl PolkitAgentIface {
    pub fn new(tx: mpsc::Sender<PolkitRequest>) -> Self {
        Self { tx }
    }
}

#[zbus::interface(name = "org.freedesktop.PolicyKit1.AuthenticationAgent")]
impl PolkitAgentIface {
    async fn initiate_authentication(
        &self,
        action_id: &str,
        message: &str,
        details: Vec<(String, String)>,
        cookie: &str,
        identities: Vec<String>,
    ) -> zbus::fdo::Result<()> {
        let (response_tx, response_rx) = oneshot::channel();
        let req = PolkitRequest {
            action_id: action_id.to_owned(),
            message: message.to_owned(),
            details,
            cookie: cookie.to_owned(),
            identities,
            response_tx,
        };
        if self.tx.send(req).is_err() {
            return Err(zbus::fdo::Error::Failed("polkit channel closed".into()));
        }
        let cookie_owned = cookie.to_owned();
        match response_rx.await {
            Ok(Some(_password)) => {
                // TODO: PAM-verify _password here before responding to polkit
                if let Err(e) = send_auth_response(&cookie_owned).await {
                    log::warn!("polkit AuthenticationAgentResponse2 failed: {e}");
                }
            }
            Ok(None) => {
                log::info!("polkit authentication cancelled by user");
            }
            Err(_) => {
                log::warn!("polkit response channel dropped without response");
            }
        }
        Ok(())
    }
}

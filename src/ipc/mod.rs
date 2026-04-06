use log::error;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[cfg(windows)]
use crate::parameters::DaemonType;
use crate::{config, outcome::Outcome, parameters::DaemonParameters, shiplog};

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;
mod zenoh;

pub use zenoh::{Rx, ZenohClient, ZenohMessage, TOPIC_CLIENTS, TOPIC_CONTROL_RELOAD, TOPIC_SERVER};

pub fn terminal_topic() -> Outcome<String> {
    Ok(format!("sinkd/{}/terminate", config::get_hostname()?))
}

pub fn send_terminate_signal() -> Outcome<()> {
    let topic = terminal_topic()?;
    let (client, _rx) = ZenohClient::new(&[], &topic)
        .map_err(|e| format!("failed to create Zenoh client for termination: {e}"))?;
    let mut payload = Payload::new()?.status(Status::NotReady(Reason::Other));
    let publish_outcome = client.publish(&mut payload);
    client.disconnect();
    publish_outcome.map_err(|e| {
        error!("failed to send terminate message: {e}");
        e
    })?;
    Ok(())
}

pub fn connect_with_terminate_topic(
    subscriptions: &[&str],
    publish_topic: &str,
) -> Outcome<(ZenohClient, Rx, String)> {
    let terminal = terminal_topic()?;
    let mut all_subscriptions = subscriptions.to_vec();
    all_subscriptions.push(TOPIC_CONTROL_RELOAD);
    all_subscriptions.push(&terminal);

    let (client, rx) = ZenohClient::new(&all_subscriptions, publish_topic)
        .map_err(|e| format!("unable to create Zenoh client: {e}"))?;

    Ok((client, rx, terminal))
}

/// Best-effort signal to running clients that configuration files changed (see `TOPIC_CONTROL_RELOAD`).
pub fn publish_config_reload_signal() -> Outcome<()> {
    let (client, _rx) = ZenohClient::new(&[], TOPIC_CONTROL_RELOAD)
        .map_err(|e| format!("failed to create Zenoh client for reload signal: {e}"))?;
    let mut payload = Payload::new()?;
    let r = client.publish(&mut payload);
    client.disconnect();
    r
}

pub fn daemon(params: &DaemonParameters) -> Outcome<()> {
    #[cfg(unix)]
    {
        match params {
            DaemonParameters::Client(p) => {
                let p = p.clone();
                unix::daemon(|| {
                    shiplog::init(&p.shared)?;
                    crate::client::init(&p)
                })
            }
            DaemonParameters::Server(p) => {
                let p = p.clone();
                unix::daemon(|| {
                    shiplog::init(&p.shared)?;
                    crate::server::init(&p)
                })
            }
        }
    }
    #[cfg(windows)]
    {
        match params.shared().daemon_type {
            DaemonType::WindowsClient => {
                let DaemonParameters::Client(p) = params else {
                    return bad!("expected client parameters for Windows client");
                };
                windows::redirect_stdio_to_null()?;
                shiplog::init(&p.shared)?;
                crate::client::init(p)
            }
            DaemonType::WindowsServer => {
                let DaemonParameters::Server(p) = params else {
                    return bad!("expected server parameters for Windows server");
                };
                windows::redirect_stdio_to_null()?;
                shiplog::init(&p.shared)?;
                crate::server::init(p)
            }
            DaemonType::UnixClient | DaemonType::UnixServer => windows::daemon().map(|_pid| ()),
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = params;
        Ok(())
    }
}

#[derive(PartialEq, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum Reason {
    Busy,   // server will enter this state
    Behind, // response to client, never enters state
    #[default]
    Other,
}

#[derive(PartialEq, Default, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Status {
    NotReady(Reason),
    #[default]
    Ready,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::NotReady(reason) => {
                write!(f, "NotReady(")?;
                match reason {
                    Reason::Busy => write!(f, "Sinking")?,
                    Reason::Behind => write!(f, "Behind")?,
                    Reason::Other => write!(f, "Other")?,
                }
                write!(f, ")")
            }
            Status::Ready => write!(f, "Ready"),
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub hostname: String,
    pub username: String,
    pub src_paths: Vec<PathBuf>,
    pub dest_path: PathBuf,
    pub date: String,
    /// Stable id for this sinkd client install (persisted locally).
    pub client_id: String,
    /// Generation the sender last reconciled with (`basis`); must match server head to push.
    pub basis_generation: u64,
    /// Server's current head generation (set on server-originated messages).
    pub head_generation: u64,
    /// When non-empty on a server message, the `client_id` that completed the last successful apply.
    pub last_writer_client_id: String,
    pub status: Status,
    pub rsync: Option<config::ResolvedRsyncConfig>,
}

#[allow(dead_code)]
impl Payload {
    pub fn new() -> Outcome<Payload> {
        Ok(Payload {
            hostname: config::get_hostname()?,
            username: config::get_username()?,
            src_paths: vec![],
            date: String::from("2022Jan4"),
            client_id: String::new(),
            basis_generation: 0,
            head_generation: 0,
            last_writer_client_id: String::new(),
            status: Status::Ready,
            dest_path: PathBuf::from("server"),
            rsync: None,
        })
    }

    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn from(
        hostname: String,
        username: String,
        src_paths: Vec<PathBuf>,
        dest_path: PathBuf,
        date: String,
        client_id: String,
        basis_generation: u64,
        head_generation: u64,
        last_writer_client_id: String,
        status: Status,
        rsync: Option<config::ResolvedRsyncConfig>,
    ) -> Payload {
        Payload {
            hostname,
            username,
            src_paths,
            dest_path,
            date,
            client_id,
            basis_generation,
            head_generation,
            last_writer_client_id,
            status,
            rsync,
        }
    }

    #[must_use]
    pub fn hostname<S: Into<String>>(mut self, hostname: S) -> Self {
        self.hostname = hostname.into();
        self
    }

    #[must_use]
    pub fn username<S: Into<String>>(mut self, username: S) -> Self {
        self.username = username.into();
        self
    }

    #[must_use]
    pub fn src_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.src_paths = paths;
        self
    }

    #[must_use]
    pub fn date<S: Into<String>>(mut self, date: S) -> Self {
        self.date = date.into();
        self
    }

    #[must_use]
    pub fn client_id<S: Into<String>>(mut self, client_id: S) -> Self {
        self.client_id = client_id.into();
        self
    }

    #[must_use]
    pub fn basis_generation(mut self, g: u64) -> Self {
        self.basis_generation = g;
        self
    }

    #[must_use]
    pub fn head_generation(mut self, g: u64) -> Self {
        self.head_generation = g;
        self
    }

    #[must_use]
    pub fn last_writer_client_id<S: Into<String>>(mut self, id: S) -> Self {
        self.last_writer_client_id = id.into();
        self
    }

    #[must_use]
    pub fn status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    #[must_use]
    pub fn dest_path<P: AsRef<Path>>(mut self, dest_path: P) -> Self {
        self.dest_path = PathBuf::from(dest_path.as_ref());
        self
    }

    #[must_use]
    pub fn rsync(mut self, rsync: config::ResolvedRsyncConfig) -> Self {
        self.rsync = Some(rsync);
        self
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "hostname: {}, username: {}, src_paths: [",
            self.hostname, self.username
        )?;
        for path in &self.src_paths {
            write!(f, "{}, ", path.display())?;
        }
        write!(
            f,
            "], dest_path: {}, date: {}, client_id: {}, basis: {}, head: {}, last_writer: {}, status: {}",
            self.dest_path.display(),
            self.date,
            self.client_id,
            self.basis_generation,
            self.head_generation,
            self.last_writer_client_id,
            self.status
        )
    }
}

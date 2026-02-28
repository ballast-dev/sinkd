use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[cfg(windows)]
use crate::parameters::DaemonType;
use crate::{config, outcome::Outcome, parameters::Parameters};

mod zenoh;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use zenoh::{Rx, TOPIC_CLIENTS, TOPIC_SERVER, ZenohClient, ZenohMessage};

pub fn terminal_topic() -> Outcome<String> {
    Ok(format!("sinkd/{}/terminate", config::get_hostname()?))
}

pub fn send_terminate_signal() -> Outcome<()> {
    let topic = terminal_topic()?;
    match ZenohClient::new(&[], &topic) {
        Ok((client, _rx)) => {
            let mut payload = Payload::new()?.status(Status::NotReady(Reason::Other));
            if let Err(e) = client.publish(&mut payload) {
                error!("failed to send terminate message: {e}");
            }
            client.disconnect();
        }
        Err(e) => {
            error!("failed to create Zenoh client for termination: {e}");
        }
    }
    Ok(())
}

pub fn connect_with_terminate_topic(
    subscriptions: &[&str],
    publish_topic: &str,
) -> Outcome<(ZenohClient, Rx, String)> {
    let terminal = terminal_topic()?;
    let mut all_subscriptions = subscriptions.to_vec();
    all_subscriptions.push(&terminal);

    let (client, rx) = ZenohClient::new(&all_subscriptions, publish_topic)
        .map_err(|e| format!("unable to create Zenoh client: {e}"))?;

    Ok((client, rx, terminal))
}

#[allow(unused_variables)]
pub fn daemon(func: fn(&Parameters) -> Outcome<()>, params: &Parameters) -> Outcome<()> {
    #[cfg(unix)]
    {
        unix::daemon(func, params)
    }
    #[cfg(windows)]
    {
        match params.daemon_type {
            DaemonType::WindowsClient => {
                windows::redirect_stdio_to_null()?;
                crate::client::init(params)
            }
            DaemonType::WindowsServer => {
                windows::redirect_stdio_to_null()?;
                crate::server::init(params)
            }
            // not daemonized yet
            _ => windows::daemon().map(|_pid| ()),
        }
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
    pub cycle: u32,
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
            cycle: 0,
            status: Status::Ready,
            dest_path: PathBuf::from("server"),
            rsync: None,
        })
    }

    pub fn from(
        hostname: String,
        username: String,
        src_paths: Vec<PathBuf>,
        dest_path: PathBuf,
        date: String,
        cycle: u32,
        status: Status,
        rsync: Option<config::ResolvedRsyncConfig>,
    ) -> Payload {
        Payload {
            hostname,
            username,
            src_paths,
            dest_path,
            date,
            cycle,
            status,
            rsync,
        }
    }

    pub fn hostname<S: Into<String>>(mut self, hostname: S) -> Self {
        self.hostname = hostname.into();
        self
    }

    pub fn username<S: Into<String>>(mut self, username: S) -> Self {
        self.username = username.into();
        self
    }

    pub fn src_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.src_paths = paths;
        self
    }

    pub fn date<S: Into<String>>(mut self, date: S) -> Self {
        self.date = date.into();
        self
    }

    pub fn cycle(mut self, cycle: u32) -> Self {
        self.cycle = cycle;
        self
    }

    pub fn status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    pub fn dest_path<P: AsRef<Path>>(mut self, dest_path: P) -> Self {
        self.dest_path = PathBuf::from(dest_path.as_ref());
        self
    }

    pub fn rsync(mut self, rsync: config::ResolvedRsyncConfig) -> Self {
        self.rsync = Some(rsync);
        self
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "hostname: {}, username: {}, src_paths: [", self.hostname, self.username)?;
        for path in &self.src_paths {
            write!(f, "{}, ", path.display())?;
        }
        write!(
            f,
            "], dest_path: {}, date: {}, cycle: {}, status: {}",
            self.dest_path.display(),
            self.date,
            self.cycle,
            self.status
        )
    }
}


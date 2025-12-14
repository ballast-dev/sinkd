use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[cfg(windows)]
use crate::parameters::DaemonType;
use crate::{config, outcome::Outcome, parameters::Parameters};

mod dds;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use dds::{DdsClient, Rx, TOPIC_CLIENTS, TOPIC_SERVER};

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
                write!(f, "NotReady(").unwrap();
                match reason {
                    Reason::Busy => write!(f, "Sinking").unwrap(),
                    Reason::Behind => write!(f, "Behind").unwrap(),
                    Reason::Other => write!(f, "Other").unwrap(),
                }
                write!(f, ")") // return result of write
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
    ) -> Payload {
        Payload {
            hostname,
            username,
            src_paths,
            dest_path,
            date,
            cycle,
            status,
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
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "hostname: {}, username: {}, src_paths: [",
            self.hostname, self.username,
        )
        .unwrap();
        for path in &self.src_paths {
            write!(f, "{}, ", path.display()).unwrap();
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


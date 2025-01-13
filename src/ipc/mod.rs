use paho_mqtt::{self as mqtt, MQTT_VERSION_3_1_1};
use serde::{Deserialize, Serialize};
use std::fs::File;

use std::{
    ffi::OsStr,
    fmt,
    path::{Path, PathBuf},
    time::Duration,
    process::{Command, Stdio},
};

use crate::{
    bad, config,
    outcome::Outcome,
    parameters::Parameters,
    shiplog, time,
};

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::daemon;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::daemon;


pub type Rx = mqtt::Receiver<Option<mqtt::Message>>;

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Reason {
    Busy,   // server will enter this state
    Behind, // response to client, never enters state
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
                };
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

    pub fn status(mut self, status: &Status) -> Self {
        self.status = *status;
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

pub fn encode(payload: &mut Payload) -> Result<Vec<u8>, mqtt::Error> {
    payload.date = time::stamp(Some("%Y%m%d"));
    match bincode::serialize(payload) {
        Err(e) => Err(mqtt::Error::GeneralString(format!(
            "FATAL, bincode::serialize >> {e}"
        ))),
        Ok(stream) => Ok(stream),
    }
}

pub fn decode(bytes: &[u8]) -> Result<Payload, mqtt::Error> {
    match bincode::deserialize(bytes) {
        Err(_) => Err(mqtt::Error::General("FATAL, bincode could not deserialize")),
        Ok(payload) => Ok(payload),
    }
}

pub struct MqttClient {
    pub client: mqtt::Client,
    publish_topic: String,
}

impl MqttClient {
    pub fn new(
        host: Option<&str>,
        subscriptions: &[&str],
        publish_topic: &str,
    ) -> Result<(Self, mqtt::Receiver<Option<mqtt::Message>>), mqtt::Error> {
        let opts = mqtt::CreateOptionsBuilder::new()
            .server_uri(resolve_host(host)?)
            .mqtt_version(MQTT_VERSION_3_1_1)
            .persistence(None)
            .finalize();
        let cli = mqtt::Client::new(opts)?;

        let rx = cli.start_consuming();

        let lwt = mqtt::MessageBuilder::new()
            .topic("sinkd/server")
            .payload("Sync consumer lost connection")
            .finalize();

        let conn_opts = mqtt::ConnectOptionsBuilder::new_v3()
            .keep_alive_interval(Duration::from_secs(20))
            .clean_session(true)
            .will_message(lwt)
            .finalize();

        let qos = vec![0; subscriptions.len()];

        debug!(
            "Connecting to MQTT broker at host: {}, subscriptions: [{}], publish_topic: {}",
            host.unwrap_or("unknown"),
            subscriptions.to_vec().join(", "),
            publish_topic
        );

        match cli.connect(conn_opts) {
            Ok(rsp) => {
                if let Some(con_rsp) = rsp.connect_response() {
                    debug!(
                        "Connected to: '{}' with MQTT version {}",
                        con_rsp.server_uri, con_rsp.mqtt_version
                    );
                    if con_rsp.session_present {
                        return Err(mqtt::Error::General("Client session already present on broker"));
                    }

                    debug!("Subscribing to topics: {:?} with QoS {:?}", subscriptions, qos);
                    cli.subscribe_many(subscriptions, &qos)
                        .map_err(|_| mqtt::Error::General("Failed to subscribe to topics"))?;

                    Ok((
                        MqttClient {
                            client: cli,
                            publish_topic: publish_topic.to_owned(),
                        },
                        rx,
                    ))
                } else {
                    Err(mqtt::Error::General("No connection response from broker"))
                }
            }
            Err(e) => Err(mqtt::Error::GeneralString(format!(
                "Could not connect to broker '{}': {:?}. Ensure the broker is running and reachable.",
                host.unwrap_or("unknown"),
                e
            ))),
        }
    }

    pub fn publish(&self, payload: &mut Payload) -> Outcome<()> {
        match self.client.publish(mqtt::Message::new(
            &self.publish_topic,
            encode(payload)?,
            mqtt::QOS_0,
        )) {
            Ok(()) => {
                info!("published payload: {}", payload);
                Ok(())
            }
            Err(e) => {
                error!("could not publish payload {}, {}", payload, e);
                bad!("could not publish payload {}, {}", payload, e)
            }
        }
    }

    pub fn disconnect(&self) {
        debug!("disconnecting from mqtt...");
        self.client.disconnect(None).expect("cannot disconnect?");
    }
}

fn resolve_host(host: Option<&str>) -> Result<String, mqtt::Error> {
    match host {
        Some(h) if h.starts_with('/') => Err(mqtt::Error::General(
            "Invalid hostname: it looks like a path. Did you mean 'localhost'?",
        )),
        Some(h) => {
            let fq_host = format!("tcp://{}:1883", h);
            debug!("Fully qualified host: {}", fq_host);
            Ok(fq_host)
        }
        None => Err(mqtt::Error::General("Host string is required but missing")),
    }
}

pub fn start_mosquitto() -> Outcome<()> {
    debug!(">> spawn mosquitto daemon");
    if let Err(spawn_error) = Command::new("mosquitto").arg("-d").spawn() {
        return bad!(format!(
            "Is mosquitto installed and in path? >> {}",
            spawn_error
        ));
    }
    Ok(())
}


pub fn rsync<P>(srcs: &Vec<P>, dest: &P)
where
    P: AsRef<OsStr> + AsRef<Path> + std::fmt::Debug,
{
    let mut cmd = Command::new("rsync");

    cmd.arg("-atR")
        .arg("--delete")
        .args(srcs)
        .arg(dest);

    match cmd.spawn() {
        Err(x) => error!("{:#?}", x),
        Ok(_) => debug!("\u{1f6b0} rsync {:#?} {:#?} \u{1f919}", srcs, dest),
    }
}

use std::{fmt, path::PathBuf, time};

use paho_mqtt as mqtt;
use serde::{Deserialize, Serialize};

use crate::outcome::Outcome;
use crate::utils;

pub type Rx = mqtt::Receiver<Option<mqtt::Message>>;

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Reason {
    Sinking,
    Behind,
    Other,
}

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Status {
    NotReady(Reason),
    Ready,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::NotReady(reason) => {
                write!(f, "NotReady(").unwrap();
                match reason {
                    Reason::Sinking => write!(f, "Sinking").unwrap(),
                    Reason::Behind => write!(f, "Behind").unwrap(),
                    Reason::Other => write!(f, "Other").unwrap(),
                };
                write!(f, ")") // return result of write
            }
            Status::Ready => write!(f, "Ready"),
        }
    }
}

/// Only time a Payload is sent is to say "new edits"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    pub hostname: String,
    pub username: String,
    pub src_paths: Vec<PathBuf>,
    pub dest_path: String,
    pub date: String,
    pub cycle: u32,
    pub status: Status,
}

impl Payload {
    pub fn new() -> Payload {
        Payload {
            hostname: utils::get_hostname(),
            username: utils::get_username(),
            src_paths: vec![],
            date: String::from("2022Jan4"),
            cycle: 0,
            status: Status::Ready,
            dest_path: String::from("server"),
        }
    }
    pub fn from(
        hostname: String,
        username: String,
        src_paths: Vec<PathBuf>,
        dest_path: String,
        date: String,
        cycle: u32,
        status: Status,
    ) -> Payload {
        Payload {
            hostname,
            username,
            dest_path,
            src_paths,
            date,
            cycle,
            status,
        }
    }
    pub fn hostname<'a>(mut self, hostname: &'a str) -> Self {
        self.hostname = hostname.to_string();
        self
    }
    pub fn username<'a>(mut self, username: &'a str) -> Self {
        self.username = username.to_string();
        self
    }
    pub fn paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.src_paths = paths; // transfer ownership
        self
    }
    pub fn date<'a>(mut self, date: &'a str) -> Self {
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
    pub fn dest<'a>(mut self, dest: &'a str) -> Self {
        self.dest_path = dest.into(); // ownership
        self
    }
    pub fn ready(self) -> Result<(), Reason> {
        match self.status {
            Status::Ready => Ok(()),
            Status::NotReady(reason) => Err(reason),
        }
    }
}

impl fmt::Display for Payload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "hostname: {}, username: {}, paths: [",
            self.hostname, self.username,
        )
        .unwrap();
        for path in &self.src_paths {
            write!(f, "{}, ", path.display()).unwrap();
        }
        write!(
            f,
            "], date: {}, cycle: {}, status: {}",
            self.date, self.cycle, self.status
        )
    }
}

/// Adds timestamp and serializes payload for transfer
pub fn encode(payload: &mut Payload) -> Result<Vec<u8>, mqtt::Error> {
    payload.date = utils::get_timestamp("%Y%m%d");
    match bincode::serialize(payload) {
        Err(e) => Err(mqtt::Error::GeneralString(format!(
            "FATAL, bincode::serialize >> {}",
            e
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

/// A wrapper for sinkd implementation
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
            .client_id(utils::get_hostname())
            .finalize();
        let cli = mqtt::Client::new(opts)?;

        // Initialize the consumer before connecting
        let rx = cli.start_consuming();

        // Define last will message
        let lwt = mqtt::MessageBuilder::new()
            .topic("sinkd/server")
            .payload("Sync consumer lost connection")
            .finalize();

        // Define the set of options for the connection
        let conn_opts = mqtt::ConnectOptionsBuilder::new_v3()
            .keep_alive_interval(time::Duration::from_secs(20))
            .clean_session(true)
            .will_message(lwt)
            .finalize();

        let qos = [1, 1];

        // Make the connection to the broker
        debug!(
            "Connecting to the MQTT broker host:{} subs:[{}], pub_topic:{}",
            host.unwrap_or("unknown"),
            subscriptions
                .iter()
                .map(|&element| element)
                .collect::<Vec<_>>()
                .join(" "),
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
                        Err(mqtt::Error::General(
                            "client session already present on broker",
                        ))
                    } else {
                        // Register subscriptions on the server
                        debug!("Subscribing to topics with requested QoS: {:?}...", qos);

                        cli.subscribe_many(subscriptions, &qos)
                            .and_then(|rsp| {
                                rsp.subscribe_many_response()
                                    .ok_or(mqtt::Error::General("Bad response"))
                            })
                            .map(|vqos| {
                                debug!("QoS granted: {:?}", vqos);
                            })?;

                        Ok((
                            MqttClient {
                                client: cli,
                                publish_topic: publish_topic.to_owned(),
                            },
                            rx,
                        ))
                    }
                } else {
                    Err(mqtt::Error::General("no connection response?"))
                }
            }
            Err(e) => Err(mqtt::Error::GeneralString(format!(
                "Could not connect to the broker '{}', is the mosquitto broker running? {:?}",
                host.unwrap_or("unknown"),
                e
            ))),
        }
    }

    pub fn publish(&self, payload: &mut Payload) -> Outcome<()> {
        match self.client.publish(mqtt::Message::new(
            &self.publish_topic,
            encode(payload)?,
            mqtt::QOS_0, // within local network, should be no lost packets
        )) {
            Ok(_) => {
                info!("published payload: {}", payload);
                Ok(())
            }
            Err(e) => {
                error!("could not publish payload {}", payload);
                bad!("could not publish payload {}", payload)
            }
        }
    }
}

fn resolve_host(host: Option<&str>) -> Result<String, mqtt::Error> {
    match host {
        Some("localhost") => Ok(String::from("tcp://localhost:1883")),
        Some(_str) => {
            if _str.starts_with('/') {
                Err(mqtt::Error::General(
                    "did you intend on localhost?, check '/etc/sinkd.conf'",
                ))
            } else {
                Ok(format!("tcp://{}:1883", _str))
            }
        }
        None => Err(mqtt::Error::General("Need host string")),
    }
}

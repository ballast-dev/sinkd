use std::{
    fmt::{self, write},
    time,
};

use bincode;
use paho_mqtt as mqtt;
use serde::{Deserialize, Serialize};

use crate::utils;

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
        match *self {
            Status::NotReady(reason) => {
                write!(f, "NotReady(");
                match reason {
                    Reason::Sinking => write!(f, "Sinking"),
                    Reason::Behind => write!(f, "Behind"),
                    Reason::Other => write!(f, "Other"),
                };
                write!(f, ")")
            }
            Status::Ready => write!(f, "Ready"),
        }
    }
}

/// Only time a Payload is sent is to say "new edits"
#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    pub hostname: String,
    pub username: String,
    pub paths: Vec<String>,
    pub date: String,
    pub cycle: u32,
    pub status: Status,
}

impl Payload {
    pub fn new() -> Payload {
        Payload {
            hostname: utils::get_hostname(),
            username: utils::get_username(),
            paths: vec![],
            date: String::from("2022Jan4"),
            cycle: 0,
            status: Status::Ready,
        }
    }
    pub fn from(
        hostname: String,
        username: String,
        paths: Vec<String>,
        date: String,
        cycle: u32,
        status: Status,
    ) -> Payload {
        Payload {
            hostname,
            username,
            paths,
            date,
            cycle,
            status,
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
        for path in &self.paths {
            write!(f, "{}, ", path).unwrap();
        }
        write!(
            f,
            "], date: {}, cycle: {}, status: {}",
            self.date, self.cycle, self.status
        )
    }
}

pub fn encode(payload: &Payload) -> Result<Vec<u8>, mqtt::Error> {
    match bincode::serialize(payload) {
        Err(e) => Err(mqtt::Error::GeneralString(format!(
            "FATAL, bincode::serialize >> {}",
            e
        ))),
        Ok(stream) => return Ok(stream),
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
        let conn_opts = mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(time::Duration::from_secs(20))
            .mqtt_version(mqtt::MQTT_VERSION_3_1_1)
            .clean_session(true)
            .will_message(lwt)
            .finalize();

        let qos = [1, 1];

        // Make the connection to the broker
        debug!("Connecting to the MQTT broker...");
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

                        cli.subscribe_many(&subscriptions, &qos)
                            .and_then(|rsp| {
                                rsp.subscribe_many_response()
                                    .ok_or(mqtt::Error::General("Bad response"))
                            })
                            .and_then(|vqos| {
                                debug!("QoS granted: {:?}", vqos);
                                Ok(())
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
                "Error connecting to the broker: {:?}",
                e
            ))),
        }
    }

    pub fn publish(&self, payload: &mut Payload) -> Result<(), mqtt::Error> {
        payload.date = utils::get_timestamp("%Y%m%d").to_owned();
        self.client.publish(mqtt::Message::new(
            &self.publish_topic,
            encode(payload)?,
            mqtt::QOS_0, // within local network, should be no lost packets
        ))?;
        Ok(())
    }
}

fn resolve_host(host: Option<&str>) -> Result<String, mqtt::Error> {
    match host {
        Some("localhost") => Ok(String::from("tcp://localhost:1883")),
        Some(_str) => {
            if _str.starts_with("/") {
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

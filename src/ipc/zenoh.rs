use std::sync::mpsc;
use serde::{Deserialize, Serialize};
use zenoh::{key_expr::KeyExpr, sample::SampleKind, Wait};

use crate::{bad, outcome::Outcome};

use super::Payload;

/// Topic names - same as the MQTT topics for consistency
pub const TOPIC_CLIENTS: &str = "sinkd/clients";
pub const TOPIC_SERVER: &str = "sinkd/server";

/// Zenoh-compatible message type
/// Uses only primitive types to keep the payload portable
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZenohPayload {
    pub hostname: String,
    pub username: String,
    /// Paths serialized as newline-separated strings
    pub src_paths: String,
    pub dest_path: String,
    pub date: String,
    pub cycle: u32,
    /// Status encoded as: 0=Ready, 1=Busy, 2=Behind, 3=Other
    pub status_code: u8,
}

impl ZenohPayload {
    /// Convert from internal Payload to Zenoh-compatible payload
    pub fn from_payload(p: &Payload) -> Self {
        let src_paths = p
            .src_paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("\n");

        let status_code = match p.status {
            super::Status::Ready => 0,
            super::Status::NotReady(super::Reason::Busy) => 1,
            super::Status::NotReady(super::Reason::Behind) => 2,
            super::Status::NotReady(super::Reason::Other) => 3,
        };

        ZenohPayload {
            hostname: p.hostname.clone(),
            username: p.username.clone(),
            src_paths,
            dest_path: p.dest_path.to_string_lossy().to_string(),
            date: p.date.clone(),
            cycle: p.cycle,
            status_code,
        }
    }

    /// Convert from Zenoh payload back to internal Payload
    pub fn to_payload(&self) -> Payload {
        use std::path::PathBuf;

        let src_paths: Vec<PathBuf> = self
            .src_paths
            .split('\n')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect();

        let status = match self.status_code {
            0 => super::Status::Ready,
            1 => super::Status::NotReady(super::Reason::Busy),
            2 => super::Status::NotReady(super::Reason::Behind),
            _ => super::Status::NotReady(super::Reason::Other),
        };

        Payload {
            hostname: self.hostname.clone(),
            username: self.username.clone(),
            src_paths,
            dest_path: PathBuf::from(&self.dest_path),
            date: self.date.clone(),
            cycle: self.cycle,
            status,
        }
    }
}

/// A message received from Zenoh with topic information
#[derive(Debug, Clone)]
pub struct ZenohMessage {
    pub topic: String,
    pub payload: Payload,
}

/// Zenoh Client that replaces the Dust DDS client
pub struct ZenohClient {
    publish_topic: String,
    tx: mpsc::Sender<ZenohPayload>,
    session: zenoh::Session,
}

impl ZenohClient {
    /// Create a new Zenoh client
    ///
    /// # Arguments
    /// * `subscriptions` - Topics to subscribe to
    /// * `publish_topic` - Topic to publish to
    ///
    /// # Returns
    /// A tuple of (`ZenohClient`, Receiver) where Receiver receives `ZenohMessage`
    pub fn new(
        subscriptions: &[&str],
        publish_topic: &str,
    ) -> Result<(Self, mpsc::Receiver<Option<ZenohMessage>>), String> {
        let session = zenoh::open(zenoh::Config::default())
            .wait()
            .map_err(|e| format!("Failed to open Zenoh session: {e:?}"))?;

        let publish_key_expr = KeyExpr::autocanonize(publish_topic.to_string())
            .map_err(|e| format!("Invalid publish topic '{publish_topic}': {e:?}"))?
            .into_owned();

        // Create publisher
        let publisher = session
            .declare_publisher(publish_key_expr)
            .wait()
            .map_err(|e| format!("Failed to declare publisher: {e:?}"))?;

        debug!(
            "Zenoh client created - subscriptions: [{}], publish_topic: {}",
            subscriptions.join(", "),
            publish_topic
        );

        // Create channels for communication
        let (msg_tx, msg_rx) = mpsc::channel();
        let (pub_tx, pub_rx) = mpsc::channel::<ZenohPayload>();

        // Spawn a thread to handle publishing
        std::thread::spawn(move || {
            while let Ok(payload) = pub_rx.recv() {
                match bincode::serialize(&payload) {
                    Ok(bytes) => {
                        if let Err(e) = publisher.put(bytes).wait() {
                            error!("Zenoh put error: {e:?}");
                        }
                    }
                    Err(e) => error!("Zenoh serialize error: {e:?}"),
                }
            }
        });

        // Create subscribers for each subscription
        for sub_topic_name in subscriptions {
            let topic_name = (*sub_topic_name).to_string();
            let msg_tx = msg_tx.clone();

            session
                .declare_subscriber(*sub_topic_name)
                .callback(move |sample| {
                    if sample.kind() == SampleKind::Delete {
                        return;
                    }

                    let bytes = sample.payload().to_bytes();
                    match bincode::deserialize::<ZenohPayload>(bytes.as_ref()) {
                        Ok(payload) => {
                            let msg = ZenohMessage {
                                topic: topic_name.clone(),
                                payload: payload.to_payload(),
                            };
                            let _ = msg_tx.send(Some(msg));
                        }
                        Err(e) => error!("Zenoh deserialize error: {e:?}"),
                    }
                })
                .background()
                .wait()
                .map_err(|e| format!("Failed to declare subscriber {sub_topic_name}: {e:?}"))?;
        }

        let client = ZenohClient {
            publish_topic: publish_topic.to_string(),
            tx: pub_tx,
            session,
        };

        Ok((client, msg_rx))
    }

    /// Publish a payload to the configured topic
    pub fn publish(&self, payload: &mut Payload) -> Outcome<()> {
        // Update the date before publishing
        payload.date = crate::time::stamp(Some("%Y%m%d"));

        let outgoing = ZenohPayload::from_payload(payload);
        match self.tx.send(outgoing) {
            Ok(()) => {
                info!("published payload to {}: {payload}", self.publish_topic);
                Ok(())
            }
            Err(e) => {
                error!("could not publish payload {payload}, {e}");
                bad!("could not publish payload {}, {}", payload, e)
            }
        }
    }

    /// Disconnect from Zenoh
    pub fn disconnect(&self) {
        debug!("disconnecting from Zenoh...");
        if let Err(e) = self.session.close().wait() {
            error!("Zenoh close error: {e:?}");
        }
    }
}

/// Receiver type alias for Zenoh messages
pub type Rx = mpsc::Receiver<Option<ZenohMessage>>;

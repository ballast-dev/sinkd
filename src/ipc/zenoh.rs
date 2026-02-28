use std::sync::mpsc;
use std::time::Duration;
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
    pub rsync: Option<crate::config::ResolvedRsyncConfig>,
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
            rsync: p.rsync.clone(),
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
            rsync: self.rsync.clone(),
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
            let delay_ms = test_publish_delay_ms();
            let drop_every_n = test_drop_every_n();
            let reorder_pairs = test_reorder_pairs();
            let mut send_count: u64 = 0;
            let mut pending_for_reorder: Option<ZenohPayload> = None;

            while let Ok(payload) = pub_rx.recv() {
                if reorder_pairs {
                    if let Some(previous) = pending_for_reorder.take() {
                        if let Err(e) =
                            send_payload(&publisher, payload, delay_ms, drop_every_n, &mut send_count)
                        {
                            error!("{e}");
                        }
                        if let Err(e) =
                            send_payload(&publisher, previous, delay_ms, drop_every_n, &mut send_count)
                        {
                            error!("{e}");
                        }
                    } else {
                        pending_for_reorder = Some(payload);
                    }
                    continue;
                }

                if let Err(e) =
                    send_payload(&publisher, payload, delay_ms, drop_every_n, &mut send_count)
                {
                    error!("{e}");
                }
            }

            if let Some(payload) = pending_for_reorder {
                if let Err(e) =
                    send_payload(&publisher, payload, delay_ms, drop_every_n, &mut send_count)
                {
                    error!("{e}");
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

fn send_payload(
    publisher: &zenoh::pubsub::Publisher<'_>,
    payload: ZenohPayload,
    delay_ms: u64,
    drop_every_n: Option<u64>,
    send_count: &mut u64,
) -> Result<(), String> {
    if delay_ms > 0 {
        std::thread::sleep(Duration::from_millis(delay_ms));
    }

    *send_count += 1;
    if let Some(n) = drop_every_n {
        if n > 0 && *send_count % n == 0 {
            warn!("Zenoh test hook dropped outbound payload #{}", send_count);
            return Ok(());
        }
    }

    let bytes = bincode::serialize(&payload).map_err(|e| format!("Zenoh serialize error: {e:?}"))?;
    publisher
        .put(bytes)
        .wait()
        .map_err(|e| format!("Zenoh put error: {e:?}"))
}

fn test_publish_delay_ms() -> u64 {
    std::env::var("SINKD_TEST_PUBLISH_DELAY_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0)
}

fn test_drop_every_n() -> Option<u64> {
    std::env::var("SINKD_TEST_DROP_EVERY_N")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
}

fn test_reorder_pairs() -> bool {
    std::env::var("SINKD_TEST_REORDER_PAIRS")
        .ok()
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::{Duration, SystemTime, UNIX_EPOCH}};

    use super::{ZenohClient, ZenohPayload};
    use crate::ipc::{Payload, Reason, Status};

    #[test]
    fn payload_roundtrip_preserves_fields() {
        let payload = Payload::from(
            "host-a".to_string(),
            "alice".to_string(),
            vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")],
            PathBuf::from("/srv/sinkd"),
            "20260203".to_string(),
            42,
            Status::NotReady(Reason::Behind),
            None,
        );

        let wire = ZenohPayload::from_payload(&payload);
        let decoded = wire.to_payload();

        assert_eq!(decoded.hostname, payload.hostname);
        assert_eq!(decoded.username, payload.username);
        assert_eq!(decoded.src_paths, payload.src_paths);
        assert_eq!(decoded.dest_path, payload.dest_path);
        assert_eq!(decoded.date, payload.date);
        assert_eq!(decoded.cycle, payload.cycle);
        assert_eq!(decoded.status, payload.status);
    }

    #[test]
    fn status_code_mapping_is_stable() {
        let ready = Payload::from(
            "h".to_string(),
            "u".to_string(),
            vec![],
            PathBuf::from("x"),
            "d".to_string(),
            0,
            Status::Ready,
            None,
        );
        let busy = Payload::from(
            "h".to_string(),
            "u".to_string(),
            vec![],
            PathBuf::from("x"),
            "d".to_string(),
            0,
            Status::NotReady(Reason::Busy),
            None,
        );
        let behind = Payload::from(
            "h".to_string(),
            "u".to_string(),
            vec![],
            PathBuf::from("x"),
            "d".to_string(),
            0,
            Status::NotReady(Reason::Behind),
            None,
        );
        let other = Payload::from(
            "h".to_string(),
            "u".to_string(),
            vec![],
            PathBuf::from("x"),
            "d".to_string(),
            0,
            Status::NotReady(Reason::Other),
            None,
        );

        assert_eq!(ZenohPayload::from_payload(&ready).status_code, 0);
        assert_eq!(ZenohPayload::from_payload(&busy).status_code, 1);
        assert_eq!(ZenohPayload::from_payload(&behind).status_code, 2);
        assert_eq!(ZenohPayload::from_payload(&other).status_code, 3);
    }

    #[test]
    fn zenoh_sync_pub_sub_smoke() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        let topic = format!("sinkd/tests/smoke/{unique}");
        let topic_ref = topic.as_str();

        let (subscriber_client, subscriber_rx) =
            ZenohClient::new(&[topic_ref], topic_ref).expect("subscriber client should start");
        let (publisher_client, _publisher_rx) =
            ZenohClient::new(&[], topic_ref).expect("publisher client should start");

        std::thread::sleep(Duration::from_millis(200));

        let mut payload = Payload::from(
            "smoke-host".to_string(),
            "smoke-user".to_string(),
            vec![PathBuf::from("/tmp/smoke-file")],
            PathBuf::from("/srv/smoke"),
            "20260101".to_string(),
            1,
            Status::Ready,
            None,
        );
        publisher_client
            .publish(&mut payload)
            .expect("publish should succeed");

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut saw_message = false;
        while std::time::Instant::now() < deadline {
            match subscriber_rx.try_recv() {
                Ok(Some(msg)) if msg.topic == topic_ref => {
                    saw_message = true;
                    assert_eq!(msg.payload.hostname, "smoke-host");
                    break;
                }
                Ok(_) => {}
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }

        publisher_client.disconnect();
        subscriber_client.disconnect();
        assert!(saw_message, "did not receive smoke message on {}", topic_ref);
    }
}

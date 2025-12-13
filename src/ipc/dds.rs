use std::sync::mpsc;
use std::time::Duration;

use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        qos::QosKind,
        status::NO_STATUS,
        type_support::DdsType,
    },
    listener::NO_LISTENER,
    subscription::data_reader::DataReader,
    std_runtime::StdRuntime,
};

use crate::{bad, outcome::Outcome};

use super::Payload;

/// Topic names - same as the MQTT topics for consistency
pub const TOPIC_CLIENTS: &str = "sinkd/clients";
pub const TOPIC_SERVER: &str = "sinkd/server";

/// DDS-compatible message type
/// Uses only primitive types that dust-dds can serialize
#[derive(Debug, Clone, Default, DdsType)]
pub struct DdsPayload {
    #[dust_dds(key)]
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

impl DdsPayload {
    /// Convert from internal Payload to DDS-compatible payload
    pub fn from_payload(p: &Payload) -> Self {
        let src_paths = p.src_paths
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

        DdsPayload {
            hostname: p.hostname.clone(),
            username: p.username.clone(),
            src_paths,
            dest_path: p.dest_path.to_string_lossy().to_string(),
            date: p.date.clone(),
            cycle: p.cycle,
            status_code,
        }
    }

    /// Convert from DDS payload back to internal Payload
    pub fn to_payload(&self) -> Payload {
        use std::path::PathBuf;

        let src_paths: Vec<PathBuf> = self.src_paths
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

/// A message received from DDS with topic information
#[derive(Debug, Clone)]
pub struct DdsMessage {
    pub topic: String,
    pub payload: Payload,
}

/// DDS Client that replaces the MQTT client
/// Uses dust-dds for peer-to-peer communication
pub struct DdsClient {
    publish_topic: String,
    tx: mpsc::Sender<DdsPayload>,
}

impl DdsClient {
    /// Create a new DDS client
    ///
    /// # Arguments
    /// * `subscriptions` - Topics to subscribe to
    /// * `publish_topic` - Topic to publish to
    ///
    /// # Returns
    /// A tuple of (`DdsClient`, Receiver) where Receiver receives `DdsMessage`
    pub fn new(
        subscriptions: &[&str],
        publish_topic: &str,
    ) -> Result<(Self, mpsc::Receiver<Option<DdsMessage>>), String> {
        let domain_id = 0;
        let participant_factory = DomainParticipantFactory::get_instance();

        let participant = participant_factory
            .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
            .map_err(|e| format!("Failed to create participant: {e:?}"))?;

        // Create publisher
        let publisher = participant
            .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
            .map_err(|e| format!("Failed to create publisher: {e:?}"))?;

        // Create topic for publishing
        let pub_topic = participant
            .create_topic::<DdsPayload>(publish_topic, "DdsPayload", QosKind::Default, NO_LISTENER, NO_STATUS)
            .map_err(|e| format!("Failed to create publish topic: {e:?}"))?;

        // Create data writer
        let writer = publisher
            .create_datawriter(&pub_topic, QosKind::Default, NO_LISTENER, NO_STATUS)
            .map_err(|e| format!("Failed to create data writer: {e:?}"))?;

        // Create subscriber
        let subscriber = participant
            .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
            .map_err(|e| format!("Failed to create subscriber: {e:?}"))?;

        // Create readers for each subscription
        let mut readers: Vec<(String, DataReader<StdRuntime, DdsPayload>)> = Vec::new();
        for sub_topic_name in subscriptions {
            let sub_topic = participant
                .create_topic::<DdsPayload>(sub_topic_name, "DdsPayload", QosKind::Default, NO_LISTENER, NO_STATUS)
                .map_err(|e| format!("Failed to create subscription topic {sub_topic_name}: {e:?}"))?;

            let reader: DataReader<StdRuntime, DdsPayload> = subscriber
                .create_datareader(&sub_topic, QosKind::Default, NO_LISTENER, NO_STATUS)
                .map_err(|e| format!("Failed to create data reader for {sub_topic_name}: {e:?}"))?;

            readers.push(((*sub_topic_name).to_string(), reader));
        }

        debug!(
            "DDS client created - subscriptions: [{}], publish_topic: {}",
            subscriptions.join(", "),
            publish_topic
        );

        // Create channels for communication
        let (msg_tx, msg_rx) = mpsc::channel();
        let (pub_tx, pub_rx) = mpsc::channel::<DdsPayload>();

        // Spawn a thread to poll for messages from readers
        std::thread::spawn(move || {
            loop {
                for (topic_name, reader) in &readers {
                    // Use take() to consume samples without blocking
                    if let Ok(samples) = reader.take(10, &[], &[], &[]) {
                        for sample in samples {
                            if let Some(dds_payload) = sample.data {
                                let msg = DdsMessage {
                                    topic: topic_name.clone(),
                                    payload: dds_payload.to_payload(),
                                };
                                if msg_tx.send(Some(msg)).is_err() {
                                    return; // Channel closed, exit thread
                                }
                            }
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });

        // Spawn a thread to handle publishing
        std::thread::spawn(move || {
            while let Ok(dds_payload) = pub_rx.recv() {
                if let Err(e) = writer.write(dds_payload, None) {
                    error!("DDS write error: {e:?}");
                }
            }
        });

        let client = DdsClient {
            publish_topic: publish_topic.to_string(),
            tx: pub_tx,
        };

        Ok((client, msg_rx))
    }

    /// Publish a payload to the configured topic
    pub fn publish(&self, payload: &mut Payload) -> Outcome<()> {
        // Update the date before publishing
        payload.date = crate::time::stamp(Some("%Y%m%d"));

        // Convert to DDS-compatible payload
        let dds_payload = DdsPayload::from_payload(payload);

        match self.tx.send(dds_payload) {
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

    /// Disconnect from DDS
    #[allow(clippy::unused_self)]
    pub fn disconnect(&self) {
        debug!("disconnecting from DDS...");
        // Channels will be closed when the client is dropped
        // The participant cleanup happens when the threads complete
    }
}

/// Receiver type alias for DDS messages
pub type Rx = mpsc::Receiver<Option<DdsMessage>>;

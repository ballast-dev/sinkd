use std::time::Duration;
use paho_mqtt;

use crate::{ bad, outcome::Outcome };

pub struct MqttClient {
    pub client: paho_mqtt::Client,
    publish_topic: String,
}

impl MqttClient {
    pub fn new(
        host: Option<&str>,
        subscriptions: &[&str],
        publish_topic: &str,
    ) -> Result<(Self, paho_mqtt::Receiver<Option<paho_mqtt::Message>>), paho_mqtt::Error> {
        let opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(super::resolve_host(host)?)
            .mqtt_version(paho_mqtt::MQTT_VERSION_3_1_1)
            .persistence(None)
            .finalize();
        let cli = paho_mqtt::Client::new(opts)?;

        let rx = cli.start_consuming();

        let lwt = paho_mqtt::MessageBuilder::new()
            .topic("sinkd/server")
            .payload("Sync consumer lost connection")
            .finalize();

        let conn_opts = paho_mqtt::ConnectOptionsBuilder::new_v3()
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
                        return Err(paho_mqtt::Error::General("Client session already present on broker"));
                    }

                    debug!("Subscribing to topics: {:?} with QoS {:?}", subscriptions, qos);
                    cli.subscribe_many(subscriptions, &qos)
                        .map_err(|_| paho_mqtt::Error::General("Failed to subscribe to topics"))?;

                    Ok((
                        MqttClient {
                            client: cli,
                            publish_topic: publish_topic.to_owned(),
                        },
                        rx,
                    ))
                } else {
                    Err(paho_mqtt::Error::General("No connection response from broker"))
                }
            }
            Err(e) => Err(paho_mqtt::Error::GeneralString(format!(
                "Could not connect to broker '{}': {:?}. Ensure the broker is running and reachable.",
                host.unwrap_or("unknown"),
                e
            ))),
        }
    }

    pub fn publish(&self, payload: &mut super::Payload) -> Outcome<()> {
        match self.client.publish(paho_mqtt::Message::new(
            &self.publish_topic,
            super::encode(payload)?,
            paho_mqtt::QOS_0,
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
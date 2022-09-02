use bincode;
use paho_mqtt as mqtt;
use serde::{Deserialize, Serialize};
use crate::{ipc};
use std::sync::Mutex;

#[derive(PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Status {
    Edits, // Needed to show new edits
    Sinkd,
    Cache, // to move files off to .sinkd_cache/ folders
    Updating,
    Behind,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload<'a> {
    pub hostname: &'a str,
    pub user: &'a str,
    pub path: &'a str, // one path per packet
    pub date: &'a str,
    pub cycle: u32,
    pub status: Status,
}

pub fn packed<'a>(
    hostname: &'a str,
    user: &'a str,
    path: &'a str,
    date: &'a str,
    cycle: u32,
    status: Status,
) -> Result<Vec<u8>, ()> {
    let payload = Payload {
        hostname,
        user,
        path,
        date,
        cycle,
        status,
    };
    match bincode::serialize(&payload) {
        Err(e) => {
            error!("FATAL, bincode::serialize >> {}", e);
            Err(())
        }
        Ok(stream) => return Ok(stream),
    }
}

pub struct MqttClient {
    client: mqtt::AsyncClient,
}

impl MqttClient {
    pub fn new<C>(host: Option<&str>, mut callback: C) -> Result<MqttClient, String>
    where
        C: FnMut(&Option<mqtt::Message>) + 'static,
    {
        let fq_host: String;
        match host {
            Some("localhost") => {
                fq_host = String::from("tcp://localhost:1883");
            }
            Some(_str) => {
                if _str.starts_with("/") {
                    return Err(String::from(
                        "did you intend on localhost?, check '/etc/sinkd.conf'",
                    ));
                }
                fq_host = format!("tcp://{}:1883", _str);
            }
            None => {
                error!("Need host string");
                std::process::exit(-1);
            }
        }

        match mqtt::AsyncClient::new(fq_host) {
            Err(e) => Err(format!("Error creating the client: {:?}", e)),
            Ok(async_client) => {
                // TODO: replumb for cleaner abstraction
                async_client.set_message_callback(move |_cli, msg| callback(&msg));
                let lwt =
                    mqtt::Message::new("sinkd/lost_conn", "Async subscriber lost connection", 1);
                let conn_opts = mqtt::ConnectOptionsBuilder::new()
                    .keep_alive_interval(std::time::Duration::from_secs(20))
                    .mqtt_version(mqtt::MQTT_VERSION_3_1_1)
                    .clean_session(true)
                    .will_message(lwt)
                    .finalize();
                async_client.connect(conn_opts.clone());
                async_client.connect_with_callbacks(
                    conn_opts,
                    MqttClient::on_connect_success,
                    MqttClient::on_connect_failure,
                );

                Ok(MqttClient {
                    client: async_client,
                })
            }
        }
    }

    pub fn publish(&mut self, msg: mqtt::Message) {
        if let Err(e) = self.client.try_publish(msg) {
            error!("Unable to publish: {}", e);
        }
    }

    pub fn subscribe<'a>(&self, topic: &'a str) {
        self.client.subscribe(topic, mqtt::QOS_0);
    }

    // Callback for a successful connection to the broker.
    // We subscribe to the topic(s) we want here.
    fn on_connect_success(cli: &mqtt::AsyncClient, _msgid: u16) {
        debug!("connected to mqtt broker");
        // cli.subscribe("sinkd/#", mqtt::QOS_0);
    }

    // Callback for a failed attempt to connect to the server.
    // We simply sleep and then try again.
    //
    // Note that normally we don't want to do a blocking operation or sleep
    // from  within a callback. But in this case, we know that the client is
    // *not* conected, and thus not doing anything important. So we don't worry
    // too much about stopping its callback thread.
    fn on_connect_failure(cli: &mqtt::AsyncClient, _msgid: u16, rc: i32) {
        println!("Connection attempt failed with error code {}.\n", rc);
        std::thread::sleep(std::time::Duration::from_millis(2500));
        cli.reconnect_with_callbacks(
            MqttClient::on_connect_success,
            MqttClient::on_connect_failure,
        );
    }

    pub fn disconnect(&mut self) {
        self.client.disconnect(
            mqtt::DisconnectOptionsBuilder::new()
                .reason_code(mqtt::ReasonCode::default())
                .finalize(),
        );
    }
}

use paho_mqtt as mqtt;

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgUpdate {
    user: String,
    path: String,
    date: String,
    cycle: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MsgStatus {
    date: String,
    cycle: u16,
}

pub struct MqttClient {
    client: mqtt::AsyncClient
}


impl MqttClient {
    //? Pass None to this constructor for localhost
    pub fn new(host: Option<&str>) -> Result<MqttClient, String> {
        match mqtt::AsyncClient::new(host.unwrap_or("tcp://localhost:1883")) {
            Err(e) => Err(format!("Error creating the client: {:?}", e)),
            Ok(_c) => {
                let mut mqtt_client = MqttClient { client: _c};
                mqtt_client.connect();
                Ok(mqtt_client)
            }
        }
    }

    pub fn callback<F>(&mut self, cb: F) 
        where F: FnMut(&mqtt::AsyncClient, Option<mqtt::Message>) + 'static 
    {
        self.client.set_message_callback(cb);
    }

    pub fn publish(&mut self, arg: &str) {
        if let Err(e) = self.client.try_publish(mqtt::Message::new("sinkd/", arg, 0)) {
            error!("{}", e)
        }
    }

    // Callback for a successful connection to the broker.
    // We subscribe to the topic(s) we want here.
    fn on_connect_success(cli: &mqtt::AsyncClient, _msgid: u16) {
        println!("Connection succeeded");
        cli.subscribe("sinkd/#", mqtt::QOS_0);
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
        cli.reconnect_with_callbacks(MqttClient::on_connect_success, MqttClient::on_connect_failure);
    }

    fn connect(&mut self) {
        let lwt = mqtt::Message::new("sinkd/lost_conn", "Async subscriber lost connection", 1);
        let conn_opts = mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(std::time::Duration::from_secs(20))
            .mqtt_version(mqtt::MQTT_VERSION_3_1_1)
            .clean_session(true)
            .will_message(lwt)
            .finalize();
        self.client.connect_with_callbacks(conn_opts, 
                                           MqttClient::on_connect_success, 
                                           MqttClient::on_connect_failure);
    }

    pub fn disconnect(&mut self) {
        self.client.disconnect(
            mqtt::DisconnectOptionsBuilder::new()
            .reason_code(mqtt::ReasonCode::default())
            .finalize()
        );
    }
}
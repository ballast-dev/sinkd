use std::process;
use paho_mqtt as mqtt;


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
    cli.reconnect_with_callbacks(on_connect_success, on_connect_failure);
}

fn callme() {
    println!("callback invoked!");
}

pub fn listen() {
    let (tx, rx) = std::sync::mpsc::channel();

    // Create a client & define connect options
    let mut client = mqtt::AsyncClient::new("tcp://localhost:1883").unwrap_or_else(|err| {
        println!("Error creating the client: {:?}", err);
        process::exit(1);
    });

    // Create a message and publish it
    let msg = mqtt::Message::new("sinkd/", "sinkd is gonna kick ass", 0);

    client.publish(msg);

    let disconn_opts = mqtt::DisconnectOptionsBuilder::new()
                    .reason_code(mqtt::ReasonCode::default())
                    .finalize();

    // Attach a closure to the client to receive callback
    // on incoming messages.
    client.set_message_callback(move |_cli,msg| {
        if let Some(msg) = msg {
            let topic = msg.topic();
            let payload_str = msg.payload_str();

            if topic == "sinkd/general" {
                println!("{} => {}", topic, payload_str);
            } else if topic == "sinkd/offtopic" {
                callme();
            } else if topic == "sinkd/discon" {
                // Disconnect from the broker
                println!("setting running to false");
                if let Err(e) = tx.send(false) {
                    println!("couldn't send! {:?}", e);
                }
            } else {
                println!("{} - {}", topic, payload_str);
            }
        }
    });
    
    let lwt = mqtt::Message::new("sinkd/lost_conn", "Async subscriber lost connection", 1);
    
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
    .keep_alive_interval(std::time::Duration::from_secs(20))
    .mqtt_version(mqtt::MQTT_VERSION_3_1_1)
    .clean_session(true)
    .will_message(lwt)
    .finalize();
    
    // Make the connection to the broker
    println!("Connecting to the MQTT server...");
    client.connect_with_callbacks(conn_opts, on_connect_success, on_connect_failure);
    
    println!("Waiting for messages...");
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Ok(val) = rx.try_recv() {
            println!("got it! {:?}", val);
            client.disconnect(disconn_opts.clone());
            std::process::exit(0);
        }
    }
    
}

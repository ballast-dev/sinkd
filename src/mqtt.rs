use std::process;
use paho_mqtt as mqtt;

pub fn listen() {
    // Create a client & define connect options
    let cli = mqtt::Client::new("tcp://localhost:1883").unwrap_or_else(|err| {
        println!("Error creating the client: {:?}", err);
        process::exit(1);
    });

    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(std::time::Duration::from_secs(20))
        .clean_session(true)
        .finalize();

    // Connect and wait for it to complete or fail
    if let Err(e) = cli.connect(conn_opts) {
        println!("Unable to connect:\n\t{:?}", e);
        process::exit(1);
    }

    // Create a message and publish it
    let msg = mqtt::Message::new("sinkd/", "sinkd is gonna kick ass", 0);

    if let Err(e) = cli.publish(msg) {
        println!("Error sending message: {:?}", e);
    }

    let disconn_opts = mqtt::DisconnectOptionsBuilder::new()
                    .reason_code(mqtt::ReasonCode::default())
                    .finalize();

    // // Attach a closure to the client to receive callback
    // // on incoming messages.
    // cli.set_message_callback(|cli,msg| {
    //     if let Some(msg) = msg {
    //         let topic = msg.topic();
    //         let payload_str = msg.payload_str();

    //         if topic == "requests/subscription/add" {
    //             let data = cli.user_data().unwrap();
    //             if let Some(lock) = data.downcast_ref::<UserTopics>() {
    //                 let mut topics = lock.write().unwrap();
    //                 let new_topic = payload_str.to_owned().to_string();
    //                 println!("Adding topic: {}", new_topic);
    //                 cli.subscribe(&new_topic, QOS);
    //                 topics.push(new_topic);
    //             }
    //             else {
    //                 println!("Failed to add topic: {}", payload_str);
    //             }
    //         }
    //         else {
    //             println!("{} - {}", topic, payload_str);
    //         }
    //     }
    // });

    // Disconnect from the broker
    if let Err(e) = cli.disconnect(disconn_opts) {
        println!("Error disconnecting, {:?}", e);
        process::exit(1);
    }
    // tok.wait().unwrap();
}

use leyline::{Client, Message};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Point-to-Point Communication Example\n");

    // Create Client A listener (receives messages on port 8080)
    let callback_a = Box::new(|msg: &Message| {
        println!("📨 Client A received: [{}] {}", 
            msg.topic, 
            String::from_utf8_lossy(&msg.payload)
        );
    });
    
    let client_a_listener = Client::new(callback_a);
    
    // Start Client A listener in a thread
    thread::spawn(move || {
        println!("🎧 Client A listening on port 8080...");
        if let Err(e) = client_a_listener.listen(8080, "chat") {
            eprintln!("❌ Client A listener error: {}", e);
        }
    });

    // Give Client A time to start listening
    thread::sleep(Duration::from_millis(100));

    // Create Client B listener (receives messages on port 8081)
    let callback_b = Box::new(|msg: &Message| {
        println!("📨 Client B received: [{}] {}", 
            msg.topic, 
            String::from_utf8_lossy(&msg.payload)
        );
    });
    
    let client_b_listener = Client::new(callback_b);
    
    // Start Client B listener in a thread
    thread::spawn(move || {
        println!("🎧 Client B listening on port 8081...");
        if let Err(e) = client_b_listener.listen(8081, "chat") {
            eprintln!("❌ Client B listener error: {}", e);
        }
    });

    // Give Client B time to start listening
    thread::sleep(Duration::from_millis(100));

    // Create Client B sender (sends to Client A on port 8080)
    let callback_b_sender = Box::new(|_: &Message| {});
    let mut client_b_sender = Client::connect("127.0.0.1:8080", callback_b_sender)?;
    println!("🔗 Client B sender connected to Client A");

    // Create Client A sender (sends to Client B on port 8081)
    let callback_a_sender = Box::new(|_: &Message| {});
    let mut client_a_sender = Client::connect("127.0.0.1:8081", callback_a_sender)?;
    println!("🔗 Client A sender connected to Client B\n");

    // Send messages back and forth
    println!("💬 Sending messages...\n");
    
    client_b_sender.send("chat", b"Hello from Client B!")?;
    thread::sleep(Duration::from_millis(100));
    
    client_a_sender.send("chat", b"Hi Client B, this is Client A!")?;
    thread::sleep(Duration::from_millis(100));
    
    client_b_sender.send("chat", b"How are you doing?")?;
    thread::sleep(Duration::from_millis(100));
    
    client_a_sender.send("chat", b"Doing great! Point-to-point works!")?;
    thread::sleep(Duration::from_millis(100));

    println!("\n✅ Communication complete!");

    // Keep threads alive for a bit
    thread::sleep(Duration::from_millis(500));
    
    Ok(())
}


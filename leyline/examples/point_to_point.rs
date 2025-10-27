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
    
    let alice_listener = Client::new(callback_a);
    
    // Start Client A listener in a thread
    thread::spawn(move || {
        println!("🎧 Client A listening on port 8080...");
        if let Err(e) = alice_listener.listen(8080, "chat") {
            eprintln!("❌ Client A listener error: {e}");
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
    
    let bob_listener = Client::new(callback_b);
    
    // Start Client B listener in a thread
    thread::spawn(move || {
        println!("🎧 Client B listening on port 8081...");
        if let Err(e) = bob_listener.listen(8081, "chat") {
            eprintln!("❌ Client B listener error: {e}");
        }
    });

    // Give Client B time to start listening
    thread::sleep(Duration::from_millis(100));

    // Create Client B sender (sends to Client A on port 8080)
    let bob_sender_callback = Box::new(|_: &Message| {});
    let mut bob_sender = Client::connect("127.0.0.1:8080", bob_sender_callback)?;
    println!("🔗 Client B sender connected to Client A");

    // Create Client A sender (sends to Client B on port 8081)
    let alice_sender_callback = Box::new(|_: &Message| {});
    let mut alice_sender = Client::connect("127.0.0.1:8081", alice_sender_callback)?;
    println!("🔗 Client A sender connected to Client B\n");

    // Send messages back and forth
    println!("💬 Sending messages...\n");
    
    bob_sender.send("chat", b"Hello from Client B!")?;
    thread::sleep(Duration::from_millis(100));
    
    alice_sender.send("chat", b"Hi Client B, this is Client A!")?;
    thread::sleep(Duration::from_millis(100));
    
    bob_sender.send("chat", b"How are you doing?")?;
    thread::sleep(Duration::from_millis(100));
    
    alice_sender.send("chat", b"Doing great! Point-to-point works!")?;
    thread::sleep(Duration::from_millis(100));

    println!("\n✅ Communication complete!");

    // Keep threads alive for a bit
    thread::sleep(Duration::from_millis(500));
    
    Ok(())
}


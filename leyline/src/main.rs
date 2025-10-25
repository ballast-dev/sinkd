use clap::{Parser, Subcommand};
use leyline::{Client, Message};

#[derive(Parser)]
#[command(name = "leyline")]
#[command(about = "Pure Rust IPC with TCP/IP pub/sub")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Publish a message to a topic
    Pub {
        /// Topic to publish to
        #[arg(short, long)]
        topic: String,
        /// Message content
        #[arg(short, long)]
        message: String,
        /// Target address (host:port)
        #[arg(long, default_value = "127.0.0.1:8080")]
        host: String,
    },
    /// Subscribe to a topic and listen for messages
    Sub {
        /// Topic to subscribe to
        #[arg(short, long)]
        topic: String,
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pub {
            topic,
            message,
            host,
        } => {
            // For publishing, create a client with connection to the target host
            let callback = Box::new(|_msg: &Message| {});
            let mut client = Client::connect(&host, callback)?;
            client.send(&topic, message.as_bytes())?;
            println!("✅ Message sent to {host} on topic '{topic}'");
        }
        Commands::Sub { topic, port } => {
            // Create callback that prints received messages
            let callback = Box::new(move |msg: &Message| {
                println!(
                    "🔔 Received on '{}': {}",
                    msg.topic,
                    String::from_utf8_lossy(&msg.payload)
                );
            });

            // Create a listener-only client (no outbound connection needed)
            let client = Client::new(callback);

            // Start listening in the main thread (blocks)
            client.listen(port, &topic)?;
        }
    }

    Ok(())
}

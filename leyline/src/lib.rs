use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub topic: String,
    pub payload: Vec<u8>,
}

pub type MessageCallback = Box<dyn Fn(&Message) + Send + Sync + 'static>;

pub struct Client {
    connection: Option<TcpStream>,
    callback: MessageCallback,
}

impl Client {
    /// Create a new client without connecting (listener-only mode initially).
    /// Use `connect_to()` later to establish an outbound connection.
    #[must_use] 
    pub fn new(callback: MessageCallback) -> Self {
        Self {
            connection: None,
            callback,
        }
    }

    /// Create a client with an immediate connection to a peer.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the connection to the specified address fails.
    pub fn connect(addr: &str, callback: MessageCallback) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(addr)?;
        Ok(Self {
            connection: Some(stream),
            callback,
        })
    }

    /// Connect to a peer (if not already connected).
    /// This allows you to create a listener-first client and connect later.
    /// 
    /// # Errors
    /// 
    /// Returns an error if the connection to the specified address fails.
    pub fn connect_to(&mut self, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.connection.is_none() {
            self.connection = Some(TcpStream::connect(addr)?);
        }
        Ok(())
    }

    /// Send a message using the established connection.
    /// Returns an error if not connected to any peer.
    /// 
    /// # Errors
    /// 
    /// Returns an error if not connected to any peer, if serialization fails,
    /// or if the network write operation fails.
    pub fn send(&mut self, topic: &str, payload: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let stream = self.connection.as_mut()
            .ok_or("Not connected to any peer. Call connect_to() first.")?;

        let msg = Message {
            topic: topic.to_string(),
            payload: payload.to_vec(),
        };
        let serialized = bincode::serialize(&msg)?;
        let len = u32::try_from(serialized.len())?;
        
        // Send length prefix (4 bytes)
        stream.write_all(&len.to_be_bytes())?;
        // Send message
        stream.write_all(&serialized)?;
        stream.flush()?;
        Ok(())
    }

    /// Start listening for incoming messages on a specific port and topic.
    /// 
    /// This is a blocking call that will not return unless an error occurs.
    /// **Important**: The implementer should wrap this in a thread for non-blocking behavior.
    /// 
    /// # Example - Point-to-Point Communication
    /// ```no_run
    /// use leyline::{Client, Message};
    /// use std::thread;
    /// 
    /// // Client A: Listen on 8080, send to B on 8081
    /// let callback = Box::new(|msg: &Message| {
    ///     println!("Received: {}", msg.topic);
    /// });
    /// 
    /// let mut client = Client::new(callback);
    /// client.connect_to("127.0.0.1:8081").unwrap();
    /// 
    /// // Spawn a thread for listening
    /// thread::spawn(move || {
    ///     client.listen(8080, "my-topic").unwrap();
    /// });
    /// ```
    /// 
    /// # Errors
    /// 
    /// Returns an error if binding to the port fails, if accepting connections fails,
    /// or if reading/deserializing messages fails.
    pub fn listen(&self, port: u16, topic: &str) -> Result<(), Box<dyn std::error::Error>> {

        // listen to all addresses on the given port on this host
        let listener = TcpListener::bind(format!("0.0.0.0:{port}"))?;
        println!("🎧 Listening on port {port} for topic '{topic}'");

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let peer_addr = stream.peer_addr()?;
                    let peer_ip = peer_addr.ip();
                    println!("📡 New connection from: {peer_ip}");
                    
                    // Read 4-byte length prefix
                    let mut len_bytes = [0u8; 4];
                    stream.read_exact(&mut len_bytes)?;
                    let len = u32::from_be_bytes(len_bytes) as usize;
                    
                    // Read the message
                    let mut buf = vec![0u8; len];
                    stream.read_exact(&mut buf)?;
                    
                    let msg = bincode::deserialize::<Message>(&buf)?;
                    (self.callback)(&msg);
                }
                Err(e) => {
                    println!("❌ Error: {e}");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            topic: "test-topic".to_string(),
            payload: vec![1, 2, 3, 4],
        };

        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: Message = bincode::deserialize(&serialized).unwrap();

        assert_eq!(msg.topic, deserialized.topic);
        assert_eq!(msg.payload, deserialized.payload);
    }

    #[test]
    fn test_message_with_utf8_payload() {
        let msg = Message {
            topic: "greetings".to_string(),
            payload: "Hello, World!".as_bytes().to_vec(),
        };

        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: Message = bincode::deserialize(&serialized).unwrap();

        assert_eq!(msg.topic, deserialized.topic);
        assert_eq!(
            String::from_utf8(deserialized.payload).unwrap(),
            "Hello, World!"
        );
    }

    #[test]
    fn test_client_send() {
        // Start a simple echo server in a thread
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let received = Arc::new(Mutex::new(None));
        let received_clone = Arc::clone(&received);

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                // Read length prefix
                let mut len_bytes = [0u8; 4];
                if stream.read_exact(&mut len_bytes).is_ok() {
                    let len = u32::from_be_bytes(len_bytes) as usize;
                    
                    // Read message
                    let mut buf = vec![0u8; len];
                    if stream.read_exact(&mut buf).is_ok()
                        && let Ok(msg) = bincode::deserialize::<Message>(&buf) {
                            *received_clone.lock().unwrap() = Some(msg);
                        }
                }
            }
        });

        // Give server time to start
        thread::sleep(Duration::from_millis(50));

        // Create client and send message
        let callback = Box::new(|_msg: &Message| {});
        let mut client = Client::connect(&addr.to_string(), callback).unwrap();
        client.send("test", b"hello").unwrap();

        // Give time for message to be received
        thread::sleep(Duration::from_millis(100));

        // Verify
        let received = received.lock().unwrap();
        assert!(received.is_some());
        let msg = received.as_ref().unwrap();
        assert_eq!(msg.topic, "test");
        assert_eq!(msg.payload, b"hello");
    }

    #[test]
    fn test_callback_invocation() {
        let callback_invoked = Arc::new(Mutex::new(false));
        let callback_invoked_clone = Arc::clone(&callback_invoked);

        let received_topic = Arc::new(Mutex::new(String::new()));
        let received_topic_clone = Arc::clone(&received_topic);

        let callback = Box::new(move |msg: &Message| {
            *callback_invoked_clone.lock().unwrap() = true;
            *received_topic_clone.lock().unwrap() = msg.topic.clone();
        });

        // Create a listener-only client
        let client = Client::new(callback);

        // Start listener in a thread
        let listen_port = 19999; // Use a specific port for this test
        thread::spawn(move || {
            client.listen(listen_port, "test-topic").ok();
        });

        thread::sleep(Duration::from_millis(100));

        // Send a message to the listener
        let sender_callback = Box::new(|_msg: &Message| {});
        let mut sender = Client::connect(&format!("127.0.0.1:{listen_port}"), sender_callback).unwrap();
        sender.send("test-topic", b"test payload").unwrap();

        // Give time for callback to be invoked
        thread::sleep(Duration::from_millis(200));

        // Verify callback was invoked
        assert!(*callback_invoked.lock().unwrap());
        assert_eq!(*received_topic.lock().unwrap(), "test-topic");
    }

    #[test]
    fn test_send_without_connection() {
        // Test that sending without connecting returns an error
        let callback = Box::new(|_msg: &Message| {});
        let mut client = Client::new(callback);
        
        let result = client.send("test", b"hello");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not connected"));
    }

    #[test]
    fn test_connect_to() {
        // Start a simple server
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut len_bytes = [0u8; 4];
                if stream.read_exact(&mut len_bytes).is_ok() {
                    let len = u32::from_be_bytes(len_bytes) as usize;
                    let mut buf = vec![0u8; len];
                    stream.read_exact(&mut buf).ok();
                }
            }
        });

        thread::sleep(Duration::from_millis(50));

        // Create client without connection
        let callback = Box::new(|_msg: &Message| {});
        let mut client = Client::new(callback);
        
        // Connect later
        client.connect_to(&addr.to_string()).unwrap();
        
        // Should now be able to send
        let result = client.send("test", b"hello");
        assert!(result.is_ok());
    }
}

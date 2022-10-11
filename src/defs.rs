use paho_mqtt;
enum SinkdError {}
pub type Outcome<T> = std::result::Result<T, paho_mqtt::Error>;

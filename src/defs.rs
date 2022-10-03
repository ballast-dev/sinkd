use paho_mqtt;
enum SinkdError {}
type Outcome<T> = std::result::Result<T, paho_mqtt::Error>;

// wrapper type to implement custom behavior
// Rust's orphan rule prevents aliasing and adding behavior to types
// outside of this crate's definiton
use std::fmt::Write;

#[derive(Debug)]
pub struct Failure(String);
pub type Outcome<T> = std::result::Result<T, Failure>;

#[macro_export]
macro_rules! bad {
    ($msg:expr) => {
        Err($msg.into()) // into will call From<T> with the right type
    };
    ($($arg:tt)*) => {
        Err(format!($($arg)*).into())
    };
}

impl std::error::Error for Failure {}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<std::io::Error> for Failure {
    fn from(value: std::io::Error) -> Self {
        Failure(value.to_string())
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Failure {
        Failure(message)
    }
}

impl From<&'static str> for Failure {
    fn from(message: &'static str) -> Failure {
        Failure(String::from(message))
    }
}

#[rustfmt::skip]
impl From<paho_mqtt::Error> for Failure {
    fn from(error: paho_mqtt::Error) -> Self {
        let mut err_str = String::from("ERROR Paho>> ");
        match error {
            paho_mqtt::Error::Publish(num, msg) => { let _ = write!(err_str, "publish num:{num}, msg:{msg}"); }
            paho_mqtt::Error::ReasonCode(code) => { let _ = write!(err_str, "mqttv5 reason code: {code}"); }
            paho_mqtt::Error::BadTopicFilter => { err_str.push_str("Bad Topic Filter"); }
            paho_mqtt::Error::Io(num) => { let _ = write!(err_str, "IO lowlevel: {num}"); }
            paho_mqtt::Error::Utf8(e) => { let _ = write!(err_str, "parsing UTF8 str: {e}"); }
            paho_mqtt::Error::Nul(_) => { err_str.push_str("Nul"); }
            paho_mqtt::Error::Conversion => { err_str.push_str("conversion between types"); }
            paho_mqtt::Error::Timeout => { err_str.push_str("timeout from synchronous operation"); }
            paho_mqtt::Error::General(msg) => { err_str.push_str(msg); }
            paho_mqtt::Error::GeneralString(msg) => { err_str.push_str(&msg); }
            paho_mqtt::Error::Failure => { err_str.push_str("Failure"); }
            paho_mqtt::Error::PersistenceError => { err_str.push_str("PersistenceError"); }
            paho_mqtt::Error::Disconnected => { err_str.push_str("Disconnected"); }
            paho_mqtt::Error::MaxMessagesInflight => { err_str.push_str("MaxMessagesInflight"); }
            paho_mqtt::Error::BadUtf8String => { err_str.push_str("BadUtf8String"); }
            paho_mqtt::Error::NullParameter => { err_str.push_str("NullParamenter"); }
            paho_mqtt::Error::TopicNameTruncated => { err_str.push_str("TopicNameTruncated"); }
            paho_mqtt::Error::BadStructure => { err_str.push_str("BadStructure"); }
            paho_mqtt::Error::BadQos => { err_str.push_str("BadQOS"); }
            paho_mqtt::Error::NoMoreMsgIds => { err_str.push_str("NoMoreMsgIds"); }
            paho_mqtt::Error::OperationIncomplete => { err_str.push_str("OperationIncomplete"); }
            paho_mqtt::Error::MaxBufferedMessages => { err_str.push_str("MaxBufferedMessages"); }
            paho_mqtt::Error::SslNotSupported => { err_str.push_str("SslNotSupported"); }
            paho_mqtt::Error::BadProtocol => { err_str.push_str("BadProtocol"); }
            paho_mqtt::Error::BadMqttOption => { err_str.push_str("BadMqttOption"); }
            paho_mqtt::Error::WrongMqttVersion => { err_str.push_str("WrongMqttVersion"); }
            paho_mqtt::Error::ZeroLenWillTopic => { err_str.push_str("ZeroLenWillTopic"); }
            paho_mqtt::Error::CommandIgnored => { err_str.push_str("CommandIgnored"); }
            paho_mqtt::Error::MaxBufferedZero => { err_str.push_str("MaxBufferedZero"); }
            paho_mqtt::Error::TcpConnectTimeout => { err_str.push_str("TcpConnectTimeout"); }
            paho_mqtt::Error::TcpConnectCompletionFailure => { err_str.push_str("TcpConnectCompletionFailure"); }
            paho_mqtt::Error::TcpTlsConnectFailure => { err_str.push_str("TcpTlsConnectFailure"); }
            paho_mqtt::Error::MissingSslOptions => { err_str.push_str("MissingSslOptions"); }
            paho_mqtt::Error::SocketError(socket_err) => { let _ = write!(err_str, "SocketError:{socket_err}"); }
            paho_mqtt::Error::ConnectReturn(connect_return_code) => { let _ = write!(err_str, "ConnectReturn {connect_return_code}"); }
            paho_mqtt::Error::ReceivedDisconnect(_) => { err_str.push_str("ReceivedDisconnect"); }
        }
        Failure(err_str)
    }
}

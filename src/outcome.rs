// wrapper type to implement custom behavior
// Rust's orphan rule prevents aliasing and adding behavior to types
// outside of this crate's definiton
#[derive(Debug)]
pub struct Failure(String);
pub type Outcome<T> = std::result::Result<T, Failure>;

macro_rules! bad {
    ($msg:expr) => {
       Outcome::Err($msg.into()) // into will call From<T> with the right type
    };
    ($($arg:tt)*) => {
        Outcome::Err(format!($($arg)*).into())
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
            paho_mqtt::Error::Paho(e) => { err_str.push_str(&format!("library, num: {}", e)); }
            paho_mqtt::Error::PahoDescr(num, msg) => { err_str.push_str(&format!("description(redundant):{}, {}", num, msg)); }
            paho_mqtt::Error::Publish(num, msg) => { err_str.push_str(&format!("publish num:{}, msg:{}", num, msg)); }
            paho_mqtt::Error::ReasonCode(code) => { err_str.push_str(&format!("mqttv5 reason code: {}", code)); }
            paho_mqtt::Error::BadTopicFilter => { err_str.push_str("Bad Topic Filter"); }
            paho_mqtt::Error::Io(num) => { err_str.push_str(&format!("IO lowlevel: {}", num)); }
            paho_mqtt::Error::Utf8(e) => { err_str.push_str(&format!("parsing UTF8 str: {}", e)); }
            paho_mqtt::Error::Nul(_) => { err_str.push_str("Nul"); }
            paho_mqtt::Error::Conversion => { err_str.push_str("conversion between types"); }
            paho_mqtt::Error::Timeout => { err_str.push_str("timeout from synchronous operation"); }
            paho_mqtt::Error::General(msg) => { err_str.push_str(msg); }
            paho_mqtt::Error::GeneralString(msg) => { err_str.push_str(&msg); }
        }
        Failure(err_str)
    }
}

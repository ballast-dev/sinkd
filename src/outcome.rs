#[derive(Debug)]
pub enum Bad {
    Stack(&'static str),
    Heap(String)
}

pub type Outcome<T> = std::result::Result<T, Bad>;

pub fn err_msg<O, E>(message: E) -> Outcome<O> where &'static str: From<E> {
    Err(Bad::Stack(message.into())) // into will call From<T> with the right type
}

pub fn err_str<O, E>(message: E) -> Outcome<O> where String: From<E> {
    Err(Bad::Heap(message.into())) // into will call From<T> with the right type
}

// trait ErrorMessage<T> {
//     fn msg(message: T) -> Bad;
// }

impl From<String> for Bad {
    fn from(message: String) -> Bad {
        Bad::Heap(message)
    }
}

impl From<&'static str> for Bad {
    fn from(message: &'static str) -> Bad {
        Bad::Stack(message)
    }
}


impl From<paho_mqtt::Error> for Bad {
    fn from(error: paho_mqtt::Error) -> Self {
        match error {
            paho_mqtt::Error::Paho(e) => Bad::Heap(format!("ERROR Paho> library, num: {}", e)),
            paho_mqtt::Error::PahoDescr(num, msg) => {
                Bad::Heap(format!("ERROR Paho> Description(redundant):{}, {}", num, msg))
            }
            paho_mqtt::Error::Publish(num, msg) => Bad::Heap(format!("ERROR Paho> publish num:{}, msg:{}", num, msg)),
            paho_mqtt::Error::ReasonCode(code) => Bad::Heap(format!("ERROR Paho> mqttv5 reason code: {}", code)),
            paho_mqtt::Error::BadTopicFilter => Bad::Stack("ERROR Paho> Bad Topic Filter"),
            paho_mqtt::Error::Io(num) => Bad::Heap(format!("ERROR Paho> IO lowlevel: {}", num)),
            paho_mqtt::Error::Utf8(e) => Bad::Heap(format!("ERROR Paho> parsing UTF8 str: {}", e)),
            paho_mqtt::Error::Nul(_) => Bad::Stack("ERROR Paho> Nul"),
            paho_mqtt::Error::Conversion => Bad::Stack("ERROR Paho> conversion between types"),
            paho_mqtt::Error::Timeout => Bad::Stack("ERROR Paho> timeout from synchronous operation"),
            paho_mqtt::Error::General(msg) => Bad::Heap(format!("ERROR Paho> {}", msg)),
            paho_mqtt::Error::GeneralString(msg) => Bad::Heap(format!("ERROR Paho> {}", msg)),
        }
    }
}

impl std::fmt::Display for Bad {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bad::Stack(msg) => write!(f, "{}", msg),
            Bad::Heap(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<std::io::Error> for Bad {
    fn from(value: std::io::Error) -> Self {
        Bad::Heap(value.to_string())
    }
}

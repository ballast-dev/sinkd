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
            paho_mqtt::Error::Paho(_) => todo!(),
            paho_mqtt::Error::PahoDescr(_, _) => todo!(),
            paho_mqtt::Error::Publish(_, _) => todo!(),
            paho_mqtt::Error::ReasonCode(_) => todo!(),
            paho_mqtt::Error::BadTopicFilter => todo!(),
            paho_mqtt::Error::Io(_) => todo!(),
            paho_mqtt::Error::Utf8(_) => todo!(),
            paho_mqtt::Error::Nul(_) => todo!(),
            paho_mqtt::Error::Conversion => todo!(),
            paho_mqtt::Error::Timeout => todo!(),
            paho_mqtt::Error::General(_) => todo!(),
            paho_mqtt::Error::GeneralString(_) => todo!(),
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
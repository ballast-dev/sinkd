// wrapper type to implement custom behavior
// Rust's orphan rule prevents aliasing and adding behavior to types
// outside of this crate's definiton

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

impl From<dust_dds::infrastructure::error::DdsError> for Failure {
    fn from(error: dust_dds::infrastructure::error::DdsError) -> Self {
        Failure(format!("DDS Error: {error:?}"))
    }
}

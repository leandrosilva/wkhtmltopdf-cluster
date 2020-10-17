use std::error::Error;
use std::fmt;
use std::io;
use std::process;
use std::convert::From;
use zmq;

type DynError = Box<dyn Error>;
type OptError = Option<DynError>;

pub type Result<T> = std::result::Result<T, AnyError>;

#[derive(Debug)]
pub struct AnyError {
    pub details: String,
    pub parent: OptError,
}

impl AnyError {
    pub fn new(details: &str, reason: OptError) -> Self {
        AnyError {
            details: details.to_string(),
            parent: reason,
        }
    }

    pub fn without_parent(details: &str) -> Self {
        AnyError::new(details, None)
    }
}

impl Error for AnyError {}

impl fmt::Display for AnyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.parent {
            Some(c) => write!(f, "{}: {}", self.details, c),
            None => write!(f, "{}", self.details),
        }
    }
}

impl From<io::Error> for AnyError {
    fn from(error: io::Error) -> Self {
        AnyError::new(&error.to_string(), Some(Box::new(error)))
    }
}

impl From<zmq::Error> for AnyError {
    fn from(error: zmq::Error) -> Self {
        AnyError::new(&error.to_string(), Some(Box::new(error)))
    }
}

pub fn error<T, U: 'static + Error>(message: &str, reason: U) -> Result<T> {
    Err(AnyError::new(&message, Some(Box::new(reason))))
}

pub fn error_without_parent<T>(message: &str) -> Result<T> {
    Err(AnyError::without_parent(&message))
}

pub fn exit_with_error<U: 'static + Error>(message: &str, reason: U) {
    eprintln!("{}: {}", message, reason);
    process::exit(666);
}

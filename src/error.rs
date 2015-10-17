use std::error;
use std::io;
use std::fmt;

#[derive(Debug)]
pub enum ProbeError {
    IO(io::Error),
    UnexpectedContent(String)
}

impl From<io::Error> for ProbeError {
    fn from(error: io::Error) -> ProbeError {
        ProbeError::IO(error)
    }
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ProbeError::IO(ref err) => write!(f, "{}", err),
            ProbeError::UnexpectedContent(ref err) => write!(f, "{}", err)
        }
    }
}

impl error::Error for ProbeError {
    fn description(&self) -> &str {
        match *self {
            ProbeError::IO(ref err) => err.description(),
            ProbeError::UnexpectedContent(ref err) => err
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ProbeError::IO(ref err) => Some(err),
            ProbeError::UnexpectedContent(_) => None
        }
    }
}

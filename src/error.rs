use std::error;
use std::io;
use std::fmt;

#[derive(Debug)]
pub enum ProbeError {
    /// IO error when opening file or command described in
    /// second field of the error
    IO(io::Error, String),
    /// Unexpected content in file or output
    UnexpectedContent(String),
    /// Input into a calculation function is invalid
    InvalidInput(String)
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ProbeError::IO(ref err, ref path) => write!(f, "{} for {}", err, path),
            ProbeError::UnexpectedContent(ref err) => write!(f, "{}", err),
            ProbeError::InvalidInput(ref err) => write!(f, "{}", err)
        }
    }
}

impl error::Error for ProbeError {
    fn description(&self) -> &str {
        match *self {
            ProbeError::IO(ref err, ref _path) => err.description(),
            ProbeError::UnexpectedContent(ref err) => err,
            ProbeError::InvalidInput(ref err) => err
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            ProbeError::IO(ref err, ref _path) => Some(err),
            ProbeError::UnexpectedContent(_) => None,
            ProbeError::InvalidInput(_) => None
        }
    }
}

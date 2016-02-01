use std::error;
use std::io;
use std::num;
use std::fmt;

#[derive(Debug)]
pub enum ProbeError {
    IO(io::Error),
    UnexpectedContent(String),
    ParseIntError(num::ParseIntError),
    ParseFloatError(num::ParseFloatError),

}

impl From<io::Error> for ProbeError {
    fn from(error: io::Error) -> ProbeError {
        ProbeError::IO(error)
    }
}

impl From<num::ParseIntError> for ProbeError {
    fn from(error: num::ParseIntError) -> ProbeError {
        ProbeError::ParseIntError(error)
    }
}

impl From<num::ParseFloatError> for ProbeError {
    fn from(error: num::ParseFloatError) -> ProbeError {
        ProbeError::ParseFloatError(error)
    }
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ProbeError::IO(ref err) => write!(f, "{}", err),
            ProbeError::UnexpectedContent(ref err) => write!(f, "{}", err),
            ProbeError::ParseIntError(ref err) => write!(f, "{}", err),
            ProbeError::ParseFloatError(ref err) => write!(f, "{}", err)
        }
    }
}

impl error::Error for ProbeError {
    fn description(&self) -> &str {
        match *self {
            ProbeError::IO(ref err) => err.description(),
            ProbeError::UnexpectedContent(ref err) => err,
            ProbeError::ParseIntError(ref err) => err.description(),
            ProbeError::ParseFloatError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ProbeError::IO(ref err) => Some(err),
            ProbeError::UnexpectedContent(_) => None,
            ProbeError::ParseIntError(ref err) => Some(err),
            ProbeError::ParseFloatError(ref err) => Some(err),
        }
    }
}

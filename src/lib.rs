extern crate libc;
extern crate time;

mod error;
pub mod disk_usage;
pub mod load;
pub mod memory;
pub mod cpu;
pub mod network;
pub mod process_memory;

use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;
use std::result;

pub use error::ProbeError;

pub type Result<T> = result::Result<T, error::ProbeError>;

#[inline]
fn file_to_string(path: &Path) -> io::Result<String> {
    let mut file = try!(fs::File::open(path));
    let mut read = String::new();
    try!(file.read_to_string(&mut read));
    Ok(read)
}

#[inline]
fn file_to_buf_reader(path: &Path) -> io::Result<io::BufReader<fs::File>> {
    fs::File::open(path).and_then(|f| Ok(io::BufReader::new(f)))
}

#[inline]
fn time_adjusted(first_value: u64, second_value: u64, time_difference_ns: u64) -> Result<u64> {
    if first_value < second_value {
        return Err(ProbeError::UnexpectedContent("value in first value was lower than in second value".to_string()))
    }
    Ok((first_value - second_value) * time_difference_ns / 60_000_000)
}

#[cfg(test)]
mod tests {
    use error::ProbeError;

    #[test]
    fn test_time_adjusted() {
        assert_eq!(1200, super::time_adjusted(2400, 1200, 60_000_000).unwrap());
        assert_eq!(600, super::time_adjusted(2400, 1200, 30_000_000).unwrap());
        assert_eq!(300, super::time_adjusted(2400, 1200, 15_000_000).unwrap());
    }

    #[test]
    fn test_time_adjusted_first_higher_than_lower() {
        match super::time_adjusted(1200, 2400, 60_000_000) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }
}

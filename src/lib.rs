extern crate libc;
extern crate time;

mod error;
pub mod cpu;
pub mod disk_stats;
pub mod disk_usage;
pub mod load;
pub mod memory;
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
fn calculate_time_difference(first_time: u64, second_time: u64) -> Result<u64> {
    if first_time > second_time {
        Err(ProbeError::InvalidInput(format!("first time {} was after second time {}", first_time, second_time)))
    } else {
        Ok(second_time - first_time)
    }
}

#[inline]
fn time_adjusted(field_name: &str, first_value: u64, second_value: u64, time_difference_ns: u64) -> Result<u64> {
    if first_value < second_value {
        Err(ProbeError::UnexpectedContent(format!("First value {} was lower than second value {} for '{}'", first_value, second_value, field_name)))
    } else {
        Ok((first_value - second_value) * time_difference_ns / 60_000_000)
    }
}

#[inline]
fn parse_u64(segment: &str) -> Result<u64> {
    segment.parse().map_err(|_| {
        ProbeError::UnexpectedContent(format!("Could not parse '{}' as u64", segment).to_owned())
    })
}

#[cfg(test)]
mod tests {
    use error::ProbeError;

    #[test]
    fn test_calculate_time_difference() {
        assert_eq!(100, super::calculate_time_difference(100, 200).unwrap());
        assert!(super::calculate_time_difference(200, 100).is_err());
    }

    #[test]
    fn test_time_adjusted() {
        assert_eq!(1200, super::time_adjusted("field", 2400, 1200, 60_000_000).unwrap());
        assert_eq!(600, super::time_adjusted("field", 2400, 1200, 30_000_000).unwrap());
        assert_eq!(300, super::time_adjusted("field", 2400, 1200, 15_000_000).unwrap());
    }

    #[test]
    fn test_time_adjusted_first_higher_than_lower() {
        match super::time_adjusted("field", 1200, 2400, 60_000_000) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_parse_u64() {
        assert_eq!(100, super::parse_u64("100").unwrap());
        assert!(super::parse_u64("something").is_err());
    }
}

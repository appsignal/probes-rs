#![feature(plugin)]
#![plugin(regex_macros)]
extern crate regex;

mod error;
pub mod load;
pub mod memory;

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
    let file = try!(fs::File::open(path));
    Ok(io::BufReader::new(file))
}

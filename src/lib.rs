#![feature(plugin)]
#![plugin(regex_macros)]
extern crate regex;

mod error;
pub mod load;

use std::fs;
use std::io;
use std::io::Read;
use std::path::Path;
use std::result;

pub use error::ProbeError;

pub type Result<T> = result::Result<T, error::ProbeError>;

#[inline]
fn read_file(path: &Path) -> io::Result<String> {
    let mut file = try!(fs::File::open(path));
    let mut read = String::new();
    try!(file.read_to_string(&mut read));
    Ok(read)
}

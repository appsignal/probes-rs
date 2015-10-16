#![feature(plugin)]
#![plugin(regex_macros)]
extern crate regex;

mod error;
pub mod load;

use std::result;

pub use error::ProbeError;

pub type Result<T> = result::Result<T, error::ProbeError>;

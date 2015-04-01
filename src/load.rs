use std::fs;
use std::io::{Read,Result};
use std::path::PathBuf;

use regex;

#[derive(Debug)]
pub struct LoadAverage {
    pub one:     f32,
    pub five:    f32,
    pub fifteen: f32
}

pub fn load_average() -> Result<LoadAverage> {
    let regex    = regex!(r"([0-9]+[\.,]\d+)");
    let raw_data = try!(read_load_avg());


    Ok(LoadAverage {
        one:     0.0,
        five:    0.0,
        fifteen: 0.0
    })
}

fn read_load_avg() -> Result<String> {
  let mut file = try!(fs::File::open(&proc_loadavg_path()));
  let mut read = String::new();
  try!(file.read_to_string(&mut read));
  Ok(read)
}

fn proc_loadavg_path() -> PathBuf {
    PathBuf::from("/proc/loadavg")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_load_average() {
        println!("{:?}", super::load_average());
        assert!(false);
    }
}

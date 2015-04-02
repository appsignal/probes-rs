use std::fs;
use std::io::{Read,Result};
use std::path::PathBuf;

#[derive(Debug)]
pub struct LoadAverage {
    pub one:     f32,
    pub five:    f32,
    pub fifteen: f32
}

pub fn load_average() -> Result<LoadAverage> {
    let raw_data = try!(read_load_avg());
    let segments: Vec<f32> = raw_data.split(" ").map(|segment|
        segment.parse().unwrap_or(0.0) // TODO remove unwrap
     ).collect();

    Ok(LoadAverage {
        one:     segments[0],
        five:    segments[1],
        fifteen: segments[2]
    })
}

#[cfg(target_os = "linux")]
fn read_load_avg() -> Result<String> {
  let mut file = try!(fs::File::open(&proc_loadavg_path()));
  let mut read = String::new();
  try!(file.read_to_string(&mut read));
  Ok(read)
}

#[cfg(target_os = "linux")]
fn proc_loadavg_path() -> PathBuf {
    PathBuf::from("/proc/loadavg")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_load_average() {
        let load_average = super::load_average().unwrap();
        assert!(load_average.one     > 0.0);
        assert!(load_average.five    > 0.0);
        assert!(load_average.fifteen > 0.0);
    }
}

use super::Result;

#[derive(Debug)]
pub struct LoadAverage {
    pub one:     f32,
    pub five:    f32,
    pub fifteen: f32
}

pub fn load_average() -> Result<LoadAverage> {
    os::load_average()
}

#[cfg(target_os = "linux")]
mod os {
    use std::path::Path;
    use std::fs;
    use std::io;
    use std::io::Read;

    use super::LoadAverage;
    use super::super::ProbeError;
    use super::super::Result;

    pub fn load_average() -> Result<LoadAverage> {
        read_and_parse_load_average(&Path::new("/proc/loadavg"))
    }

    pub fn read_and_parse_load_average(path: &Path) -> Result<LoadAverage> {
        let raw_data = try!(read_loadavg(path));
        let segments: Vec<f32> = raw_data.split(" ").map(|segment|
            segment.parse().unwrap_or(0.0)
        ).collect();

        if segments.len() < 3 {
            return Err(ProbeError::UnexpectedContent("Incorrect number of segments"))
        }

        Ok(LoadAverage {
            one:     segments[0],
            five:    segments[1],
            fifteen: segments[2]
        })
    }

    fn read_loadavg(path: &Path) -> io::Result<String> {
      let mut file = try!(fs::File::open(path));
      let mut read = String::new();
      try!(file.read_to_string(&mut read));
      Ok(read)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::super::ProbeError;

    #[test]
    fn test_load_average() {
        assert!(super::load_average().is_ok());
    }

    #[test]
    fn test_read_and_parse_load_average() {
        let path = Path::new("fixtures/linux/proc_loadavg");
        let load_average = super::os::read_and_parse_load_average(&path).unwrap();

        assert_eq!(load_average.one, 0.01);
        assert_eq!(load_average.five, 0.02);
        assert_eq!(load_average.fifteen, 0.03);
    }

    #[test]
    fn test_read_and_parse_load_average_wrong_path() {
        let path = Path::new("/nonsense");
        match super::os::read_and_parse_load_average(&path) {
            Err(ProbeError::IO(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_load_average_garbage_content() {
        let path = Path::new("fixtures/linux/proc_loadavg_garbage");
        match super::os::read_and_parse_load_average(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }
}

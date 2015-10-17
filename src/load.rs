use super::Result;

#[derive(Debug,PartialEq)]
pub struct LoadAverage {
    pub one:     f32,
    pub five:    f32,
    pub fifteen: f32
}

/// Read the current load average of the system.
#[cfg(target_os = "linux")]
pub fn read() -> Result<LoadAverage> {
    os::read()
}

#[cfg(target_os = "linux")]
mod os {
    use std::path::Path;

    use super::LoadAverage;
    use super::super::ProbeError;
    use super::super::Result;
    use super::super::read_file;

    #[inline]
    pub fn read() -> Result<LoadAverage> {
        read_and_parse_load_average(&Path::new("/proc/loadavg"))
    }

    #[inline]
    pub fn read_and_parse_load_average(path: &Path) -> Result<LoadAverage> {
        let raw_data = try!(read_file(path));
        let segments: Vec<&str> = raw_data.split_whitespace().collect();

        if segments.len() < 3 {
            return Err(ProbeError::UnexpectedContent("Incorrect number of segments".to_owned()))
        }

        Ok(LoadAverage {
            one:     try!(parse_segment(segments[0])),
            five:    try!(parse_segment(segments[1])),
            fifteen: try!(parse_segment(segments[2]))
        })
    }

    #[inline]
    fn parse_segment(segment: &str) -> Result<f32> {
        segment.parse().map_err(|_| {
            ProbeError::UnexpectedContent("Could not parse segment".to_owned())
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::super::ProbeError;
    use super::LoadAverage;

    #[test]
    fn test_read_load_average() {
        assert!(super::read().is_ok());
    }

    #[test]
    fn test_read_and_parse_load_average() {
        let path = Path::new("fixtures/linux/load/proc_loadavg");
        let load_average = super::os::read_and_parse_load_average(&path).unwrap();

        let expected = LoadAverage {
            one: 0.01,
            five: 0.02,
            fifteen: 0.03
        };

        assert_eq!(expected, load_average);
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
    fn test_read_and_parse_load_average_incomplete() {
        let path = Path::new("fixtures/linux/load/proc_loadavg_incomplete");
        match super::os::read_and_parse_load_average(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_load_average_garbage() {
        let path = Path::new("fixtures/linux/load/proc_loadavg_garbage");
        match super::os::read_and_parse_load_average(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }
}

use std::path::Path;
use error::ProbeError;
use super::Result;

/// Measurement of cpu stats at a certain time
#[derive(Debug,PartialEq)]
pub struct CpuMeasurement {
    pub precise_time_ns: u64,
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64
}

impl CpuMeasurement {
    /// Calculate the cpu stats based on this measurement and a measurement in the future.
    /// It is advisable to make the next measurement roughly a minute from this one for the
    /// most reliable result.
    pub fn calculate_per_minute(&self, next_measurement: &CpuMeasurement) -> Result<CpuStat> {
        if next_measurement.precise_time_ns < self.precise_time_ns {
            return Err(ProbeError::InvalidInput("time of next measurement was before time of this one".to_string()))
        }

        let time_difference = next_measurement.precise_time_ns - self.precise_time_ns;

        Ok(CpuStat {
            user: try!(super::time_adjusted(next_measurement.user, self.user, time_difference)),
            nice: try!(super::time_adjusted(next_measurement.nice, self.nice, time_difference)),
            system: try!(super::time_adjusted(next_measurement.system, self.system, time_difference)),
            idle: try!(super::time_adjusted(next_measurement.idle, self.idle, time_difference)),
            iowait: try!(super::time_adjusted(next_measurement.iowait, self.iowait, time_difference))
        })
    }
}

/// Cpu stats for a minute
#[derive(Debug,PartialEq)]
pub struct CpuStat {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64
}

#[cfg(target_os = "linux")]
pub fn read() -> Result<CpuMeasurement> {
    // columns: user nice system idle iowait irq softirq
    os::read_proc_cpu_stat(&Path::new("/proc/stat"))
}

#[cfg(target_os = "linux")]
mod os {
    use std::path::Path;
    use std::io::BufRead;
    use time;
    use super::super::{Result,file_to_buf_reader};
    use super::CpuMeasurement;
    use error::ProbeError;

    pub fn read_proc_cpu_stat(path: &Path) -> Result<CpuMeasurement> {
        let mut line = String::new();
        let mut reader = try!(file_to_buf_reader(path));
        let time = time::precise_time_ns();
        try!(reader.read_line(&mut line));

        let stats: Vec<&str> = line
            .split_whitespace()
            .skip(1)
            .collect();

        if stats.len() < 5 {
            return Err(ProbeError::UnexpectedContent("Incorrect number of stats".to_owned()));
        }

        Ok(CpuMeasurement {
            precise_time_ns: time,
            user: try!(parse_stat(stats[0])),
            nice: try!(parse_stat(stats[1])),
            system: try!(parse_stat(stats[2])),
            idle: try!(parse_stat(stats[3])),
            iowait: try!(parse_stat(stats[4]))
        })
    }

    fn parse_stat(stat: &str) -> Result<u64> {
        stat.parse().map_err(|_| {
            ProbeError::UnexpectedContent(format!("Could not parse stat {:?}", stat).to_owned())
        })
    }
}

#[cfg(test)]
mod test {
    use super::{CpuMeasurement,CpuStat};
    use super::os::read_proc_cpu_stat;
    use std::path::Path;
    use error::ProbeError;

    #[test]
    fn test_read_cpu_measurement() {
        let measurement = read_proc_cpu_stat(&Path::new("fixtures/linux/cpu_stat/proc_cpu_stat")).unwrap();
        assert_eq!(measurement.user, 0);
        assert_eq!(measurement.nice, 1);
        assert_eq!(measurement.system, 2);
        assert_eq!(measurement.idle, 3);
        assert_eq!(measurement.iowait, 4);
    }

    #[test]
    fn test_wrong_path() {
        match read_proc_cpu_stat(&Path::new("bananas")) {
            Err(ProbeError::IO(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_incomplete() {
        match read_proc_cpu_stat(&Path::new("fixtures/linux/cpu_stat/proc_cpu_stat_incomplete")) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_cpu_stat_garbage() {
        let path = Path::new("fixtures/linux/cpu_stat/proc_cpu_stat_garbage");
        match read_proc_cpu_stat(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_calculate_per_minute_wrong_times() {
        let measurement1 = CpuMeasurement {
            precise_time_ns: 90_000_000,
            user: 0,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0
        };

        let measurement2 = CpuMeasurement {
            precise_time_ns: 60_000_000,
            user: 0,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::InvalidInput(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_calculate_per_minute_full_minute() {
        let measurement1 = CpuMeasurement {
            precise_time_ns: 60_000_000,
            user: 1000,
            nice: 1100,
            system: 1200,
            idle: 1300,
            iowait: 1400
        };

        let measurement2 = CpuMeasurement {
            precise_time_ns: 120_000_000,
            user: 1006,
            nice: 1106,
            system: 1206,
            idle: 1306,
            iowait: 1406
        };

        let expected = CpuStat {
            user: 6,
            nice: 6,
            system: 6,
            idle: 6,
            iowait: 6
        };

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();

        assert_eq!(stat, expected);
    }

    #[test]
    fn test_calculate_per_minute_partial_minute() {
        let measurement1 = CpuMeasurement {
            precise_time_ns: 60_000_000,
            user: 1000,
            nice: 1100,
            system: 1200,
            idle: 1300,
            iowait: 1400
        };

        let measurement2 = CpuMeasurement {
            precise_time_ns: 90_000_000,
            user: 1006,
            nice: 1106,
            system: 1206,
            idle: 1306,
            iowait: 1406
        };

        let expected = CpuStat {
            user: 3,
            nice: 3,
            system: 3,
            idle: 3,
            iowait: 3
        };

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();

        assert_eq!(stat, expected);
    }

    #[test]
    fn test_calculate_per_minute_values_lower() {
        let measurement1 = CpuMeasurement {
            precise_time_ns: 60_000_000,
            user: 1000,
            nice: 1100,
            system: 1200,
            idle: 1300,
            iowait: 1400
        };

        let measurement2 = CpuMeasurement {
            precise_time_ns: 90_000_000,
            user: 106,
            nice: 116,
            system: 126,
            idle: 136,
            iowait: 146
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }
}

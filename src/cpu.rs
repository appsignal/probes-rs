use std::path::Path;
use super::{Result,calculate_time_difference};

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
        let time_difference = try!(calculate_time_difference(self.precise_time_ns, next_measurement.precise_time_ns));

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

impl CpuStat {
    /// Calculate the weight of the various components in percentages
    pub fn in_percentages(&self) -> CpuStatPercentages {
        let total = (self.user + self.system + self.idle) as f64;

        CpuStatPercentages {
            user: Self::percentage_of_total(self.user, total),
            nice: Self::percentage_of_total(self.nice, total),
            system: Self::percentage_of_total(self.system, total),
            idle: Self::percentage_of_total(self.idle, total),
            iowait: Self::percentage_of_total(self.iowait, total)
        }
    }

    fn percentage_of_total(value: u64, total: f64) -> f32 {
        (value as f64 / total * 100.0) as f32
    }
}

/// Cpu stats converted to percentages
#[derive(Debug,PartialEq)]
pub struct CpuStatPercentages {
    pub user: f32,
    pub nice: f32,
    pub system: f32,
    pub idle: f32,
    pub iowait: f32
}

#[cfg(target_os = "linux")]
pub fn read() -> Result<CpuMeasurement> {
    // columns: user nice system idle iowait irq softirq
    os::read_and_parse_proc_stat(&Path::new("/proc/stat"))
}

#[cfg(target_os = "linux")]
mod os {
    use std::path::Path;
    use std::io::BufRead;
    use time;
    use super::super::{Result,file_to_buf_reader,parse_u64};
    use super::CpuMeasurement;
    use error::ProbeError;

    pub fn read_and_parse_proc_stat(path: &Path) -> Result<CpuMeasurement> {
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
            user: try!(parse_u64(stats[0])),
            nice: try!(parse_u64(stats[1])),
            system: try!(parse_u64(stats[2])),
            idle: try!(parse_u64(stats[3])),
            iowait: try!(parse_u64(stats[4]))
        })
    }
}

#[cfg(test)]
mod test {
    use super::{CpuMeasurement,CpuStat,CpuStatPercentages};
    use super::os::read_and_parse_proc_stat;
    use std::path::Path;
    use error::ProbeError;

    #[test]
    fn test_read_cpu_measurement() {
        let measurement = read_and_parse_proc_stat(&Path::new("fixtures/linux/cpu/proc_stat")).unwrap();
        assert_eq!(measurement.user, 0);
        assert_eq!(measurement.nice, 1);
        assert_eq!(measurement.system, 2);
        assert_eq!(measurement.idle, 3);
        assert_eq!(measurement.iowait, 4);
    }

    #[test]
    fn test_wrong_path() {
        match read_and_parse_proc_stat(&Path::new("bananas")) {
            Err(ProbeError::IO(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_proc_stat_incomplete() {
        match read_and_parse_proc_stat(&Path::new("fixtures/linux/cpu/proc_stat_incomplete")) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_proc_stat_garbage() {
        let path = Path::new("fixtures/linux/cpu/proc_stat_garbage");
        match read_and_parse_proc_stat(&path) {
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

    #[test]
    fn test_in_percentages() {
        let stat = CpuStat {
            user: 500,
            nice: 100,
            system: 250,
            idle: 250,
            iowait: 100
        };

        let expected = CpuStatPercentages {
            user: 50.0,
            nice: 10.0,
            system: 25.0,
            idle: 25.0,
            iowait: 10.0
        };

        assert_eq!(stat.in_percentages(), expected);
    }

    #[test]
    fn test_in_percentages_fractions() {
        let stat = CpuStat {
            user: 495,
            nice: 100,
            system: 250,
            idle: 255,
            iowait: 100
        };

        let expected = CpuStatPercentages {
            user: 49.5,
            nice: 10.0,
            system: 25.0,
            idle: 25.5,
            iowait: 10.0
        };

        assert_eq!(stat.in_percentages(), expected);
    }

    #[test]
    fn test_in_percentages_integration() {
        let measurement1 = read_and_parse_proc_stat(&Path::new("fixtures/linux/cpu/proc_stat_1")).unwrap();
        let measurement2 = read_and_parse_proc_stat(&Path::new("fixtures/linux/cpu/proc_stat_2")).unwrap();
        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();
        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.

        assert!(in_percentages.user > 4.6);
        assert!(in_percentages.user < 4.7);

        assert!(in_percentages.nice < 0.1);

        assert!(in_percentages.system > 1.4);
        assert!(in_percentages.system < 1.5);

        assert!(in_percentages.idle > 93.8);
        assert!(in_percentages.idle < 94.0);

        assert!(in_percentages.iowait < 0.1);

        // The total of all values should be 100.

        let total = in_percentages.user + in_percentages.nice + in_percentages.system +
                      in_percentages.idle + in_percentages.iowait;

        assert!(total < 100.1);
        assert!(total > 99.9);
    }
}

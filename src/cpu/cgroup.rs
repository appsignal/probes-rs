use super::super::{Result,calculate_time_difference,time_adjusted};

/// Measurement of cpu stats at a certain time
#[derive(Debug,PartialEq)]
pub struct CpuMeasurement {
    pub precise_time_ns: u64,
    pub stat: CpuStat
}

impl CpuMeasurement {
    /// Calculate the cpu stats based on this measurement and a measurement in the future.
    /// It is advisable to make the next measurement roughly a minute from this one for the
    /// most reliable result.
    pub fn calculate_per_minute(&self, next_measurement: &CpuMeasurement) -> Result<CpuStat> {
        let time_difference = calculate_time_difference(self.precise_time_ns, next_measurement.precise_time_ns)?;

        Ok(CpuStat {
            total: time_adjusted("total", next_measurement.stat.total, self.stat.total, time_difference)?,
            user: time_adjusted("user", next_measurement.stat.user, self.stat.user, time_difference)?,
            nice: time_adjusted("nice", next_measurement.stat.nice, self.stat.nice, time_difference)?,
            system: time_adjusted("system", next_measurement.stat.system, self.stat.system, time_difference)?,
            idle: time_adjusted("idle", next_measurement.stat.idle, self.stat.idle, time_difference)?,
            iowait: time_adjusted("iowait", next_measurement.stat.iowait, self.stat.iowait, time_difference)?,
            irq: time_adjusted("irq", next_measurement.stat.irq, self.stat.irq, time_difference)?,
            softirq: time_adjusted("softirq", next_measurement.stat.softirq, self.stat.softirq, time_difference)?,
            steal: time_adjusted("steal", next_measurement.stat.steal, self.stat.steal, time_difference)?,
            guest: time_adjusted("guest", next_measurement.stat.guest, self.stat.guest, time_difference)?,
            guestnice: time_adjusted("guestnice", next_measurement.stat.guestnice, self.stat.guestnice, time_difference)?
        })
    }
}

/// Cpu stats for a minute
#[derive(Debug,PartialEq)]
pub struct CpuStat {
    pub total: u64,
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
    pub guest: u64,
    pub guestnice: u64
}

impl CpuStat {
    /// Calculate the weight of the various components in percentages
    pub fn in_percentages(&self) -> CpuStatPercentages {
        CpuStatPercentages {
            user: self.percentage_of_total(self.user),
            nice: self.percentage_of_total(self.nice),
            system: self.percentage_of_total(self.system),
            idle: self.percentage_of_total(self.idle),
            iowait: self.percentage_of_total(self.iowait),
            irq: self.percentage_of_total(self.irq),
            softirq: self.percentage_of_total(self.softirq),
            steal: self.percentage_of_total(self.steal),
            guest: self.percentage_of_total(self.guest),
            guestnice: self.percentage_of_total(self.guestnice)
        }
    }

    fn percentage_of_total(&self, value: u64) -> f32 {
        (value as f64 / self.total as f64 * 100.0) as f32
    }
}

/// Cpu stats converted to percentages
#[derive(Debug,PartialEq)]
pub struct CpuStatPercentages {
    pub user: f32,
    pub nice: f32,
    pub system: f32,
    pub idle: f32,
    pub iowait: f32,
    pub irq: f32,
    pub softirq: f32,
    pub steal: f32,
    pub guest: f32,
    pub guestnice: f32
}

/// Read the current CPU stats of the system.
#[cfg(target_os = "linux")]
pub fn read() -> Result<CpuMeasurement> {
    os::read()
}

#[cfg(target_os = "linux")]
mod os {
    use std::path::Path;
    use std::io::BufRead;
    use time;
    use super::super::super::{Result,file_to_buf_reader,parse_u64,path_to_string,read_file_value_as_u64,dir_exists};
    use super::{CpuMeasurement,CpuStat};
    use error::ProbeError;

    const CPU_SYS_NUMBER_OF_FIELDS: usize = 2;

    #[inline]
    pub fn read() -> Result<CpuMeasurement> {
        let sys_fs_dir = Path::new("/sys/fs/cgroup/cpuacct/");
        if dir_exists(sys_fs_dir) {
            read_and_parse_sys_stat(&sys_fs_dir)
        } else {
            let message = format!("Directory `{}` not found", sys_fs_dir.to_str().unwrap_or("unknown path"));
            Err(ProbeError::UnexpectedContent(message))
        }
    }

    pub fn read_and_parse_sys_stat(path: &Path) -> Result<CpuMeasurement> {
        let time = time::precise_time_ns();
        let reader = file_to_buf_reader(&path.join("cpuacct.stat"))?;
        let total = nano_to_user(read_file_value_as_u64(&path.join("cpuacct.usage"))?);

        let mut cpu = CpuStat {
            total: total,
            user: 0,
            system: 0,
            nice: 0,
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guestnice: 0
        };

        let mut fields_encountered = 0;
        for line in reader.lines() {
            let line = line.map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
            let segments: Vec<&str> = line.split_whitespace().collect();
            let value = parse_u64(&segments[1])?;
            fields_encountered += match segments[0] {
                "user" => {
                    cpu.user = value;
                    1
                },
                "system" => {
                    cpu.system = value;
                    1
                },
                _ => 0
            };

            if fields_encountered == CPU_SYS_NUMBER_OF_FIELDS {
                break
            }
        }

        if fields_encountered != CPU_SYS_NUMBER_OF_FIELDS {
            return Err(ProbeError::UnexpectedContent("Did not encounter all expected fields".to_owned()))
        }

        Ok(CpuMeasurement {
            precise_time_ns: time,
            stat: cpu
        })
    }

    // [CPU usage] times are expressed in ticks of 1/100th of a second, also called "user jiffies".
    // There are USER_HZ “jiffies” per second, and on x86 systems, USER_HZ is 100.
    // See: https://docs.docker.com/config/containers/runmetrics/#cpu-metrics-cpuacctstat
    fn nano_to_user(value: u64) -> u64 {
        value.checked_div(10_000_000).unwrap_or(0)
    }
}

#[cfg(test)]
mod test {
    use super::{CpuMeasurement,CpuStat,CpuStatPercentages};
    use super::os::read_and_parse_sys_stat;
    use std::path::Path;
    use error::ProbeError;

    #[test]
    fn test_read_cpu() {
        assert!(super::read().is_ok());
    }

    #[test]
    fn test_read_sys_measurement() {
        let measurement = read_and_parse_sys_stat(&Path::new("fixtures/linux/sys/fs/cgroup/cpuacct_1/")).unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total, 1395);
        assert_eq!(cpu.user, 404);
        assert_eq!(cpu.nice, 0);
        assert_eq!(cpu.system, 749);
        assert_eq!(cpu.idle, 0);
        assert_eq!(cpu.iowait, 0);
        assert_eq!(cpu.irq, 0);
        assert_eq!(cpu.softirq, 0);
        assert_eq!(cpu.steal, 0);
        assert_eq!(cpu.guest, 0);
        assert_eq!(cpu.guestnice, 0);
    }

    #[test]
    fn test_read_sys_wrong_path() {
        match read_and_parse_sys_stat(&Path::new("bananas")) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_sys_stat_incomplete() {
        match read_and_parse_sys_stat(&Path::new("fixtures/linux/sys/fs/cgroup/cpuacct_incomplete/")) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_sys_stat_garbage() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/cpuacct_garbage/");
        match read_and_parse_sys_stat(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_calculate_per_minute_wrong_times() {
        let measurement1 = CpuMeasurement {
            precise_time_ns: 90_000_000_000,
            stat: CpuStat {
                total: 0,
                user: 0,
                nice: 0,
                system: 0,
                idle: 0,
                iowait: 0,
                irq: 0,
                softirq: 0,
                steal: 0,
                guest: 0,
                guestnice: 0
            }
        };

        let measurement2 = CpuMeasurement {
            precise_time_ns: 60_000_000_000,
            stat: CpuStat {
                total: 0,
                user: 0,
                nice: 0,
                system: 0,
                idle: 0,
                iowait: 0,
                irq: 0,
                softirq: 0,
                steal: 0,
                guest: 0,
                guestnice: 0
            }
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::InvalidInput(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_calculate_per_minute_full_minute() {
        let measurement1 = CpuMeasurement {
            precise_time_ns: 60_000_000_000,
            stat: CpuStat {
                total: 6380,
                user: 1000,
                nice: 1100,
                system: 1200,
                idle: 1300,
                iowait: 1400,
                irq: 50,
                softirq: 10,
                steal: 20,
                guest: 200,
                guestnice: 100
            }
        };

        let measurement2 = CpuMeasurement {
            precise_time_ns: 120_000_000_000,
            stat: CpuStat {
                total: 6440,
                user: 1006,
                nice: 1106,
                system: 1206,
                idle: 1306,
                iowait: 1406,
                irq: 56,
                softirq: 16,
                steal: 26,
                guest: 206,
                guestnice: 106
            }
        };

        let expected = CpuStat {
            total: 60,
            user: 6,
            nice: 6,
            system: 6,
            idle: 6,
            iowait: 6,
            irq: 6,
            softirq: 6,
            steal: 6,
            guest: 6,
            guestnice: 6
        };

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();

        assert_eq!(stat, expected);
    }

    #[test]
    fn test_calculate_per_minute_partial_minute() {
        let measurement1 = CpuMeasurement {
            precise_time_ns: 60_000_000_000,
            stat: CpuStat {
                total: 6380,
                user: 1000,
                nice: 1100,
                system: 1200,
                idle: 1300,
                iowait: 1400,
                irq: 50,
                softirq: 10,
                steal: 20,
                guest: 200,
                guestnice: 100
            }
        };

        let measurement2 = CpuMeasurement {
            precise_time_ns: 90_000_000_000,
            stat: CpuStat {
                total: 6440,
                user: 1006,
                nice: 1106,
                system: 1206,
                idle: 1306,
                iowait: 1406,
                irq: 56,
                softirq: 16,
                steal: 26,
                guest: 206,
                guestnice: 106
            }
        };

        let expected = CpuStat {
            total: 120,
            user: 12,
            nice: 12,
            system: 12,
            idle: 12,
            iowait: 12,
            irq: 12,
            softirq: 12,
            steal: 12,
            guest: 12,
            guestnice: 12
        };

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();

        assert_eq!(stat, expected);
    }

    #[test]
    fn test_calculate_per_minute_values_lower() {
        let measurement1 = CpuMeasurement {
            precise_time_ns: 60_000_000_000,
            stat: CpuStat {
                total: 6380,
                user: 1000,
                nice: 1100,
                system: 1200,
                idle: 1300,
                iowait: 1400,
                irq: 50,
                softirq: 10,
                steal: 20,
                guest: 200,
                guestnice: 100
            }
        };

        let measurement2 = CpuMeasurement {
            precise_time_ns: 90_000_000_000,
            stat: CpuStat {
                total: 1040,
                user: 106,
                nice: 116,
                system: 126,
                idle: 136,
                iowait: 146,
                irq: 56,
                softirq: 16,
                steal: 26,
                guest: 206,
                guestnice: 106
            }
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_in_percentages() {
        let stat = CpuStat {
            total: 1000,
            user: 450,
            nice: 70,
            system: 100,
            idle: 100,
            iowait: 120,
            irq: 10,
            softirq: 20,
            steal: 50,
            guest: 50,
            guestnice: 30
        };

        let expected = CpuStatPercentages {
            user: 45.0,
            nice: 7.0,
            system: 10.0,
            idle: 10.0,
            iowait: 12.0,
            irq: 1.0,
            softirq: 2.0,
            steal: 5.0,
            guest: 5.0,
            guestnice: 3.0
        };

        assert_eq!(stat.in_percentages(), expected);
    }

    #[test]
    fn test_in_percentages_fractions() {
        let stat = CpuStat {
            total: 1000,
            user: 445,
            nice: 65,
            system: 100,
            idle: 100,
            iowait: 147,
            irq: 1,
            softirq: 2,
            steal: 50,
            guest: 55,
            guestnice: 35
        };

        let expected = CpuStatPercentages {
            user: 44.5,
            nice: 6.5,
            system: 10.0,
            idle: 10.0,
            iowait: 14.7,
            irq: 0.1,
            softirq: 0.2,
            steal: 5.0,
            guest: 5.5,
            guestnice: 3.5
        };

        assert_eq!(stat.in_percentages(), expected);
    }

    #[test]
    fn test_in_percentages_integration() {
        let mut measurement1 = read_and_parse_sys_stat(&Path::new("fixtures/linux/sys/fs/cgroup/cpuacct_1")).unwrap();
        measurement1.precise_time_ns = 60_000_000_000;
        let mut measurement2 = read_and_parse_sys_stat(&Path::new("fixtures/linux/sys/fs/cgroup/cpuacct_2")).unwrap();
        measurement2.precise_time_ns = 120_000_000_000;

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();
        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.

        assert!(in_percentages.user >= 10.0);
        assert!(in_percentages.user <= 10.0);

        assert!(in_percentages.system >= 10.0);
        assert!(in_percentages.system <= 10.0);
    }
}

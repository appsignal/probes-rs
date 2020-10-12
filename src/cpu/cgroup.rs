use super::super::{calculate_time_difference, time_adjusted, Result};

/// Measurement of cpu stats at a certain time
#[derive(Debug, PartialEq)]
pub struct CgroupCpuMeasurement {
    pub precise_time_ns: u64,
    pub stat: CgroupCpuStat,
}

impl CgroupCpuMeasurement {
    pub fn calculate_per_minute(
        &self,
        next_measurement: &CgroupCpuMeasurement,
    ) -> Result<CgroupCpuStat> {
        let time_difference =
            calculate_time_difference(self.precise_time_ns, next_measurement.precise_time_ns)?;

        Ok(CgroupCpuStat {
            total_usage: time_adjusted(
                "total_usage",
                next_measurement.stat.total_usage,
                self.stat.total_usage,
                time_difference,
            )?,
            user: time_adjusted(
                "user",
                next_measurement.stat.user,
                self.stat.user,
                time_difference,
            )?,
            system: time_adjusted(
                "system",
                next_measurement.stat.system,
                self.stat.system,
                time_difference,
            )?,
        })
    }
}

/// Container CPU stats for a minute
#[derive(Debug, PartialEq)]
pub struct CgroupCpuStat {
    pub total_usage: u64,
    pub user: u64,
    pub system: u64,
}

impl CgroupCpuStat {
    /// Calculate the weight of the various components in percentages
    pub fn in_percentages(&self) -> CgroupCpuStatPercentages {
        CgroupCpuStatPercentages {
            total_usage: self.percentage_of_total(self.total_usage),
            user: self.percentage_of_total(self.user),
            system: self.percentage_of_total(self.system),
        }
    }

    fn percentage_of_total(&self, value: u64) -> f32 {
        // 60_000_000_000 being the total value. This is 60 seconds expressed in nanoseconds.
        (value as f32 / 60_000_000_000.0) * 100.0
    }
}

/// Cgroup Cpu stats converted to percentages
#[derive(Debug, PartialEq)]
pub struct CgroupCpuStatPercentages {
    pub total_usage: f32,
    pub user: f32,
    pub system: f32,
}

/// Read the current CPU stats of the container.
#[cfg(target_os = "linux")]
pub fn read() -> Result<CgroupCpuMeasurement> {
    os::read()
}

#[cfg(target_os = "linux")]
mod os {
    use super::super::super::{
        dir_exists, file_to_buf_reader, parse_u64, path_to_string, precise_time_ns,
        read_file_value_as_u64, Result,
    };
    use super::{CgroupCpuMeasurement, CgroupCpuStat};
    use crate::error::ProbeError;
    use std::io::BufRead;
    use std::path::Path;

    const CPU_SYS_NUMBER_OF_FIELDS: usize = 2;

    pub fn read() -> Result<CgroupCpuMeasurement> {
        let sys_fs_dir = Path::new("/sys/fs/cgroup/cpuacct/");
        if dir_exists(sys_fs_dir) {
            read_and_parse_sys_stat(&sys_fs_dir)
        } else {
            let message = format!(
                "Directory `{}` not found",
                sys_fs_dir.to_str().unwrap_or("unknown path")
            );
            Err(ProbeError::UnexpectedContent(message))
        }
    }

    pub fn read_and_parse_sys_stat(path: &Path) -> Result<CgroupCpuMeasurement> {
        let time = precise_time_ns();

        let reader = file_to_buf_reader(&path.join("cpuacct.stat"))?;
        let total_usage = read_file_value_as_u64(&path.join("cpuacct.usage"))?;

        let mut cpu = CgroupCpuStat {
            total_usage,
            user: 0,
            system: 0,
        };

        let mut fields_encountered = 0;
        for line in reader.lines() {
            let line = line.map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
            let segments: Vec<&str> = line.split_whitespace().collect();
            let value = parse_u64(&segments[1])?;
            fields_encountered += match segments[0] {
                "user" => {
                    cpu.user = value * 10_000_000;
                    1
                }
                "system" => {
                    cpu.system = value * 10_000_000;
                    1
                }
                _ => 0,
            };

            if fields_encountered == CPU_SYS_NUMBER_OF_FIELDS {
                break;
            }
        }

        if fields_encountered != CPU_SYS_NUMBER_OF_FIELDS {
            return Err(ProbeError::UnexpectedContent(
                "Did not encounter all expected fields".to_owned(),
            ));
        }
        let measurement = CgroupCpuMeasurement {
            precise_time_ns: time,
            stat: cpu,
        };
        Ok(measurement)
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod test {
    use super::os::read_and_parse_sys_stat;
    use super::{CgroupCpuMeasurement, CgroupCpuStat};
    use crate::error::ProbeError;
    use std::path::Path;

    #[test]
    fn test_read() {
        assert!(super::read().is_ok());
    }

    #[test]
    fn test_read_sys_measurement() {
        let measurement =
            read_and_parse_sys_stat(&Path::new("fixtures/linux/sys/fs/cgroup/cpuacct_1/")).unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 152657213021);
        assert_eq!(cpu.user, 149340000000);
        assert_eq!(cpu.system, 980000000);
    }

    #[test]
    fn test_read_sys_wrong_path() {
        match read_and_parse_sys_stat(&Path::new("bananas")) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_sys_stat_incomplete() {
        match read_and_parse_sys_stat(&Path::new(
            "fixtures/linux/sys/fs/cgroup/cpuacct_incomplete/",
        )) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_sys_stat_garbage() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/cpuacct_garbage/");
        match read_and_parse_sys_stat(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_calculate_per_minute_wrong_times() {
        let measurement1 = CgroupCpuMeasurement {
            precise_time_ns: 90_000_000_000,
            stat: CgroupCpuStat {
                total_usage: 0,
                user: 0,
                system: 0,
            },
        };

        let measurement2 = CgroupCpuMeasurement {
            precise_time_ns: 60_000_000_000,
            stat: CgroupCpuStat {
                total_usage: 0,
                user: 0,
                system: 0,
            },
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::InvalidInput(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_cgroup_calculate_per_minute_full_minute() {
        let measurement1 = CgroupCpuMeasurement {
            precise_time_ns: 60_000_000_000,
            stat: CgroupCpuStat {
                total_usage: 6380,
                user: 1000,
                system: 1200,
            },
        };

        let measurement2 = CgroupCpuMeasurement {
            precise_time_ns: 120_000_000_000,
            stat: CgroupCpuStat {
                total_usage: 6440,
                user: 1006,
                system: 1206,
            },
        };

        let expected = CgroupCpuStat {
            total_usage: 60,
            user: 6,
            system: 6,
        };

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();

        assert_eq!(stat, expected);
    }

    #[test]
    fn test_calculate_per_minute_partial_minute() {
        let measurement1 = CgroupCpuMeasurement {
            precise_time_ns: 60_000_000_000,
            stat: CgroupCpuStat {
                total_usage: 1_000_000_000,
                user: 10000_000_000,
                system: 12000_000_000,
            },
        };

        let measurement2 = CgroupCpuMeasurement {
            precise_time_ns: 90_000_000_000,
            stat: CgroupCpuStat {
                total_usage: 1_500_000_000,
                user: 10060_000_000,
                system: 12060_000_000,
            },
        };

        let expected = CgroupCpuStat {
            total_usage: 1_000_000_000,
            user: 120_000_000,
            system: 120_000_000,
        };

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();

        assert_eq!(stat, expected);
    }

    #[test]
    fn test_calculate_per_minute_values_lower() {
        let measurement1 = CgroupCpuMeasurement {
            precise_time_ns: 60_000_000_000,
            stat: CgroupCpuStat {
                total_usage: 63800_000_000,
                user: 10000_000_000,
                system: 12000_000_000,
            },
        };

        let measurement2 = CgroupCpuMeasurement {
            precise_time_ns: 90_000_000_000,
            stat: CgroupCpuStat {
                total_usage: 10400_000_000,
                user: 1060_000_000,
                system: 1260_000_000,
            },
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_in_percentages() {
        let stat = CgroupCpuStat {
            total_usage: 24000000000,
            user: 16800000000,
            system: 1200000000,
        };

        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.
        assert!(in_percentages.total_usage > 39.9);
        assert!(in_percentages.total_usage <= 40.0);

        assert!(in_percentages.user > 27.9);
        assert!(in_percentages.user <= 28.0);

        assert!(in_percentages.system > 1.9);
        assert!(in_percentages.system <= 2.0);
    }

    #[test]
    fn test_in_percentages_fractions() {
        let stat = CgroupCpuStat {
            total_usage: 24000000000,
            user: 17100000000,
            system: 900000000,
        };

        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.
        assert!(in_percentages.total_usage > 39.9);
        assert!(in_percentages.total_usage <= 40.0);

        assert!(in_percentages.user > 28.4);
        assert!(in_percentages.user <= 28.5);

        assert!(in_percentages.system > 1.4);
        assert!(in_percentages.system <= 1.5);
    }

    #[test]
    fn test_in_percentages_integration() {
        let mut measurement1 =
            read_and_parse_sys_stat(&Path::new("fixtures/linux/sys/fs/cgroup/cpuacct_1/")).unwrap();
        measurement1.precise_time_ns = 375953965125920;
        let mut measurement2 =
            read_and_parse_sys_stat(&Path::new("fixtures/linux/sys/fs/cgroup/cpuacct_2/")).unwrap();
        measurement2.precise_time_ns = 376013815302920;

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();
        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.
        assert!(in_percentages.total_usage > 49.70);
        assert!(in_percentages.total_usage < 49.71);

        assert!(in_percentages.user > 47.60);
        assert!(in_percentages.user < 47.61);

        assert!(in_percentages.system > 0.38);
        assert!(in_percentages.system < 0.39);
    }
}

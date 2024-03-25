use crate::error::ProbeError;
use crate::{calculate_time_difference, dir_exists, time_adjusted, Result};
use std::path::Path;

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

    // Divide the values by the number of (potentially fractional) CPUs allocated to the system.
    pub fn by_cpu_count(&self, cpu_count: Option<f64>) -> CgroupCpuStat {
        let cpu_count = cpu_count.filter(|count| *count != 0.0).unwrap_or(1.0);

        CgroupCpuStat {
            total_usage: (self.total_usage as f64 / cpu_count).round() as u64,
            user: (self.user as f64 / cpu_count).round() as u64,
            system: (self.system as f64 / cpu_count).round() as u64,
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
pub fn read(cpu_count: Option<f64>) -> Result<CgroupCpuMeasurement> {
    use super::cgroup_v1::read_and_parse_v1_sys_stat;
    use super::cgroup_v2::read_and_parse_v2_sys_stat;

    let v2_sys_fs_file = Path::new("/sys/fs/cgroup/cpu.stat");
    if v2_sys_fs_file.exists() {
        let v2_sys_fs_cpu_max_file = Path::new("/sys/fs/cgroup/cpu.max");
        return read_and_parse_v2_sys_stat(&v2_sys_fs_file, v2_sys_fs_cpu_max_file, cpu_count);
    }

    let v1_sys_fs_dir = Path::new("/sys/fs/cgroup/cpuacct/");
    if dir_exists(v1_sys_fs_dir) {
        return read_and_parse_v1_sys_stat(
            &v1_sys_fs_dir,
            &Path::new("/sys/fs/cgroup/cpu/cpu.cfs_period_us"),
            &Path::new("/sys/fs/cgroup/cpu/cpu.cfs_quota_us"),
            cpu_count,
        );
    }

    Err(ProbeError::UnexpectedContent(format!(
        "Directory `{}` and file `{}` not found",
        v1_sys_fs_dir.to_str().unwrap_or("unknown path"),
        v2_sys_fs_file.to_str().unwrap_or("unknown path")
    )))
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod test {
    use super::{CgroupCpuMeasurement, CgroupCpuStat};
    use crate::error::ProbeError;

    #[test]
    fn test_read() {
        assert!(super::read(None).is_ok());
        assert!(super::read(Some(0.5)).is_ok());
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
}

use std::collections::HashMap;
use std::path::Path;
use super::{Result,time_adjusted,calculate_time_difference};
use error::ProbeError;

pub type DiskStats = HashMap<String, DiskStat>;

#[derive(Debug,PartialEq)]
pub struct DiskStatsMeasurement {
    pub precise_time_ns: u64,
    pub stats: DiskStats
}

impl DiskStatsMeasurement {
    /// Calculate the disk stats per minute based on this measurement and a measurement in the
    /// future. It is advisable to make the next measurement roughly a minute from this one for the
    /// most reliable result.
    pub fn calculate_per_minute(&self, next_measurement: &DiskStatsMeasurement) -> Result<DiskStatsPerMinute> {
        let time_difference = calculate_time_difference(self.precise_time_ns, next_measurement.precise_time_ns)?;

        let mut stats = HashMap::new();

        for (name, stat) in self.stats.iter() {
            let next_stat = match next_measurement.stats.get(name) {
                Some(stat) => stat,
                None => return Err(ProbeError::UnexpectedContent(format!("{} is not present in the next measurement", name)))
            };
            let result = match stat {
                DiskStat::Proc(s) => {
                    let next_stat = match next_stat {
                        DiskStat::Proc(s) => s,
                        DiskStat::Sys(_s) => panic!("wrong type expected here TODO")
                    };
                    DiskStat::Proc(ProcDiskStat {
                        reads_completed_successfully: time_adjusted("reads_completed_successfully", next_stat.reads_completed_successfully, s.reads_completed_successfully, time_difference)?,
                        reads_merged: time_adjusted("reads_merged", next_stat.reads_merged, s.reads_merged, time_difference)?,
                        sectors_read: time_adjusted("sectors_read", next_stat.sectors_read, s.sectors_read, time_difference)?,
                        time_spent_reading_ms: time_adjusted("time_spent_reading_ms", next_stat.time_spent_reading_ms, s.time_spent_reading_ms, time_difference)?,
                        writes_completed: time_adjusted("writes_completed", next_stat.writes_completed, s.writes_completed, time_difference)?,
                        writes_merged: time_adjusted("writes_merged", next_stat.writes_merged, s.writes_merged, time_difference)?,
                        sectors_written: time_adjusted("sectors_written", next_stat.sectors_written, s.sectors_written, time_difference)?,
                        time_spent_writing_ms: time_adjusted("time_spent_writing_ms", next_stat.time_spent_writing_ms, s.time_spent_writing_ms, time_difference)?,
                        ios_currently_in_progress: time_adjusted("ios_currently_in_progress", next_stat.ios_currently_in_progress, s.ios_currently_in_progress, time_difference)?,
                        time_spent_doing_ios_ms: time_adjusted("time_spent_doing_ios_ms", next_stat.time_spent_doing_ios_ms, s.time_spent_doing_ios_ms, time_difference)?,
                        weighted_time_spent_doing_ios_ms: time_adjusted("weighted_time_spent_doing_ios_ms", next_stat.weighted_time_spent_doing_ios_ms, s.weighted_time_spent_doing_ios_ms, time_difference)?
                    })
                },
                DiskStat::Sys(s) => {
                    let next_stat = match next_stat {
                        DiskStat::Proc(_s) => panic!("wrong type expected here TODO"),
                        DiskStat::Sys(s) => s,
                    };
                    DiskStat::Sys(SysDiskStat {
                        read: time_adjusted("read", next_stat.read, s.read, time_difference)?,
                        written: time_adjusted("written", next_stat.written, s.written, time_difference)?,
                    })
                }
            };
            stats.insert(name.to_owned(), result);
        }

        Ok(DiskStatsPerMinute {
            stats: stats
        })
    }
}

#[derive(Debug,PartialEq)]
pub struct ProcDiskStat {
    pub reads_completed_successfully: u64,
    pub reads_merged: u64,
    pub sectors_read: u64,
    pub time_spent_reading_ms: u64,
    pub writes_completed: u64,
    pub writes_merged: u64,
    pub sectors_written: u64,
    pub time_spent_writing_ms: u64,
    pub ios_currently_in_progress: u64,
    pub time_spent_doing_ios_ms: u64,
    pub weighted_time_spent_doing_ios_ms: u64
}
#[derive(Debug,PartialEq)]
pub struct SysDiskStat {
    pub read: u64,
    pub written: u64
}
#[derive(Debug,PartialEq)]
pub enum DiskStat {
  Proc(ProcDiskStat),
  Sys(SysDiskStat)
}

impl DiskStat {
  pub fn bytes_read(&self) -> u64 {
    match self {
      DiskStat::Proc(s) => s.sectors_read * 512,
      DiskStat::Sys(s) => s.read,
    }
  }

  pub fn bytes_written(&self) -> u64 {
    match self {
      DiskStat::Proc(s) => s.sectors_written * 512,
      DiskStat::Sys(s) => s.written,
    }
  }
}

#[derive(Debug,PartialEq)]
pub struct DiskStatsPerMinute {
   pub stats: DiskStats
}

/// Read the current Disk IO stats of the system.
#[cfg(target_os = "linux")]
pub fn read() -> Result<DiskStatsMeasurement> {
    os::read_and_parse_proc_diskstats(&Path::new("/proc/diskstats"))
}

/// Read the current Disk IO stats of the container.
#[cfg(target_os = "linux")]
pub fn read_from_container() -> Result<DiskStatsMeasurement> {
    os::read_from_container()
}

#[cfg(target_os = "linux")]
mod os {
    use std::io::BufRead;
    use std::path::Path;
    use std::collections::HashMap;
    use time;
    use super::{DiskStatsMeasurement,DiskStat,ProcDiskStat,SysDiskStat};
    use super::super::{file_to_buf_reader,parse_u64,Result,ProbeError,path_to_string,dir_exists};

    const DISK_STATS_SYS_NUMBER_OF_FIELDS: usize = 2;

    #[inline]
    pub fn read_and_parse_proc_diskstats(path: &Path) -> Result<DiskStatsMeasurement> {
        let reader = file_to_buf_reader(path)?;

        let mut out = DiskStatsMeasurement {
            precise_time_ns: time::precise_time_ns(),
            stats: HashMap::new()
        };

        for line_result in reader.lines() {
            let line = line_result.map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
            let segments: Vec<&str> = line.split_whitespace().collect();

            if segments.len() != 14 {
                return Err(ProbeError::UnexpectedContent("Incorrect number of segments".to_owned()))
            }

            let disk_stat = ProcDiskStat {
                reads_completed_successfully: parse_u64(segments[3])?,
                reads_merged: parse_u64(segments[4])?,
                sectors_read: parse_u64(segments[5])?,
                time_spent_reading_ms: parse_u64(segments[6])?,
                writes_completed: parse_u64(segments[7])?,
                writes_merged: parse_u64(segments[8])?,
                sectors_written: parse_u64(segments[9])?,
                time_spent_writing_ms: parse_u64(segments[10])?,
                ios_currently_in_progress: parse_u64(segments[11])?,
                time_spent_doing_ios_ms: parse_u64(segments[12])?,
                weighted_time_spent_doing_ios_ms: parse_u64(segments[13])?
            };
            out.stats.insert(segments[2].to_owned(), DiskStat::Proc(disk_stat));
        }

        Ok(out)
    }

    pub fn read_from_container() -> Result<DiskStatsMeasurement> {
        let sys_fs_dir = Path::new("/sys/fs/cgroup/blkio/");
        if dir_exists(sys_fs_dir) {
            read_and_parse_sys_stat(&sys_fs_dir)
        } else {
            let message = format!("Directory `{}` not found", sys_fs_dir.to_str().unwrap_or("unknown path"));
            Err(ProbeError::UnexpectedContent(message))
        }
    }

    pub fn read_and_parse_sys_stat(path: &Path) -> Result<DiskStatsMeasurement> {
        let reader = file_to_buf_reader(&path.join("blkio.throttle.io_service_bytes"))?;

        let mut disk_stat = SysDiskStat {
            read: 0,
            written: 0
        };

        let mut fields_encountered = 0;
        for line_result in reader.lines() {
            let line = line_result.map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
            let segments: Vec<&str> = line.split_whitespace().collect();
            let value = parse_u64(&segments[2])?;
            fields_encountered += match segments[1] {
                "Read" => {
                    disk_stat.read = value;
                    1
                },
                "Write" => {
                    disk_stat.written = value;
                    1
                },
                _ => 0
            };

            if fields_encountered == DISK_STATS_SYS_NUMBER_OF_FIELDS {
                break
            }
        }
        let mut stats = HashMap::new();
        stats.insert("container".to_owned(), DiskStat::Sys(disk_stat));
        Ok(DiskStatsMeasurement {
            precise_time_ns: time::precise_time_ns(),
            stats: stats
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;
    use error::ProbeError;
    use super::DiskStatsMeasurement;
    use super::os::read_and_parse_proc_diskstats;

    #[test]
    fn test_read_disk_stats() {
        assert!(super::read().is_ok());
    }

    #[test]
    fn test_read_and_parse_proc_diskstats() {
        let measurement = read_and_parse_proc_diskstats(&Path::new("fixtures/linux/disk_stats/proc_diskstats")).unwrap();

        assert!(measurement.precise_time_ns > 0);

        assert_eq!(2, measurement.stats.len());

        let sda_enum = measurement.stats.get("sda").unwrap();
        let sda = helpers::get_proc_disk_stat(&sda_enum);
        assert_eq!(6185, sda.reads_completed_successfully);
        assert_eq!(9367, sda.reads_merged);
        assert_eq!(403272, sda.sectors_read);
        assert_eq!(206475264, sda_enum.bytes_read());
        assert_eq!(22160, sda.time_spent_reading_ms);
        assert_eq!(2591, sda.writes_completed);
        assert_eq!(8251, sda.writes_merged);
        assert_eq!(84452, sda.sectors_written);
        assert_eq!(43239424, sda_enum.bytes_written());
        assert_eq!(2860, sda.time_spent_writing_ms);
        assert_eq!(0, sda.ios_currently_in_progress);
        assert_eq!(8960, sda.time_spent_doing_ios_ms);
        assert_eq!(24990, sda.weighted_time_spent_doing_ios_ms);

        let sda1_enum = measurement.stats.get("sda1").unwrap();
        let sda1 = helpers::get_proc_disk_stat(&sda1_enum);
        assert_eq!(483, sda1.reads_completed_successfully);
        assert_eq!(4782, sda1.reads_merged);
        assert_eq!(41466, sda1.sectors_read);
        assert_eq!(21230592, sda1_enum.bytes_read());
        assert_eq!(1100, sda1.time_spent_reading_ms);
        assert_eq!(7, sda1.writes_completed);
        assert_eq!(1, sda1.writes_merged);
        assert_eq!(28, sda1.sectors_written);
        assert_eq!(14336, sda1_enum.bytes_written());
        assert_eq!(40, sda1.time_spent_writing_ms);
        assert_eq!(0, sda1.ios_currently_in_progress);
        assert_eq!(930, sda1.time_spent_doing_ios_ms);
        assert_eq!(1140, sda1.weighted_time_spent_doing_ios_ms);
    }

    #[test]
    fn test_read_and_parse_proc_diskstats_incomplete() {
        match read_and_parse_proc_diskstats(&Path::new("fixtures/linux/disk_stats/proc_diskstats_incomplete")) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_proc_diskstats_garbage() {
        match read_and_parse_proc_diskstats(&Path::new("fixtures/linux/disk_stats/proc_diskstats_garbage")) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_calculate_per_minute_full_minute() {
        let mut stats1 = HashMap::new();
        stats1.insert("sda1".to_owned(), helpers::proc_disk_stat(0));
        let measurement1 = DiskStatsMeasurement {
            precise_time_ns: 60_000_000_000,
            stats: stats1
        };
        let mut stats2 = HashMap::new();
        stats2.insert("sda1".to_owned(), helpers::proc_disk_stat(120));
        let measurement2 = DiskStatsMeasurement {
            precise_time_ns: 120_000_000_000,
            stats: stats2
        };

        let per_minute = measurement1.calculate_per_minute(&measurement2).unwrap();
        let sda1 = helpers::get_proc_disk_stat(&per_minute.stats.get("sda1").unwrap());
        assert_eq!(sda1.reads_completed_successfully, 120);
        assert_eq!(sda1.reads_merged, 120);
        assert_eq!(sda1.sectors_read, 120);
        assert_eq!(sda1.time_spent_reading_ms, 120);
        assert_eq!(sda1.writes_completed, 120);
        assert_eq!(sda1.writes_merged, 120);
        assert_eq!(sda1.sectors_written, 120);
        assert_eq!(sda1.time_spent_writing_ms, 120);
        assert_eq!(sda1.ios_currently_in_progress, 120);
        assert_eq!(sda1.time_spent_doing_ios_ms, 120);
        assert_eq!(sda1.weighted_time_spent_doing_ios_ms, 120);
    }

    #[test]
    fn test_calculate_per_minute_partial_minute() {
        let mut stats1 = HashMap::new();
        stats1.insert("sda1".to_owned(), helpers::proc_disk_stat(0));
        let measurement1 = DiskStatsMeasurement {
            precise_time_ns: 60_000_000_000,
            stats: stats1
        };
        let mut stats2 = HashMap::new();
        stats2.insert("sda1".to_owned(), helpers::proc_disk_stat(120));
        let measurement2 = DiskStatsMeasurement {
            precise_time_ns: 90_000_000_000,
            stats: stats2
        };

        let per_minute = measurement1.calculate_per_minute(&measurement2).unwrap();
        let sda1 = helpers::get_proc_disk_stat(&per_minute.stats.get("sda1").unwrap());
        assert_eq!(sda1.reads_completed_successfully, 60);
        assert_eq!(sda1.reads_merged, 60);
        assert_eq!(sda1.sectors_read, 60);
        assert_eq!(sda1.time_spent_reading_ms, 60);
        assert_eq!(sda1.writes_completed, 60);
        assert_eq!(sda1.writes_merged, 60);
        assert_eq!(sda1.sectors_written, 60);
        assert_eq!(sda1.time_spent_writing_ms, 60);
        assert_eq!(sda1.ios_currently_in_progress, 60);
        assert_eq!(sda1.time_spent_doing_ios_ms, 60);
        assert_eq!(sda1.weighted_time_spent_doing_ios_ms, 60);
    }

    #[test]
    fn test_calculate_per_minute_wrong_times() {
        let measurement1 = DiskStatsMeasurement {
            precise_time_ns: 500,
            stats: HashMap::new()
        };
        let measurement2 = DiskStatsMeasurement {
            precise_time_ns: 300,
            stats: HashMap::new()
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::InvalidInput(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_calculate_per_minute_values_lower() {
        let mut stats1 = HashMap::new();
        stats1.insert("sda1".to_owned(), helpers::proc_disk_stat(500));
        let measurement1 = DiskStatsMeasurement {
            precise_time_ns: 500,
            stats: stats1
        };
        let mut stats2 = HashMap::new();
        stats2.insert("sda1".to_owned(), helpers::proc_disk_stat(400));
        let measurement2 = DiskStatsMeasurement {
            precise_time_ns: 600,
            stats: stats2
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_calculate_per_minute_different_disks() {
        let mut stats1 = HashMap::new();
        stats1.insert("sda1".to_owned(), helpers::proc_disk_stat(500));
        let measurement1 = DiskStatsMeasurement {
            precise_time_ns: 500,
            stats: stats1
        };
        let mut stats2 = HashMap::new();
        stats2.insert("sda2".to_owned(), helpers::proc_disk_stat(600));
        let measurement2 = DiskStatsMeasurement {
            precise_time_ns: 600,
            stats: stats2
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    mod helpers {
        use super::super::{DiskStat,ProcDiskStat};

        pub fn get_proc_disk_stat(value: &DiskStat) -> &ProcDiskStat {
            match value {
                DiskStat::Proc(s) => s,
                s => panic!("Not a ProcDiskStat: {:?}", value)
            }
        }

        pub fn proc_disk_stat(value: u64) -> DiskStat {
            DiskStat::Proc(ProcDiskStat {
                reads_completed_successfully: value,
                reads_merged: value,
                sectors_read: value,
                time_spent_reading_ms: value,
                writes_completed: value,
                writes_merged: value,
                sectors_written: value,
                time_spent_writing_ms: value,
                ios_currently_in_progress: value,
                time_spent_doing_ios_ms: value,
                weighted_time_spent_doing_ios_ms: value
            })
        }
    }
}

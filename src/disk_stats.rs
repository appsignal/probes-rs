use std::path::Path;
use super::Result;

pub type DiskStatMeasurements = Vec<DiskStatMeasurement>;

#[derive(Debug,PartialEq)]
pub struct DiskStatMeasurement {
    pub precise_time_ns: u64,
    pub device_name: String,
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

#[cfg(target_os = "linux")]
pub fn read() -> Result<DiskStatMeasurements> {
    os::read_and_parse_proc_diskstats(&Path::new("/proc/diskstats"))
}

#[cfg(target_os = "linux")]
mod os {
    use time;
    use std::io::BufRead;
    use std::path::Path;
    use super::{DiskStatMeasurement,DiskStatMeasurements};
    use super::super::{file_to_buf_reader,parse_u64,Result,ProbeError};

    #[inline]
    pub fn read_and_parse_proc_diskstats(path: &Path) -> Result<DiskStatMeasurements> {
        let reader = try!(file_to_buf_reader(path));
        let precise_time_ns = time::precise_time_ns();

        let mut out = DiskStatMeasurements::new();

        for line_result in reader.lines() {
            let line = try!(line_result);
            let segments: Vec<&str> = line.split_whitespace().collect();

            if segments.len() != 14 {
                return Err(ProbeError::UnexpectedContent("Incorrect number of segments".to_owned()))
            }

            let disk_stat = DiskStatMeasurement {
                precise_time_ns: precise_time_ns,
                device_name: segments[2].to_owned(),
                reads_completed_successfully: try!(parse_u64(segments[3])),
                reads_merged: try!(parse_u64(segments[4])),
                sectors_read: try!(parse_u64(segments[5])),
                time_spent_reading_ms: try!(parse_u64(segments[6])),
                writes_completed: try!(parse_u64(segments[7])),
                writes_merged: try!(parse_u64(segments[8])),
                sectors_written: try!(parse_u64(segments[9])),
                time_spent_writing_ms: try!(parse_u64(segments[10])),
                ios_currently_in_progress: try!(parse_u64(segments[11])),
                time_spent_doing_ios_ms: try!(parse_u64(segments[12])),
                weighted_time_spent_doing_ios_ms: try!(parse_u64(segments[13]))
            };
            out.push(disk_stat);
        }

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::os::read_and_parse_proc_diskstats;
    use error::ProbeError;

    #[test]
    fn test_read_disk_stats() {
        assert!(super::read().is_ok());
    }

    #[test]
    fn test_read_and_parse_proc_diskstats() {
        let disks = read_and_parse_proc_diskstats(&Path::new("fixtures/linux/disk_stats/proc_diskstats")).unwrap();

        assert_eq!(2, disks.len());

        let sda = &disks[0];
        assert_eq!("sda".to_owned(), sda.device_name);
        assert_eq!(6185, sda.reads_completed_successfully);
        assert_eq!(9367, sda.reads_merged);
        assert_eq!(403272, sda.sectors_read);
        assert_eq!(22160, sda.time_spent_reading_ms);
        assert_eq!(2591, sda.writes_completed);
        assert_eq!(8251, sda.writes_merged);
        assert_eq!(84452, sda.sectors_written);
        assert_eq!(2860, sda.time_spent_writing_ms);
        assert_eq!(0, sda.ios_currently_in_progress);
        assert_eq!(8960, sda.time_spent_doing_ios_ms);
        assert_eq!(24990, sda.weighted_time_spent_doing_ios_ms);

        let sda1 = &disks[1];
        assert_eq!("sda1".to_owned(), sda1.device_name);
        assert_eq!(483, sda1.reads_completed_successfully);
        assert_eq!(4782, sda1.reads_merged);
        assert_eq!(41466, sda1.sectors_read);
        assert_eq!(1100, sda1.time_spent_reading_ms);
        assert_eq!(7, sda1.writes_completed);
        assert_eq!(1, sda1.writes_merged);
        assert_eq!(28, sda1.sectors_written);
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
}

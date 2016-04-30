use std::path::Path;
use super::Result;

#[derive(Debug)]
pub struct CPUStat {
    pub user: u32,
    pub nice: u32,
    pub system: u32,
    pub idle: u32,
    pub iowait: u32
}

#[cfg(target_os = "linux")]
pub fn read() -> Result<CPUStat> {
    // columns: user nice system idle iowait irq softirq
    os::read_proc_cpu_stat(&Path::new("/proc/stat"))
}

#[cfg(target_os = "linux")]
mod os {
    use std::path::Path;
    use std::io::BufRead;
    use super::super::{Result, file_to_buf_reader};
    use super::CPUStat;
    use error::ProbeError;

    pub fn read_proc_cpu_stat(path: &Path) -> Result<CPUStat> {
        let mut line = String::new();
        let mut reader = try!(file_to_buf_reader(path));
        try!(reader.read_line(&mut line));

        let stats: Vec<&str> = line
            .split_whitespace()
            .skip(1)
            .collect();

        if stats.len() < 5 {
            return Err(ProbeError::UnexpectedContent("Incorrect number of stats".to_owned()));
        }

        Ok(CPUStat {
            user   : try!(parse_stat(stats[0])),
            nice   : try!(parse_stat(stats[1])),
            system : try!(parse_stat(stats[2])),
            idle   : try!(parse_stat(stats[3])),
            iowait : try!(parse_stat(stats[4]))
        })
    }

    fn parse_stat(stat: &str) -> Result<u32> {
        stat.parse().map_err(|_| {
            ProbeError::UnexpectedContent(format!("Could not parse stat {:?}", stat).to_owned())
        })
    }
}

#[cfg(test)]
mod test {
    use super::os::read_proc_cpu_stat;
    use std::path::Path;
    use error::ProbeError;

    #[test]
    fn test_cpu_stat() {
        let stat = read_proc_cpu_stat(&Path::new("fixtures/linux/cpu_stat/proc_cpu_stat")).unwrap();
        assert_eq!(stat.user, 0);
        assert_eq!(stat.nice, 1);
        assert_eq!(stat.system, 2);
        assert_eq!(stat.idle, 3);
        assert_eq!(stat.iowait, 4);
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
}

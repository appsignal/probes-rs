const MISSING_LINE_ERROR: &'static str = "[process_io] Could not find line";
const PIDSTAT_READ_ERROR: &'static str = "[process_io] Could not convert bytes into string from pidstat";

use super::Result;
use libc::pid_t;

#[derive(Debug)]
pub struct ProcessIO {
    pub uid: u32,
    pub pid: pid_t,
    pub read_kbs: f32,
    pub write_kbs: f32,
    pub canceled_kbs: f32,
    pub iodelay: u32,
}

pub fn read(pid: pid_t) -> Result<ProcessIO> {
    os::read_process_io(pid)
}

#[cfg(target_os = "linux")]
mod os {
    use super::{MISSING_LINE_ERROR, PIDSTAT_READ_ERROR, ProcessIO};
    use super::super::Result;
    use error::ProbeError;
    use std::io::BufRead;
    use std::process::Command;
    use libc::pid_t;

    pub fn read_process_io(pid: pid_t) -> Result<ProcessIO> {
        let raw = try!(run_pidstat(pid));
        read_pidstat_io(raw)
    }

    pub fn read_pidstat_io(raw: String) -> Result<ProcessIO> {
        get_io_line(&raw).and_then(parse)
    }

    fn run_pidstat(pid: pid_t) -> Result<String> {
        Command::new("pidstat")
            .arg("-d")
            .arg(format!("-p {}", pid))
            .output()
            .map_err(|_| ProbeError::UnexpectedContent(PIDSTAT_READ_ERROR.to_string()))
            .and_then(|c| Ok(c.stdout))
            .and_then(|bytes| String::from_utf8(bytes).map_err(|_| ProbeError::UnexpectedContent(PIDSTAT_READ_ERROR.to_string())) )
    }


    fn get_io_line<'a>(rawb: &'a str) -> Result<&'a str> {
        rawb.lines().skip(3).next().ok_or(ProbeError::UnexpectedContent(MISSING_LINE_ERROR.to_string()))
    }

    fn parse(stats: &str) -> Result<ProcessIO> {
        let stats: Vec<&str> = stats.split_whitespace().skip(2).collect();

        Ok(ProcessIO {
            uid      : try!(stats[0].parse()),
            pid      : try!(stats[1].parse::<pid_t>()),
            read_kbs   : try!(stats[2].parse()),
            write_kbs   : try!(stats[3].parse()),
            canceled_kbs : try!(stats[4].parse()),
            iodelay  : try!(stats[5].parse()),
        })
    }
}

#[cfg(test)]
mod test {
    use super::os::read_pidstat_io;
    use super::read;
    use super::super::file_to_string;
    use std::path::Path;
    use error::ProbeError;

    #[test]
    fn test_pidstat_ok() {
        let raw = file_to_string(&Path::new("fixtures/linux/process_io/pidstat")).unwrap();
        let stat = read_pidstat_io(raw).unwrap();
        assert_eq!(stat.uid, 1000);
        assert_eq!(stat.pid, 26792);
        assert_eq!(stat.read_kbs, 0.92);
        assert_eq!(stat.write_kbs, 1.44);
        assert_eq!(stat.canceled_kbs, 0.00);
        assert_eq!(stat.iodelay, 81);
    }

    #[test]
    fn test_pidstat_missing() {
        let raw = file_to_string(&Path::new("fixtures/linux/process_io/pidstat_missing")).unwrap();
        match read_pidstat_io(raw) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            other @ _ => panic!("Expected missing line error, got {:?}", other)
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_integration() {
        let stat = read(1);
        assert!(stat.is_ok());
        assert_eq!(stat.unwrap().pid, 1);
    }
}

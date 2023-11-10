// These methods are described in:
// http://nadeausoftware.com/articles/2012/07/c_c_tip_how_get_process_resident_set_size_physical_memory_use

use super::Result;

/// Get the current RSS memory of this process in KB
#[cfg(target_os = "linux")]
pub fn current_rss() -> Result<u64> {
    os::current_rss()
}

/// Get the current RSS memory of a process with given pid in KB
#[cfg(target_os = "linux")]
pub fn current_rss_of(pid: libc::pid_t) -> Result<u64> {
    os::current_rss_of(pid)
}

/// Get the max RSS memory of this process in KB
#[cfg(target_os = "linux")]
pub fn max_rss() -> u64 {
    os::max_rss()
}

#[cfg(target_os = "linux")]
mod os {
    use super::super::file_to_string;
    use super::super::ProbeError;
    use super::super::Result;
    use std::mem;
    use std::path::Path;

    #[inline]
    pub fn current_rss() -> Result<u64> {
        read_and_get_current_rss(&Path::new("/proc/self/statm"))
    }

    #[inline]
    pub fn current_rss_of(pid: libc::pid_t) -> Result<u64> {
        read_and_get_current_rss(&Path::new(&format!("/proc/{}/statm", pid)))
    }

    #[inline]
    pub fn read_and_get_current_rss(path: &Path) -> Result<u64> {
        let raw_data = file_to_string(path)?;
        let segments: Vec<&str> = raw_data.split_whitespace().collect();

        if segments.len() < 2 {
            return Err(ProbeError::UnexpectedContent(
                "Incorrect number of segments".to_owned(),
            ));
        }

        let pages: u64 = segments[1]
            .parse()
            .map_err(|_| ProbeError::UnexpectedContent("Could not parse segment".to_owned()))?;

        // Value is in pages, needs to be multiplied by the page size to get a value in KB. We ask
        // the OS for this information using sysconf.
        let pagesize = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as u64 / 1024;

        Ok(pages * pagesize)
    }

    #[inline]
    pub fn max_rss() -> u64 {
        let mut rusage = mem::MaybeUninit::<libc::rusage>::uninit();
        unsafe {
            libc::getrusage(libc::RUSAGE_SELF, rusage.as_mut_ptr());
            rusage.assume_init()
        }
        .ru_maxrss as u64
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::super::ProbeError;
    use std::path::Path;

    #[test]
    fn test_current_rss() {
        assert!(super::current_rss().is_ok());
        // See if it's a sort of sane value, between 1 and 250 mb
        assert!(super::current_rss().unwrap() > 1_000);
        assert!(super::current_rss().unwrap() < 250_000);
    }

    #[test]
    fn test_read_and_get_current_rss() {
        let path = Path::new("fixtures/linux/process_memory/proc_self_statm");
        let value = super::os::read_and_get_current_rss(&path).unwrap();
        assert_eq!(4552, value);
    }

    #[test]
    fn test_read_and_get_current_rss_wrong_path() {
        let path = Path::new("/nonsense");
        match super::os::read_and_get_current_rss(&path) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_get_current_rss_incomplete() {
        let path = Path::new("fixtures/linux/process_memory/proc_self_statm_incomplete");
        match super::os::read_and_get_current_rss(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_get_current_rss_garbage() {
        let path = Path::new("fixtures/linux/process_memory/proc_self_statm_garbage");
        match super::os::read_and_get_current_rss(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_current_rss_of() {
        let pid = unsafe { libc::getpid() };
        assert!(super::current_rss_of(pid).is_ok());
        // See if it's a sort of sane value, between 1 and 250 mb
        assert!(super::current_rss_of(pid).unwrap() > 1_000);
        assert!(super::current_rss_of(pid).unwrap() < 250_000);
    }

    #[test]
    fn test_current_rss_of_invalid_pid() {
        assert!(super::current_rss_of(0).is_err());
    }

    #[test]
    fn test_max_rss() {
        // See if it's a sort of sane value, between 1 and 250 mb
        print!("!!! RSS: {}", super::max_rss());
        assert_eq!(super::max_rss(), 0);
        assert!(super::max_rss() > 1_000);
        assert!(super::max_rss() < 250_000);
    }
}

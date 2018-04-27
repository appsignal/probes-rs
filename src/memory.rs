use super::Result;

#[derive(Debug,PartialEq)]
pub struct Memory {
    total: u64,
    free: u64,
    used: u64,
    buffers: u64,
    cached: u64,
    swap_total: u64,
    swap_free: u64,
    swap_used: u64,
}

const MEMORY_NUMBER_OF_FIELDS: usize = 6;

impl Memory {
    /// Total amount of physical memory in Kb.
    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn free(&self) -> u64 {
        self.free
    }

    /// Total amount of used physical memory in Kb.
    pub fn used(&self) -> u64 {
        self.used
    }

    /// Total amount of swap space in Kb.
    pub fn swap_total(&self) -> u64 {
        self.swap_total
    }

    /// Total amount of free swap space in Kb.
    pub fn swap_free(&self) -> u64 {
        self.swap_free
    }

    /// Total amount of used swap space in Kb.
    pub fn swap_used(&self) -> u64 {
        self.swap_used
    }
}

/// Read the current memory status of the system.
#[cfg(target_os = "linux")]
pub fn read() -> Result<Memory> {
    os::read()
}

#[cfg(target_os = "linux")]
mod os {
    use std::io::BufRead;
    use std::path::Path;

    use super::{Memory,MEMORY_NUMBER_OF_FIELDS};
    use super::super::{ProbeError,Result,container};
    use super::super::{file_to_buf_reader,parse_u64,read_file_value_as_u64};

    #[inline]
    pub fn read() -> Result<Memory> {
        if container::in_container() {
            read_and_parse_sys_memory(&Path::new("/sys/fs/cgroup/memory/"))
        } else {
            read_and_parse_proc_memory(&Path::new("/proc/memory.stat"))
        }
    }

    #[inline]
    pub fn read_and_parse_proc_memory(path: &Path) -> Result<Memory> {
        let mut memory = Memory {
            total: 0,
            free: 0,
            used: 0,
            buffers: 0,
            cached: 0,
            swap_total: 0,
            swap_free: 0,
            swap_used: 0,
        };
        let mut free = 0;

        let reader = try!(file_to_buf_reader(path));

        let mut fields_encountered = 0;
        for line in reader.lines() {
            let line = try!(line);
            let segments: Vec<&str> = line.split_whitespace().collect();
            let value: u64 = try!(parse_u64(segments[1]));

            // If this is a field we recognize set it's value and increment the
            // number of fields we encountered.
            fields_encountered += match segments[0] {
                "MemTotal:" => {
                    memory.total = value;
                    1
                },
                "MemFree:" => {
                    free = value;
                    1
                },
                "Buffers:" => {
                    memory.buffers = value;
                    1
                },
                "Cached:" => {
                    memory.cached = value;
                    1
                },
                "SwapTotal:" => {
                    memory.swap_total = value;
                    1
                },
                "SwapFree:" => {
                    memory.swap_free = value;
                    1
                },
                _ => 0
            };

            if fields_encountered == MEMORY_NUMBER_OF_FIELDS {
                break
            }
        }

        if fields_encountered != MEMORY_NUMBER_OF_FIELDS {
            return Err(ProbeError::UnexpectedContent("Did not encounter all expected fields".to_owned()))
        }

        // Total amount of free physical memory in Kb.
        // Inclused buffers and caches, these will be freed
        // up by the OS when the memory is needed.
        memory.free = free + memory.buffers + memory.cached;
        memory.used = memory.total - memory.free;
        memory.swap_used = memory.swap_total - memory.swap_free;

        Ok(memory)
    }

    pub fn read_and_parse_sys_memory(path: &Path) -> Result<Memory> {
        let mut memory = Memory {
            total: 0,
            free: 0,
            used: 0,
            buffers: 0,
            cached: 0,
            swap_total: 0,
            swap_free: 0,
            swap_used: 0,
        };

        let reader = try!(file_to_buf_reader(&path.join("memory.stat")));
        let used_memory = bytes_to_kilo_bytes(try!(read_file_value_as_u64(&path.join("memory.usage_in_bytes"))));
        // TODO: or this one for swap total? /sys/fs/cgroup/memory/memory.stat: hierarchical_memsw_limit
        // If swap is not configured for the container, read 0 as value
        match read_file_value_as_u64(&path.join("memory.memsw.limit_in_bytes")) {
            Ok(value) => memory.swap_total = bytes_to_kilo_bytes(value),
            Err(_) => ()
        }
        let mut swap_used: Option<u64> = None;

        for line in reader.lines() {
            let line = try!(line);
            let segments: Vec<&str> = line.split_whitespace().collect();
            let value = try!(parse_u64(&segments[1]));

            match segments[0] {
                // TODO: or this one for memory total? /sys/fs/cgroup/memory/memory.limit_in_bytes
                "hierarchical_memory_limit" => memory.total = bytes_to_kilo_bytes(value),
                "swap" => swap_used = Some(bytes_to_kilo_bytes(value)),
                "cache" => memory.cached = bytes_to_kilo_bytes(value),
                _ => ()
            };
        }

        memory.used = used_memory - memory.cached;
        memory.free = memory.total - memory.used;
        match swap_used {
            Some(value) => {
                memory.swap_used = value;
                memory.swap_free = memory.swap_total - value
            },
            None => ()
        }

        Ok(memory)
    }

    fn bytes_to_kilo_bytes(bytes: u64) -> u64 {
        bytes.checked_div(1024).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use super::super::ProbeError;
    use super::Memory;

    #[test]
    fn test_read_memory() {
        assert!(super::read().is_ok());
    }

    #[test]
    fn test_read_and_parse_proc_memory() {
        let path = Path::new("fixtures/linux/memory/proc_meminfo");
        let memory = super::os::read_and_parse_proc_memory(&path).unwrap();

        let expected = Memory {
            total: 376072,
            free: 324248,
            used: 51824,
            buffers: 22820,
            cached: 176324,
            swap_total: 1101816,
            swap_free: 1100644,
            swap_used: 1172,
        };
        assert_eq!(expected, memory);
        assert_eq!(memory.total, memory.used + memory.free);
        assert_eq!(memory.swap_total, memory.swap_used + memory.swap_free);
    }

    #[test]
    fn test_read_and_parse_memory_wrong_path() {
        let path = Path::new("/nonsense");
        match super::os::read_and_parse_proc_memory(&path) {
            Err(ProbeError::IO(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_memory_incomplete() {
        let path = Path::new("fixtures/linux/memory/proc_meminfo_incomplete");
        match super::os::read_and_parse_proc_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_memory_garbage() {
        let path = Path::new("fixtures/linux/memory/proc_meminfo_garbage");
        match super::os::read_and_parse_proc_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_sys_memory() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/memory/");
        let memory = super::os::read_and_parse_sys_memory(&path).unwrap();

        let expected = Memory {
            total: 512000, // 500mb
            free: 503400, // total - used
            used: 8600,
            buffers: 0,
            cached: 58928,
            swap_total: 2000,
            swap_free: 1500,
            swap_used: 500,
        };
        assert_eq!(expected, memory);
        assert_eq!(memory.total, memory.used + memory.free);
        assert_eq!(memory.swap_total, memory.swap_used + memory.swap_free);
    }

    #[test]
    fn test_read_and_parse_sys_memory_wrong_path() {
        let path = Path::new("/nonsense");
        match super::os::read_and_parse_sys_memory(&path) {
            Err(ProbeError::IO(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_sys_memory_incomplete() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/memory_incomplete/");
        match super::os::read_and_parse_sys_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_sys_memory_garbage() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/memory_garbage/");
        match super::os::read_and_parse_sys_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_sys_memory_no_swap() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/memory_without_swap/");
        let memory = super::os::read_and_parse_sys_memory(&path).unwrap();

        let expected = Memory {
            total: 512000, // 500mb
            free: 503400, // total - used
            used: 8600,
            buffers: 0,
            cached: 58928,
            swap_total: 0, // Reads 0 swap
            swap_free: 0,  // Reads 0 swap
            swap_used: 0,
        };
        assert_eq!(expected, memory);
        assert_eq!(memory.total, memory.used + memory.free);
        assert_eq!(memory.swap_total, memory.swap_used + memory.swap_free);
    }
}

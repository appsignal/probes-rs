use super::Result;

#[derive(Debug, PartialEq)]
pub struct Memory {
    total: u64,
    free: u64,
    used: u64,
    buffers: u64,
    cached: u64,
    shmem: u64,
    swap_total: u64,
    swap_free: u64,
    swap_used: u64,
}

const PROC_MEMORY_NUMBER_OF_FIELDS: usize = 7;

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

    /// Total amount of shared memory
    pub fn shmem(&self) -> u64 {
        self.shmem
    }
}

/// Read the current memory status of the system.
#[cfg(target_os = "linux")]
pub fn read() -> Result<Memory> {
    os::read()
}

/// Read the current memory status of the container.
#[cfg(target_os = "linux")]
pub fn read_from_container() -> Result<Memory> {
    os::read_from_container()
}

#[cfg(target_os = "linux")]
mod os {
    use std::io::BufRead;
    use std::path::Path;

    use super::super::{dir_exists, file_to_buf_reader, parse_u64};
    use super::super::{path_to_string, ProbeError, Result};
    use super::{Memory, PROC_MEMORY_NUMBER_OF_FIELDS};

    #[inline]
    pub fn read() -> Result<Memory> {
        read_and_parse_proc_memory(&Path::new("/proc/meminfo"))
    }

    pub fn read_from_container() -> Result<Memory> {
        let sys_fs_dir = Path::new("/sys/fs/cgroup/memory/");
        if dir_exists(sys_fs_dir) {
            read_and_parse_sys_memory(&sys_fs_dir)
        } else {
            let message = format!(
                "Directory `{}` not found",
                sys_fs_dir.to_str().unwrap_or("unknown path")
            );
            Err(ProbeError::UnexpectedContent(message))
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
            shmem: 0,
            swap_total: 0,
            swap_free: 0,
            swap_used: 0,
        };
        let mut free = 0;

        let reader = file_to_buf_reader(path)?;

        let mut fields_encountered = 0;
        for line_result in reader.lines() {
            let line = line_result.map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
            let segments: Vec<&str> = line.split_whitespace().collect();
            let value: u64 = parse_u64(segments[1])?;

            // If this is a field we recognize set it's value and increment the
            // number of fields we encountered.
            fields_encountered += match segments[0] {
                "MemTotal:" => {
                    memory.total = value;
                    1
                }
                "MemFree:" => {
                    free = value;
                    1
                }
                "Buffers:" => {
                    memory.buffers = value;
                    1
                }
                "Cached:" => {
                    memory.cached = value;
                    1
                }
                "SwapTotal:" => {
                    memory.swap_total = value;
                    1
                }
                "SwapFree:" => {
                    memory.swap_free = value;
                    1
                }
                "Shmem:" => {
                    memory.shmem = value;
                    1
                }
                _ => 0,
            };

            if fields_encountered == PROC_MEMORY_NUMBER_OF_FIELDS {
                break;
            }
        }

        if fields_encountered != PROC_MEMORY_NUMBER_OF_FIELDS {
            return Err(ProbeError::UnexpectedContent(
                "Did not encounter all expected fields".to_owned(),
            ));
        }

        // Total amount of free physical memory in Kb.
        // Includes buffers and caches, these will be freed
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
            shmem: 0,
            swap_total: 0,
            swap_free: 0,
            swap_used: 0,
        };

        memory.total =
            bytes_to_kilo_bytes(read_file_value_as_u64(&path.join("memory.limit_in_bytes"))?);
        let used_memory =
            bytes_to_kilo_bytes(read_file_value_as_u64(&path.join("memory.usage_in_bytes"))?);

        let reader = file_to_buf_reader(&path.join("memory.stat"))?;
        for line_result in reader.lines() {
            let line = line_result.map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
            let segments: Vec<&str> = line.split_whitespace().collect();
            let value = parse_u64(&segments[1])?;

            match segments[0] {
                "shmem" => {
                    memory.shmem = bytes_to_kilo_bytes(value);
                }
                "cache" => {
                    memory.cached = bytes_to_kilo_bytes(value);
                }
                _ => (),
            };
        }
        memory.used = used_memory - memory.cached;
        memory.free = memory.total - memory.used;

        // If swap is not configured for the container, read 0 as value
        memory.swap_total = match read_file_value_as_u64(&path.join("memory.memsw.limit_in_bytes"))
        {
            Ok(value) => bytes_to_kilo_bytes(value).saturating_sub(memory.total),
            Err(_) => 0,
        };
        // If swap is not configured for the container, read 0 as value
        memory.swap_used = match read_file_value_as_u64(&path.join("memory.memsw.usage_in_bytes")) {
            Ok(value) => bytes_to_kilo_bytes(value).saturating_sub(used_memory),
            Err(_) => 0,
        };
        memory.swap_free = memory.swap_total.saturating_sub(memory.swap_used);

        Ok(memory)
    }

    fn read_file_value_as_u64(path: &Path) -> Result<u64> {
        let mut reader = file_to_buf_reader(path)?;
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
        parse_u64(&line.trim())
    }

    fn bytes_to_kilo_bytes(bytes: u64) -> u64 {
        bytes.checked_div(1024).unwrap_or(0)
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::super::ProbeError;
    use super::Memory;
    use std::path::Path;

    #[test]
    fn test_read_memory() {
        assert!(super::read().is_ok());
    }

    #[test]
    fn test_read_from_container() {
        let result = super::read_from_container();
        assert!(result.is_ok(), "{:?}", result);
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
            shmem: 548,
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
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_memory_incomplete() {
        let path = Path::new("fixtures/linux/memory/proc_meminfo_incomplete");
        match super::os::read_and_parse_proc_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_memory_garbage() {
        let path = Path::new("fixtures/linux/memory/proc_meminfo_garbage");
        match super::os::read_and_parse_proc_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_sys_memory() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/memory/");
        let memory = super::os::read_and_parse_sys_memory(&path).unwrap();

        let expected = Memory {
            total: 512000, // 500mb
            free: 503400,  // total - used
            used: 8600,
            buffers: 0,
            cached: 58928,
            shmem: 0,
            swap_total: 1_488_000, // reported swap total - reported memory total
            swap_free: 1_055_528,
            swap_used: 432_472, // reported swap used - (reported memory used, including cache)
        };
        assert_eq!(expected, memory);
        assert_eq!(memory.total, memory.used + memory.free);
        assert_eq!(memory.swap_total, memory.swap_used + memory.swap_free);
    }

    #[test]
    fn test_read_and_parse_sys_memory_wrong_path() {
        let path = Path::new("/nonsense");
        match super::os::read_and_parse_sys_memory(&path) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_sys_memory_incomplete() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/memory_incomplete/");
        match super::os::read_and_parse_sys_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_sys_memory_missing_files() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/memory_missing_files/");
        match super::os::read_and_parse_sys_memory(&path) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_sys_memory_garbage() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/memory_garbage/");
        match super::os::read_and_parse_sys_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_sys_memory_no_swap() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup/memory_without_swap/");
        let memory = super::os::read_and_parse_sys_memory(&path).unwrap();

        let expected = Memory {
            total: 512000, // 500mb
            free: 503400,  // total - used
            used: 8600,
            buffers: 0,
            cached: 58928,
            shmem: 0,
            swap_total: 0, // Reads 0 swap
            swap_free: 0,  // Reads 0 swap
            swap_used: 0,
        };
        assert_eq!(expected, memory);
        assert_eq!(memory.total, memory.used + memory.free);
        assert_eq!(memory.swap_total, memory.swap_used + memory.swap_free);
    }
}

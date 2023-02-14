#[cfg(target_os = "linux")]
pub mod os {
    use std::io::BufRead;
    use std::path::Path;

    use super::super::Memory;
    use crate::{file_to_buf_reader, parse_u64};
    use crate::{path_to_string, ProbeError, Result};

    pub fn read_and_parse_v1_sys_memory(path: &Path) -> Result<Memory> {
        let mut memory = Memory {
            total: None,
            free: None,
            used: 0,
            buffers: 0,
            cached: 0,
            shmem: 0,
            swap_total: None,
            swap_free: None,
            swap_used: None,
        };

        let limit = read_file_value_as_u64(&path.join("memory.limit_in_bytes"))?;
        // Ignore number reported by cgroups v1 when no limit is set
        if limit < 9223372036854771712u64 {
            memory.total = Some(bytes_to_kilo_bytes(limit));
        }

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
        memory.free = memory.total.map(|total| total - memory.used);

        memory.swap_total = match read_file_value_as_u64(&path.join("memory.memsw.limit_in_bytes"))
        {
            Ok(value) => memory
                .total
                .map(|total| bytes_to_kilo_bytes(value).saturating_sub(total)),
            Err(_) => None,
        };
        memory.swap_used = match read_file_value_as_u64(&path.join("memory.memsw.usage_in_bytes")) {
            Ok(value) => Some(bytes_to_kilo_bytes(value).saturating_sub(used_memory)),
            Err(_) => None,
        };
        memory.swap_free = memory
            .swap_total
            .zip(memory.swap_used)
            .map(|(total, used)| total.saturating_sub(used));

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
    use super::super::Memory;
    use crate::ProbeError;
    use std::path::Path;

    #[test]
    fn test_read_and_parse_v1_sys_memory() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v1/memory/");
        let memory = super::os::read_and_parse_v1_sys_memory(&path).unwrap();

        let expected = Memory {
            total: Some(512000), // 500mb
            free: Some(503400),  // total - used
            used: 8600,
            buffers: 0,
            cached: 58928,
            shmem: 0,
            swap_total: Some(1_488_000), // reported swap total - reported memory total
            swap_free: Some(1_055_528),
            swap_used: Some(432_472), // reported swap used - (reported memory used, including cache)
        };
        assert_eq!(expected, memory);
        assert_eq!(memory.total.unwrap(), memory.used + memory.free.unwrap());
        assert_eq!(
            memory.swap_total.unwrap(),
            memory.swap_used.unwrap() + memory.swap_free.unwrap()
        );
    }

    #[test]
    fn test_read_and_parse_v1_sys_memory_wrong_path() {
        let path = Path::new("/nonsense");
        match super::os::read_and_parse_v1_sys_memory(&path) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v1_sys_memory_incomplete() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v1/memory_incomplete/");
        match super::os::read_and_parse_v1_sys_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v1_sys_memory_missing_files() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v1/memory_missing_files/");
        match super::os::read_and_parse_v1_sys_memory(&path) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v1_sys_memory_garbage() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v1/memory_garbage/");
        match super::os::read_and_parse_v1_sys_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v1_sys_memory_no_swap() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v1/memory_without_swap/");
        let memory = super::os::read_and_parse_v1_sys_memory(&path).unwrap();

        let expected = Memory {
            total: Some(512000), // 500mb
            free: Some(503400),  // total - used
            used: 8600,
            buffers: 0,
            cached: 58928,
            shmem: 0,
            swap_total: None, // Reads 0 swap
            swap_free: None,  // Reads 0 swap
            swap_used: None,
        };
        assert_eq!(expected, memory);
        assert_eq!(memory.total.unwrap(), memory.used + memory.free.unwrap());
        assert_eq!(memory.swap_total, None);
        assert_eq!(memory.swap_free, None);
        assert_eq!(memory.swap_used, None);
    }
}

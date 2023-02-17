use std::io::BufRead;
use std::path::Path;

use super::Memory;
use crate::{bytes_to_kilo_bytes, file_to_buf_reader, parse_u64, read_file_value_as_u64};
use crate::{path_to_string, ProbeError, Result};
#[cfg(target_os = "linux")]
pub fn read_and_parse_v2_sys_memory(path: &Path) -> Result<Memory> {
    let mut memory = Memory {
        total: None,
        free: None,
        used: 0,
        buffers: None,
        cached: None,
        shmem: None,
        swap_total: None,
        swap_free: None,
        swap_used: None,
    };

    memory.total = read_file_value_as_u64(&path.join("memory.max"))
        .ok()
        .map(bytes_to_kilo_bytes);

    memory.used = bytes_to_kilo_bytes(read_file_value_as_u64(&path.join("memory.current"))?);

    let reader = file_to_buf_reader(&path.join("memory.stat"))?;
    for line_result in reader.lines() {
        let line = line_result.map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
        let segments: Vec<&str> = line.split_whitespace().collect();
        let value = parse_u64(&segments[1])?;

        if segments[0] == "shmem" {
            memory.shmem = Some(bytes_to_kilo_bytes(value));
            break;
        };
    }

    memory.free = memory.total.map(|total| total - memory.used);

    memory.swap_total = read_file_value_as_u64(&path.join("memory.swap.max"))
        .ok()
        .map(bytes_to_kilo_bytes);
    memory.swap_used = read_file_value_as_u64(&path.join("memory.swap.current"))
        .ok()
        .map(bytes_to_kilo_bytes);
    memory.swap_free = memory
        .swap_total
        .zip(memory.swap_used)
        .map(|(total, used)| total.saturating_sub(used));

    Ok(memory)
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::super::Memory;
    use crate::ProbeError;
    use std::path::Path;

    #[test]
    fn test_read_and_parse_v2_sys_memory() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v2/memory/");
        let memory = super::read_and_parse_v2_sys_memory(&path).unwrap();

        let expected = Memory {
            total: Some(512000), // 500mb
            free: Some(444472),  // total - used
            used: 67528,
            buffers: None,
            cached: None,
            shmem: Some(0),
            swap_total: Some(2000000),  // reported swap total
            swap_free: Some(1_500_000), // swap total - swap used
            swap_used: Some(500_000),   // reported swap used
        };
        assert_eq!(expected, memory);
        assert_eq!(memory.total.unwrap(), memory.used + memory.free.unwrap());
        assert_eq!(
            memory.swap_total.unwrap(),
            memory.swap_used.unwrap() + memory.swap_free.unwrap()
        );
    }

    #[test]
    fn test_read_and_parse_v2_sys_memory_wrong_path() {
        let path = Path::new("/nonsense");
        match super::read_and_parse_v2_sys_memory(&path) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v2_sys_memory_incomplete() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v2/memory_incomplete/");
        match super::read_and_parse_v2_sys_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v1_sys_memory_missing_files() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v2/memory_missing_files/");
        match super::read_and_parse_v2_sys_memory(&path) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v1_sys_memory_garbage() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v2/memory_garbage/");
        match super::read_and_parse_v2_sys_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v1_sys_memory_no_swap() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v2/memory_without_swap/");
        let memory = super::read_and_parse_v2_sys_memory(&path).unwrap();

        let expected = Memory {
            total: Some(512000), // 500mb
            free: Some(444472),  // total - used
            used: 67528,
            buffers: None,
            cached: None,
            shmem: Some(0),
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

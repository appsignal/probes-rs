use super::Memory;
use crate::Result;

/// Read the current memory status of the system.
#[cfg(target_os = "linux")]
pub fn read() -> Result<Memory> {
    os::read()
}

#[cfg(target_os = "linux")]
mod os {
    use std::io::BufRead;
    use std::path::Path;

    use super::super::Memory;
    use crate::{file_to_buf_reader, parse_u64};
    use crate::{path_to_string, ProbeError, Result};

    const PROC_MEMORY_NUMBER_OF_FIELDS: usize = 7;

    #[inline]
    pub fn read() -> Result<Memory> {
        read_and_parse_proc_memory(&Path::new("/proc/meminfo"))
    }

    #[inline]
    pub fn read_and_parse_proc_memory(path: &Path) -> Result<Memory> {
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
                    memory.total = Some(value);
                    1
                }
                "MemFree:" => {
                    free = value;
                    1
                }
                "Buffers:" => {
                    memory.buffers = Some(value);
                    1
                }
                "Cached:" => {
                    memory.cached = Some(value);
                    1
                }
                "SwapTotal:" => {
                    memory.swap_total = Some(value);
                    1
                }
                "SwapFree:" => {
                    memory.swap_free = Some(value);
                    1
                }
                "Shmem:" => {
                    memory.shmem = Some(value);
                    1
                }
                _ => 0,
            };

            if fields_encountered == PROC_MEMORY_NUMBER_OF_FIELDS {
                break;
            }
        }

        if fields_encountered != PROC_MEMORY_NUMBER_OF_FIELDS || memory.total.is_none() {
            return Err(ProbeError::UnexpectedContent(
                "Did not encounter all expected fields".to_owned(),
            ));
        }

        // Total amount of free physical memory in Kb.
        // Includes buffers and caches, these will be freed
        // up by the OS when the memory is needed.
        memory.free = Some(free + memory.buffers.unwrap_or(0) + memory.cached.unwrap_or(0));
        memory.used = memory.total.unwrap() - memory.free.unwrap();
        memory.swap_used = memory
            .swap_total
            .zip(memory.swap_free)
            .map(|(total, free)| total - free);

        Ok(memory)
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::super::Memory;
    use crate::ProbeError;
    use std::path::Path;

    #[test]
    fn test_read_memory() {
        assert!(super::read().is_ok());
    }

    #[test]
    fn test_read_and_parse_proc_memory() {
        let path = Path::new("fixtures/linux/memory/proc_meminfo");
        let memory = super::os::read_and_parse_proc_memory(&path).unwrap();

        let expected = Memory {
            total: Some(376072),
            free: Some(324248),
            used: 51824,
            buffers: Some(22820),
            cached: Some(176324),
            shmem: Some(548),
            swap_total: Some(1101816),
            swap_free: Some(1100644),
            swap_used: Some(1172),
        };
        assert_eq!(expected, memory);
        assert_eq!(memory.total.unwrap(), memory.used + memory.free.unwrap());
        assert_eq!(
            memory.swap_total.unwrap(),
            memory.swap_used.unwrap() + memory.swap_free.unwrap()
        );
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
}

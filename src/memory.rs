use super::Result;

#[derive(Debug,PartialEq)]
pub struct Memory {
    total: u64,
    free: u64,
    buffers: u64,
    cached: u64,
    swap_total: u64,
    swap_free: u64,
}

const MEMORY_NUMBER_OF_FIELDS: usize = 6;

impl Memory {
    /// Total amount of physical memory in Kb.
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Total amount of free physical memory in Kb.
    /// Inclused buffers and caches, these will be freed
    /// up by the OS when the memory is needed.
    pub fn free(&self) -> u64 {
        self.free + self.buffers + self.cached
    }

    /// Total amount of used physical memory in Kb.
    pub fn used(&self) -> u64 {
        self.total() - self.free()
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
        self.swap_total() - self.swap_free()
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
    use super::super::{ProbeError,Result};
    use super::super::file_to_buf_reader;

    #[inline]
    pub fn read() -> Result<Memory> {
        read_and_parse_memory(&Path::new("/proc/meminfo"))
    }

    #[inline]
    pub fn read_and_parse_memory(path: &Path) -> Result<Memory> {
        let mut memory = Memory {
            total: 0,
            free: 0,
            buffers: 0,
            cached: 0,
            swap_total: 0,
            swap_free: 0,
        };

        let reader = try!(file_to_buf_reader(path));

        let mut fields_encountered = 0;
        for line in reader.lines() {
            let line = try!(line);
            let segments: Vec<&str> = line.split_whitespace().collect();
            let value: u64 = try!(segments[1].parse().map_err(|_| {
                ProbeError::UnexpectedContent(format!("Could not parse value for '{}'", segments[0]))
            }));

            // If this is a field we recognize set it's value and increment the
            // number of fields we encountered.
            fields_encountered += match segments[0] {
                "MemTotal:" => {
                    memory.total = value;
                    1
                },
                "MemFree:" => {
                    memory.free = value;
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

        Ok(memory)
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
    fn test_read_and_parse_memory() {
        let path = Path::new("fixtures/linux/memory/proc_meminfo");
        let memory = super::os::read_and_parse_memory(&path).unwrap();

        let expected = Memory {
            total: 376072,
            free: 125104,
            buffers: 22820,
            cached: 176324,
            swap_total: 1101816,
            swap_free: 1100644,
        };
        assert_eq!(expected, memory);
    }

    #[test]
    fn test_read_and_parse_memory_wrong_path() {
        let path = Path::new("/nonsense");
        match super::os::read_and_parse_memory(&path) {
            Err(ProbeError::IO(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_memory_incomplete() {
        let path = Path::new("fixtures/linux/memory/proc_meminfo_incomplete");
        match super::os::read_and_parse_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_read_and_parse_memory_garbage() {
        let path = Path::new("fixtures/linux/memory/proc_meminfo_garbage");
        match super::os::read_and_parse_memory(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r)
        }
    }

    #[test]
    fn test_memory_calculations() {
        let memory = Memory {
            total: 1000,
            free: 200,
            buffers: 100,
            cached: 100,
            swap_total: 500,
            swap_free: 100,
        };

        // Physical memory
        assert_eq!(1000, memory.total());
        assert_eq!(400, memory.free());
        assert_eq!(600, memory.used());

        // Swap space
        assert_eq!(500, memory.swap_total());
        assert_eq!(100, memory.swap_free());
        assert_eq!(400, memory.swap_used());
    }
}

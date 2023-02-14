pub mod cgroup;
mod cgroup_v1;
pub mod proc;

#[derive(Debug, PartialEq)]
pub struct Memory {
    total: Option<u64>,
    free: Option<u64>,
    used: u64,
    buffers: u64,
    cached: u64,
    shmem: u64,
    swap_total: Option<u64>,
    swap_free: Option<u64>,
    swap_used: Option<u64>,
}

impl Memory {
    /// Total amount of physical memory in Kb.
    pub fn total(&self) -> Option<u64> {
        self.total
    }

    pub fn free(&self) -> Option<u64> {
        self.free
    }

    /// Total amount of used physical memory in Kb.
    pub fn used(&self) -> u64 {
        self.used
    }

    /// Total amount of swap space in Kb.
    pub fn swap_total(&self) -> Option<u64> {
        self.swap_total
    }

    /// Total amount of free swap space in Kb.
    pub fn swap_free(&self) -> Option<u64> {
        self.swap_free
    }

    /// Total amount of used swap space in Kb.
    pub fn swap_used(&self) -> Option<u64> {
        self.swap_used
    }

    /// Total amount of shared memory
    pub fn shmem(&self) -> u64 {
        self.shmem
    }
}

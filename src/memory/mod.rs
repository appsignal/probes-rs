pub mod cgroup;
mod cgroup_v1;
mod cgroup_v2;
pub mod proc;

#[derive(Debug, PartialEq)]
pub struct Memory {
    pub total: Option<u64>,
    pub free: Option<u64>,
    pub used: u64,
    pub buffers: Option<u64>,
    pub cached: Option<u64>,
    pub shmem: Option<u64>,
    pub swap_total: Option<u64>,
    pub swap_free: Option<u64>,
    pub swap_used: Option<u64>,
}

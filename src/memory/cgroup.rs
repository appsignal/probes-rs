use std::path::Path;

use super::Memory;
use crate::{dir_exists, ProbeError, Result};

/// Read the current memory status of the container.
#[cfg(target_os = "linux")]
pub fn read() -> Result<Memory> {
    use super::cgroup_v1::read_and_parse_v1_sys_memory;
    use super::cgroup_v2::read_and_parse_v2_sys_memory;

    let v2_sys_fs_dir = Path::new("/sys/fs/cgroup");
    let v2_sys_fs_file = v2_sys_fs_dir.join("memory.current");

    if v2_sys_fs_file.exists() {
        return read_and_parse_v2_sys_memory(&v2_sys_fs_dir);
    }

    let v1_sys_fs_dir = Path::new("/sys/fs/cgroup/memory/");
    if dir_exists(v1_sys_fs_dir) {
        return read_and_parse_v1_sys_memory(&v1_sys_fs_dir);
    }

    let message = format!(
        "Directory `{}` not found",
        v1_sys_fs_dir.to_str().unwrap_or("unknown path")
    );
    Err(ProbeError::UnexpectedContent(message))
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    #[test]
    fn test_read_from_container() {
        assert!(super::read().is_ok());
    }
}

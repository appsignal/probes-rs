use std::path::Path;
use super::{file_to_string};

#[cfg(target_os = "linux")]
pub fn in_container() -> bool {
    determine_container_for_cgroups("/proc/self/cgroup")
}

fn determine_container_for_cgroups(path: &str) -> bool {
    match file_to_string(&Path::new(&path)) {
        Ok(buffer) => {
            buffer.contains("/docker") || buffer.contains("/lxc") ||
                buffer.contains("/kubepods")
        },
        Err(_) => false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_in_container() {
        assert!(super::determine_container_for_cgroups("fixtures/linux/proc/self/cgroup/docker"));
        assert!(super::determine_container_for_cgroups("fixtures/linux/proc/self/cgroup/docker_systemd"));
        assert!(super::determine_container_for_cgroups("fixtures/linux/proc/self/cgroup/lxc"));
        assert!(super::determine_container_for_cgroups("fixtures/linux/proc/self/cgroup/kubernetes"));
        assert!(!super::determine_container_for_cgroups("fixtures/linux/proc/self/cgroup/none"));
    }
}

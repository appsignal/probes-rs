use super::cgroup::{CgroupCpuMeasurement, CgroupCpuStat};
use crate::error::ProbeError;
use crate::{
    file_to_buf_reader, file_to_string, parse_u64, path_to_string, precise_time_ns,
    read_file_value_as_u64, Result,
};
use std::io::BufRead;
use std::path::Path;

const CPU_SYS_V1_NUMBER_OF_FIELDS: usize = 2;

#[cfg(target_os = "linux")]
pub fn read_and_parse_v1_sys_stat(
    path: &Path,
    cpu_period_path: &Path,
    cpu_quota_path: &Path,
) -> Result<CgroupCpuMeasurement> {
    let time = precise_time_ns();

    // If the CPU period and quota files exist, we can use it to calculate the number of CPUs in
    // the cgroup.
    let mut cpu_count = 0.0;
    if cpu_period_path.exists() && cpu_quota_path.exists() {
        let cpu_period = parse_u64(file_to_string(&cpu_period_path)?.trim())? as f64;
        let cpu_quota_raw = file_to_string(&cpu_quota_path)?.trim().to_string();
        // The value `-1` means no quota is set and we can't calculate the number of CPUs present.
        if cpu_quota_raw != "-1" {
            let cpu_quota = parse_u64(&cpu_quota_raw)? as f64;
            cpu_count = cpu_quota / cpu_period;
        }
    }

    let reader = file_to_buf_reader(&path.join("cpuacct.stat"))?;
    let total_usage = read_file_value_as_u64(&path.join("cpuacct.usage"))?;

    let mut cpu = CgroupCpuStat {
        total_usage,
        user: 0,
        system: 0,
    };
    if cpu_count > 0.0 {
        cpu.total_usage = (cpu.total_usage as f64 / cpu_count).round() as u64;
    }

    let mut fields_encountered = 0;
    for line in reader.lines() {
        let line = line.map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
        let segments: Vec<&str> = line.split_whitespace().collect();
        let value = parse_u64(&segments[1])?;
        fields_encountered += match segments[0] {
            "user" => {
                cpu.user = value * 10_000_000;
                1
            }
            "system" => {
                cpu.system = value * 10_000_000;
                1
            }
            _ => 0,
        };

        if fields_encountered == CPU_SYS_V1_NUMBER_OF_FIELDS {
            break;
        }
    }

    if fields_encountered != CPU_SYS_V1_NUMBER_OF_FIELDS {
        return Err(ProbeError::UnexpectedContent(
            "Did not encounter all expected fields".to_owned(),
        ));
    }
    let measurement = CgroupCpuMeasurement {
        precise_time_ns: time,
        stat: cpu,
    };
    Ok(measurement)
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod test {
    use super::read_and_parse_v1_sys_stat;
    use crate::error::ProbeError;
    use std::path::Path;

    #[test]
    fn test_read_v1_sys_measurement_no_quota() {
        let measurement = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_1/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
        )
        .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 152657213021);
        assert_eq!(cpu.user, 149340000000);
        assert_eq!(cpu.system, 980000000);
    }

    #[test]
    fn test_read_v1_sys_measurement_one_cpu() {
        let measurement = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_1/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_period_us"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_quota_us.one_cpu"),
        )
        .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 152657213021);
        assert_eq!(cpu.user, 149340000000);
        assert_eq!(cpu.system, 980000000);
    }

    #[test]
    fn test_read_v1_sys_measurement_two_cpu() {
        let measurement = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_1/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_period_us"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_quota_us.two_cpu"),
        )
        .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 76328606511);
        assert_eq!(cpu.user, 149340000000);
        assert_eq!(cpu.system, 980000000);
    }

    #[test]
    fn test_read_v1_sys_measurement_half_cpu() {
        let measurement = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_1/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_period_us"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_quota_us.half_cpu"),
        )
        .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 305314426042);
        assert_eq!(cpu.user, 149340000000);
        assert_eq!(cpu.system, 980000000);
    }

    #[test]
    fn test_read_v1_sys_measurement_minus_one() {
        let measurement = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_1/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_period_us"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_quota_us.minus_one"),
        )
        .unwrap();
        let cpu = measurement.stat;
        // Does not divide by the number of CPUs
        assert_eq!(cpu.total_usage, 152657213021);
        assert_eq!(cpu.user, 149340000000);
        assert_eq!(cpu.system, 980000000);
    }

    #[test]
    fn test_read_v1_sys_wrong_path() {
        match read_and_parse_v1_sys_stat(
            &Path::new("bananas"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
        ) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v1_sys_stat_incomplete() {
        match read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_incomplete/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
        ) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v1_sys_stat_garbage() {
        match read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_garbage/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
        ) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_in_percentages_integration_v1() {
        let mut measurement1 = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_1/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
        )
        .unwrap();
        measurement1.precise_time_ns = 375953965125920;
        let mut measurement2 = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_2/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/does_not_exist"),
        )
        .unwrap();
        measurement2.precise_time_ns = 376013815302920;

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();
        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.
        assert!(in_percentages.total_usage > 49.70);
        assert!(in_percentages.total_usage < 49.71);

        assert!(in_percentages.user > 47.60);
        assert!(in_percentages.user < 47.61);

        assert!(in_percentages.system > 0.38);
        assert!(in_percentages.system < 0.39);
    }

    #[test]
    fn test_in_percentages_integration_two_cpu() {
        let mut measurement1 = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_1/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_period_us"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_quota_us.two_cpu"),
        )
        .unwrap();
        measurement1.precise_time_ns = 375953965125920;
        let mut measurement2 = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_2/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_period_us"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_quota_us.two_cpu"),
        )
        .unwrap();
        measurement2.precise_time_ns = 376013815302920;

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();
        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.
        assert!(in_percentages.total_usage > 24.85);
        assert!(in_percentages.total_usage < 24.86);

        assert!(in_percentages.user > 47.60);
        assert!(in_percentages.user < 47.61);

        assert!(in_percentages.system > 0.38);
        assert!(in_percentages.system < 0.39);
    }

    #[test]
    fn test_in_percentages_integration_half_cpu() {
        let mut measurement1 = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_1/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_period_us"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_quota_us.half_cpu"),
        )
        .unwrap();
        measurement1.precise_time_ns = 375953965125920;
        let mut measurement2 = read_and_parse_v1_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpuacct_2/"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_period_us"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v1/cpu_quota/cpu.cfs_quota_us.half_cpu"),
        )
        .unwrap();
        measurement2.precise_time_ns = 376013815302920;

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();
        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.
        assert!(in_percentages.total_usage > 99.40);
        assert!(in_percentages.total_usage < 99.41);

        assert!(in_percentages.user > 47.60);
        assert!(in_percentages.user < 47.61);

        assert!(in_percentages.system > 0.38);
        assert!(in_percentages.system < 0.39);
    }
}

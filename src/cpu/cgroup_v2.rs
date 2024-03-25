use super::cgroup::{CgroupCpuMeasurement, CgroupCpuStat};
use crate::error::ProbeError;
use crate::{file_to_buf_reader, parse_u64, path_to_string, precise_time_ns, Result};
use std::io::BufRead;
use std::path::Path;

const CPU_SYS_V2_NUMBER_OF_FIELDS: usize = 3;

#[cfg(target_os = "linux")]
pub fn read_and_parse_v2_sys_stat(
    path: &Path,
    cpu_max_path: &Path,
    mut cpu_count: Option<f64>,
) -> Result<CgroupCpuMeasurement> {
    // If the cpu.max file exists, we can use it to calculate the number of CPUs
    // in the cgroup. It's also required that the first value is not set to "max",
    // otherwise we can't calculate the number of CPUs.
    if cpu_count.is_none() {
        if cpu_max_path.exists() {
            let reader = file_to_buf_reader(&cpu_max_path)?;
            let mut lines = reader.lines();
            if let Some(Ok(line)) = lines.next() {
                let segments: Vec<&str> = line.split_whitespace().collect();
                let max = segments[0];

                if max != "max" {
                    let period = parse_u64(&segments[1])? as f64;
                    cpu_count = Some(parse_u64(&max)? as f64 / period);
                }
            }
        }
    }

    let time = precise_time_ns();
    let reader = file_to_buf_reader(&path)?;

    let mut cpu = CgroupCpuStat {
        total_usage: 0,
        user: 0,
        system: 0,
    };

    let mut fields_encountered = 0;
    for line in reader.lines() {
        let line = line.map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
        let segments: Vec<&str> = line.split_whitespace().collect();
        let value = parse_u64(&segments[1])?;
        fields_encountered += match segments[0] {
            "usage_usec" => {
                cpu.total_usage = value * 1_000;
                1
            }
            "user_usec" => {
                cpu.user = value * 1_000;
                1
            }
            "system_usec" => {
                cpu.system = value * 1_000;
                1
            }
            _ => 0,
        };

        if fields_encountered == CPU_SYS_V2_NUMBER_OF_FIELDS {
            break;
        }
    }

    if fields_encountered != CPU_SYS_V2_NUMBER_OF_FIELDS {
        return Err(ProbeError::UnexpectedContent(
            "Did not encounter all expected fields".to_owned(),
        ));
    }
    let measurement = CgroupCpuMeasurement {
        precise_time_ns: time,
        stat: cpu.by_cpu_count(cpu_count),
    };
    Ok(measurement)
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod test {
    use super::read_and_parse_v2_sys_stat;
    use crate::error::ProbeError;
    use std::{option::Option::None, path::Path};

    #[test]
    fn test_read_v2_sys_measurement_default_cpu_max() {
        let measurement = read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_default"),
            None,
        )
        .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 171462000);
        assert_eq!(cpu.user, 53792000);
        assert_eq!(cpu.system, 117670000);
    }

    #[test]
    fn test_read_v2_sys_measurement_2_cpus() {
        let measurement = read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_2_cpus"),
            None,
        )
        .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 85731000);
        assert_eq!(cpu.user, 26896000);
        assert_eq!(cpu.system, 58835000);
    }

    #[test]
    fn test_read_v2_sys_measurement_half_usage() {
        let measurement = read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_half"),
            None,
        )
        .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 342924000);
        assert_eq!(cpu.user, 107584000);
        assert_eq!(cpu.system, 235340000);
    }

    #[test]
    fn test_read_v2_sys_one_cpu_count() {
        let measurement = read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_garbage"),
            Some(1.0),
        )
        .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 171462000);
        assert_eq!(cpu.user, 53792000);
        assert_eq!(cpu.system, 117670000);
    }

    #[test]
    fn test_read_v2_sys_measurement_two_cpu_count() {
        let measurement = read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_garbage"),
            Some(2.0),
        )
        .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 85731000);
        assert_eq!(cpu.user, 26896000);
        assert_eq!(cpu.system, 58835000);
    }

    #[test]
    fn test_read_v2_sys_measurement_half_cpu_count() {
        let measurement = read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_garbage"),
            Some(0.5),
        )
        .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 342924000);
        assert_eq!(cpu.user, 107584000);
        assert_eq!(cpu.system, 235340000);
    }

    #[test]
    fn test_read_v2_sys_wrong_path() {
        match read_and_parse_v2_sys_stat(&Path::new("bananas"), &Path::new("potato"), None) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v2_sys_stat_incomplete() {
        match read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_incomplete"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_default"),
            None,
        ) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v2_sys_stat_garbage() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_garbage");
        let max_file_path = Path::new("fixtures/linux/fs/cgroup_v2/cpu.max");
        match read_and_parse_v2_sys_stat(&path, &max_file_path, None) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v2_sys_max_garbage() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1");
        let max_file_path = Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_garbage");
        match read_and_parse_v2_sys_stat(&path, &max_file_path, None) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_in_percentages_integration_v2_two_cpu() {
        let mut measurement1 = read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_2_cpus"),
            None,
        )
        .unwrap();
        measurement1.precise_time_ns = 375953965125920;
        let mut measurement2 = read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_2"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_2_cpus"),
            None,
        )
        .unwrap();
        measurement2.precise_time_ns = 376013815302920;

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();
        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.
        assert!(in_percentages.total_usage > 0.08);
        assert!(in_percentages.total_usage < 0.09);

        assert!(in_percentages.user > 0.01);
        assert!(in_percentages.user < 0.02);

        assert!(in_percentages.system > 0.06);
        assert!(in_percentages.system < 0.07);
    }

    // When the cpu.max file does not return an integer.
    #[test]
    fn test_in_percentages_integration_v2_half_cpu() {
        let mut measurement1 = read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_half"),
            None,
        )
        .unwrap();
        measurement1.precise_time_ns = 375953965125920;
        let mut measurement2 = read_and_parse_v2_sys_stat(
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_2"),
            &Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.max_half"),
            None,
        )
        .unwrap();
        measurement2.precise_time_ns = 376013815302920;

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();
        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.
        assert!(in_percentages.total_usage > 0.33);
        assert!(in_percentages.total_usage < 0.34);

        assert!(in_percentages.user > 0.05);
        assert!(in_percentages.user < 0.06);

        assert!(in_percentages.system > 0.27);
        assert!(in_percentages.system < 0.28);
    }
}

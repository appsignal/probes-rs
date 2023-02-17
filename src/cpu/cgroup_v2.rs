use super::cgroup::{CgroupCpuMeasurement, CgroupCpuStat};
use crate::error::ProbeError;
use crate::{file_to_buf_reader, parse_u64, path_to_string, precise_time_ns, Result};
use std::io::BufRead;
use std::path::Path;

const CPU_SYS_V2_NUMBER_OF_FIELDS: usize = 3;

#[cfg(target_os = "linux")]
pub fn read_and_parse_v2_sys_stat(path: &Path) -> Result<CgroupCpuMeasurement> {
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
        stat: cpu,
    };
    Ok(measurement)
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod test {
    use super::read_and_parse_v2_sys_stat;
    use crate::error::ProbeError;
    use std::path::Path;

    #[test]
    fn test_read_v2_sys_measurement() {
        let measurement =
            read_and_parse_v2_sys_stat(&Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1"))
                .unwrap();
        let cpu = measurement.stat;
        assert_eq!(cpu.total_usage, 171462000);
        assert_eq!(cpu.user, 53792000);
        assert_eq!(cpu.system, 117670000);
    }

    #[test]
    fn test_read_v2_sys_wrong_path() {
        match read_and_parse_v2_sys_stat(&Path::new("bananas")) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v2_sys_stat_incomplete() {
        match read_and_parse_v2_sys_stat(&Path::new(
            "fixtures/linux/sys/fs/cgroup_v2/cpu.stat_incomplete",
        )) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_v2_sys_stat_garbage() {
        let path = Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_garbage");
        match read_and_parse_v2_sys_stat(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_in_percentages_integration_v2() {
        let mut measurement1 =
            read_and_parse_v2_sys_stat(&Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_1"))
                .unwrap();
        measurement1.precise_time_ns = 375953965125920;
        let mut measurement2 =
            read_and_parse_v2_sys_stat(&Path::new("fixtures/linux/sys/fs/cgroup_v2/cpu.stat_2"))
                .unwrap();
        measurement2.precise_time_ns = 376013815302920;

        let stat = measurement1.calculate_per_minute(&measurement2).unwrap();
        let in_percentages = stat.in_percentages();

        // Rounding in the floating point calculations can vary, so check if this
        // is in the correct range.
        assert!(in_percentages.total_usage > 0.16);
        assert!(in_percentages.total_usage < 0.17);

        assert!(in_percentages.user > 0.02);
        assert!(in_percentages.user < 0.03);

        assert!(in_percentages.system > 0.13);
        assert!(in_percentages.system < 0.14);
    }
}

use super::Result;

#[derive(Debug, PartialEq)]
pub struct DiskUsage {
    pub filesystem: Option<String>,
    pub one_k_blocks: u64,
    pub one_k_blocks_used: u64,
    pub one_k_blocks_free: u64,
    pub used_percentage: u32,
    pub mountpoint: String,
}

#[derive(Debug, PartialEq)]
pub struct DiskInodeUsage {
    pub filesystem: Option<String>,
    pub inodes: u64,
    pub iused: u64,
    pub ifree: u64,
    pub iused_percentage: u32,
    pub mountpoint: String,
}

/// Read the current usage of all disks
#[cfg(target_os = "linux")]
pub fn read() -> Result<Vec<DiskUsage>> {
    os::read()
}

/// Read the current inode usage of all disks
#[cfg(target_os = "linux")]
pub fn read_inodes() -> Result<Vec<DiskInodeUsage>> {
    os::read_inodes()
}

#[cfg(target_os = "linux")]
mod os {
    use super::super::{parse_u64, ProbeError, Result};
    use super::{DiskInodeUsage, DiskUsage};
    use std::process::Command;

    #[inline]
    pub fn read() -> Result<Vec<DiskUsage>> {
        let mut out: Vec<DiskUsage> = Vec::new();
        let local_out = match disk_fs_local_raw(Some(&["--local"])) {
            Ok(o) => o,
            Err(_) => match disk_fs_local_raw(None) {
                Ok(o) => o,
                Err(e) => return Err(e),
            },
        };

        let parsed = parse_df_output(&local_out)?;

        for segment in parsed.iter() {
            let usage = DiskUsage {
                filesystem: parse_filesystem(segment[0]),
                one_k_blocks: parse_u64(segment[1])?,
                one_k_blocks_used: parse_u64(segment[2])?,
                one_k_blocks_free: parse_u64(segment[3])?,
                used_percentage: parse_percentage_segment(segment[4])?,
                mountpoint: segment[5].to_string(),
            };

            out.push(usage);
        }

        Ok(out)
    }

    #[inline]
    pub fn read_inodes() -> Result<Vec<DiskInodeUsage>> {
        let inodes_out = disk_fs_inodes_raw()?;
        parse_df_inodes_output(parse_df_output(&inodes_out)?)
    }

    #[inline]
    pub fn parse_df_inodes_output(parsed_segments: Vec<Vec<&str>>) -> Result<Vec<DiskInodeUsage>> {
        let mut out: Vec<DiskInodeUsage> = Vec::new();

        for segment in parsed_segments.iter() {
            let iuse_percentage = segment[4];
            if iuse_percentage == "-" {
                continue;
            }
            let usage = DiskInodeUsage {
                filesystem: parse_filesystem(segment[0]),
                inodes: parse_u64(segment[1])?,
                iused: parse_u64(segment[2])?,
                ifree: parse_u64(segment[3])?,
                iused_percentage: parse_percentage_segment(iuse_percentage)?,
                mountpoint: segment[5].to_string(),
            };

            out.push(usage);
        }

        Ok(out)
    }

    #[inline]
    pub fn parse_df_output(output: &str) -> Result<Vec<Vec<&str>>> {
        let mut out: Vec<Vec<&str>> = Vec::new();

        // Sometimes the filesystem is on a separate line
        let mut filesystem_on_previous_line: Option<&str> = None;

        for line in output.split("\n").skip(1) {
            let mut segments: Vec<&str> = line.split_whitespace().collect();

            match segments.len() {
                0 => {
                    // Skip
                }
                1 => filesystem_on_previous_line = Some(segments[0]),
                5 => {
                    // Filesystem should be on the previous line
                    if let Some(fs) = filesystem_on_previous_line {
                        // Get filesystem first
                        let mut disk = vec![fs];
                        disk.append(&mut segments);

                        out.push(disk);

                        // Reset this to none
                        filesystem_on_previous_line = None;
                    } else {
                        return Err(ProbeError::UnexpectedContent(
                            "filesystem expected on previous line".to_owned(),
                        ));
                    }
                }
                6 => {
                    // All information is on 1 line
                    out.push(segments);
                }
                _ => {
                    return Err(ProbeError::UnexpectedContent(
                        "Incorrect number of segments".to_owned(),
                    ));
                }
            }
        }

        Ok(out)
    }

    #[inline]
    fn parse_percentage_segment(segment: &str) -> Result<u32> {
        // Strip % from the used value
        let segment_minus_percentage = &segment[..segment.len() - 1];

        segment_minus_percentage.parse().map_err(|_| {
            ProbeError::UnexpectedContent("Could not parse percentage segment".to_owned())
        })
    }

    #[inline]
    fn parse_filesystem(segment: &str) -> Option<String> {
        match segment {
            "none" => None,
            value => Some(value.to_string()),
        }
    }

    #[inline]
    fn disk_fs_inodes_raw() -> Result<String> {
        let output = Command::new("df")
            .arg("-i")
            .output()
            .map_err(|e| ProbeError::IO(e, "df -i".to_owned()))?
            .stdout;

        Ok(String::from_utf8_lossy(&output).to_string())
    }

    #[inline]
    fn disk_fs_local_raw(options: Option<&[&str]>) -> Result<String> {
        let mut cmd = Command::new("df");
        if let Some(opts) = options {
            cmd.args(opts);
        }

        let output = cmd
            .output()
            .map_err(|e| ProbeError::IO(e, format!("df {}", options.unwrap_or(&[]).join(" "))))?;
        let stdout = output.stdout;
        let status = output.status;

        if status.success() {
            Ok(String::from_utf8_lossy(&stdout).to_string())
        } else {
            Err(ProbeError::StatusFailure(format!(
                "Command `df` returned failure exit code '{:?}': df {}",
                status.code(),
                options.unwrap_or(&[]).join(" ")
            )))
        }
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::super::file_to_string;
    use super::super::ProbeError;
    use std::path::Path;

    #[test]
    fn test_read_disks() {
        assert!(super::read().is_ok());
        assert!(!super::read().unwrap().is_empty());
    }

    #[test]
    fn test_parse_df_output() {
        let expected = vec![
            vec![
                "/dev/mapper/lucid64-root",
                "81234688",
                "2344444",
                "74763732",
                "4%",
                "/",
            ],
            vec!["none", "183176", "180", "182996", "1%", "/dev"],
            vec!["/dev/sda1", "233191", "17217", "203533", "8%", "/boot"],
        ];

        let df = file_to_string(Path::new("fixtures/linux/disk_usage/df")).unwrap();
        let disks = super::os::parse_df_output(&df).unwrap();

        assert_eq!(expected, disks);
    }

    #[test]
    fn test_parse_df_i_output() {
        let expected = vec![
            vec!["overlay", "2097152", "122591", "1974561", "6%", "/"],
            vec!["tmpfs", "254863", "16", "254847", "1%", "/dev"],
            vec!["tmpfs", "254863", "15", "254848", "1%", "/sys/fs/cgroup"],
        ];

        let df = file_to_string(Path::new("fixtures/linux/disk_usage/df_i")).unwrap();
        let disks = super::os::parse_df_output(&df).unwrap();

        assert_eq!(expected, disks);
    }

    #[test]
    fn test_parse_df_output_incomplete() {
        let df = file_to_string(Path::new("fixtures/linux/disk_usage/df_incomplete")).unwrap();
        match super::os::parse_df_output(&df) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_parse_df_output_garbage() {
        let df = file_to_string(Path::new("fixtures/linux/disk_usage/df_garbage")).unwrap();
        match super::os::parse_df_output(&df) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_parse_df_i_output_dash_percentage() {
        let df =
            file_to_string(Path::new("fixtures/linux/disk_usage/df_i_dash_percentage")).unwrap();
        let disks =
            super::os::parse_df_inodes_output(super::os::parse_df_output(&df).unwrap()).unwrap();

        // Does not include the mountpoint with a dash (-) as a percentage
        assert_eq!(
            disks,
            vec![super::DiskInodeUsage {
                filesystem: Some("overlay".to_string()),
                inodes: 2097152,
                iused: 122591,
                ifree: 1974561,
                iused_percentage: 6,
                mountpoint: "/".to_string(),
            }]
        );
    }
}

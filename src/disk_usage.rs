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
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    const DF_TIMEOUT_IN_S: u64 = 5;

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
        run_with_timeout(DF_TIMEOUT_IN_S, "df", &["-i"])
    }

    #[inline]
    fn run_with_timeout(timeout_in_seconds: u64, command: &str, args: &[&str]) -> Result<String> {
        let timeout = Duration::from_secs(timeout_in_seconds);
        let start = Instant::now();

        let mut child = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            // Discard stderr, as `Command::output()` did before 0.7.4: `read()`
            // tries `df --local` first and falls back to `df`, and BusyBox's
            // `df` rejects `--local` with a usage message that otherwise ends
            // up in the host application's logs on every collection.
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| ProbeError::IO(e, command.to_owned()))?;

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        let mut output = String::new();
                        if let Some(mut stdout) = child.stdout.take() {
                            stdout
                                .read_to_string(&mut output)
                                .map_err(|e| ProbeError::IO(e, command.to_owned()))?;
                        }
                        return Ok(output);
                    } else {
                        return Err(ProbeError::StatusFailure(format!(
                            "Command `{} {}` failed with status: {}",
                            command,
                            args.join(" "),
                            status
                        )));
                    }
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        child.kill().ok();
                        child.wait().ok();
                        return Err(ProbeError::StatusFailure(format!(
                            "Command `{} {}` timed out after {}s",
                            command,
                            args.join(" "),
                            timeout_in_seconds
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(ProbeError::IO(e, command.to_owned())),
            }
        }
    }

    #[inline]
    fn disk_fs_local_raw(options: Option<&[&str]>) -> Result<String> {
        run_with_timeout(DF_TIMEOUT_IN_S, "df", options.unwrap_or(&[]))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::time::Instant;

        #[test]
        fn test_run_with_timeout_normal() {
            let start = Instant::now();
            let result = run_with_timeout(2, "sh", &["-c", "sleep 1 && echo hello"]);
            let elapsed = start.elapsed();

            assert!(result.is_ok());
            assert!(result.unwrap().contains("hello"));
            assert!(
                elapsed.as_secs_f64() < 2.0,
                "expected result before ~2s, but took {}s",
                elapsed.as_secs_f64()
            );
        }

        #[test]
        fn test_run_with_timeout_hung() {
            let start = Instant::now();
            let result = run_with_timeout(1, "sleep", &["2"]);
            let elapsed = start.elapsed();

            assert!(result.is_err());
            assert!(
                elapsed.as_secs_f64() < 1.5,
                "expected timeout after 1s, but took {}s",
                elapsed.as_secs_f64()
            );
        }

        #[test]
        fn test_run_with_timeout_failure() {
            let result = run_with_timeout(2, "ls", &["--made-up-flag"]);

            assert!(result.is_err());
            match result {
                Err(ProbeError::StatusFailure(msg)) => {
                    assert!(
                        msg.contains("ls --made-up-flag"),
                        "expected error to mention the command, got: {}",
                        msg
                    );
                }
                other => panic!("expected StatusFailure, got: {:?}", other),
            }
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

use super::Result;

pub type DiskUsages = Vec<DiskUsage>;

#[derive(Debug, PartialEq)]
pub struct DiskUsage {
    pub filesystem: Option<String>,
    pub one_k_blocks: u64,
    pub one_k_blocks_used: u64,
    pub one_k_blocks_free: u64,
    pub used_percentage: u32,
    pub mountpoint: String,
}

/// Read the current usage of all disks
#[cfg(target_os = "linux")]
pub fn read() -> Result<DiskUsages> {
    os::read()
}

#[cfg(target_os = "linux")]
mod os {
    use super::super::{parse_u64, ProbeError, Result};
    use super::{DiskUsage, DiskUsages};
    use std::process::Command;

    #[inline]
    pub fn read() -> Result<DiskUsages> {
        let output = Command::new("df")
            .arg("-l")
            .output()
            .map_err(|e| ProbeError::IO(e, "df -l".to_owned()))?
            .stdout;
        let output_string = String::from_utf8_lossy(&output);

        parse_df_output(output_string.as_ref())
    }

    #[inline]
    pub fn parse_df_output(output: &str) -> Result<DiskUsages> {
        let lines = output.split('\n');

        let mut out: DiskUsages = Vec::new();

        // Sometimes the filesystem is on a separate line
        let mut filesystem_on_previous_line: Option<String> = None;

        for line in lines.skip(1) {
            let segments: Vec<&str> = line.split_whitespace().collect();
            let segments_len = segments.len();
            if segments_len == 1 {
                filesystem_on_previous_line = Some(segments[0].to_string())
            } else if segments_len == 6 {
                // All information is on 1 line

                // Get filesystem
                let filesystem = match segments[0] {
                    "none" => None,
                    value => Some(value.to_string()),
                };

                let disk = DiskUsage {
                    filesystem,
                    one_k_blocks: parse_u64(segments[1])?,
                    one_k_blocks_used: parse_u64(segments[2])?,
                    one_k_blocks_free: parse_u64(segments[3])?,
                    used_percentage: parse_percentage_segment(&segments[4])?,
                    mountpoint: segments[5].to_string(),
                };

                out.push(disk);
            } else if segments_len == 5 {
                // Filesystem should be on the previous line

                match filesystem_on_previous_line {
                    Some(ref previous_filesystem) => {
                        // Get filesystem
                        let filesystem = match previous_filesystem.as_ref() {
                            "none" => None,
                            value => Some(value.to_string()),
                        };

                        let disk = DiskUsage {
                            filesystem,
                            one_k_blocks: parse_u64(segments[0])?,
                            one_k_blocks_used: parse_u64(segments[1])?,
                            one_k_blocks_free: parse_u64(segments[2])?,
                            used_percentage: parse_percentage_segment(&segments[3])?,
                            mountpoint: segments[4].to_string(),
                        };

                        out.push(disk);
                    }
                    None => {
                        return Err(ProbeError::UnexpectedContent(
                            "filesystem expected on previous line".to_owned(),
                        ))
                    }
                }

                // Reset this to none
                filesystem_on_previous_line = None;
            } else if segments_len == 0 {
                // Skip
            } else {
                return Err(ProbeError::UnexpectedContent(
                    "Incorrect number of segments".to_owned(),
                ));
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
}

#[cfg(test)]
mod tests {
    use super::super::file_to_string;
    use super::super::ProbeError;
    use super::DiskUsage;
    use std::path::Path;

    #[test]
    fn test_read_disks() {
        assert!(super::read().is_ok());
        assert!(!super::read().unwrap().is_empty());
    }

    #[test]
    fn test_parse_df_output() {
        let expected = vec![
            DiskUsage {
                filesystem: Some("/dev/mapper/lucid64-root".to_owned()),
                one_k_blocks: 81234688,
                one_k_blocks_used: 2344444,
                one_k_blocks_free: 74763732,
                used_percentage: 4,
                mountpoint: "/".to_owned(),
            },
            DiskUsage {
                filesystem: None,
                one_k_blocks: 183176,
                one_k_blocks_used: 180,
                one_k_blocks_free: 182996,
                used_percentage: 1,
                mountpoint: "/dev".to_owned(),
            },
            DiskUsage {
                filesystem: Some("/dev/sda1".to_owned()),
                one_k_blocks: 233191,
                one_k_blocks_used: 17217,
                one_k_blocks_free: 203533,
                used_percentage: 8,
                mountpoint: "/boot".to_owned(),
            },
        ];

        let df = file_to_string(Path::new("fixtures/linux/disk_usage/df")).unwrap();
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
}

use super::Result;

pub struct DiskUsage {
    pub file_system: String,
    pub total_bytes: u32,
    pub used_bytes: u32,
    pub available_bytes: u32
}

#[cfg(target_os = "linux")]
pub fn read() -> Result<Vec<DiskUsage>> {
    let df = os::run_df();
    os::read_disk_usage(df);
}

#[cfg(target_os = "linux")]
mod os {
    use super::DiskUsage;
    use super::super::Result;

    pub fn run_df() -> String {
        "todo".to_owned()
    }

    pub fn read_disk_usage(dfoutput: String) -> Result<Vec<DiskUsage>> {
        Ok(dfoutput
            .lines()
            .skip(2)
            .filter(is_local_device)
            .map(parse)
            .collect::<Vec<DiskUsage>>())
    }


    fn is_local_device(line: &&str) -> bool {
        line.starts_with("/dev/")
    }

    fn parse(line: &str) -> Result<DiskUsage> {
        let stats = line.split_whitespace().collect();

        Ok(DiskUsage {
            file_system: try!(stats[0].parse()),
            total_bytes: try!(stats[1].parse()),
            used_bytes: try!(stats[2].parse()),
            available_bytes: try!(stats[3].parse())
        })
    }
}

#[cfg(test)]
mod test {
    use super::os::read_disk_usage;
    use super::super::file_to_string;
    use std::path::Path;

    fn test_disk_usage_single_device() {
        let test_data = file_to_string(&Path::new("fixtures/linux/disk_usage/single_device")).unwrap();
        let usage = read_disk_usage(test_data);
        assert_eq!(usage.length, 1);

        let dev = usage[0];
        assert_eq!(dev.total_bytes, 100294088);
        assert_eq!(dev.used_bytes, 61759264);
        assert_eq!(dev.available_bytes, 33417084);
        assert_eq!(dev.device, "/dev/sda1");
    }

    // multiple device
    // only dev
}

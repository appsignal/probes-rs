use super::{calculate_time_difference, ProbeError, Result};
use std::collections::HashMap;

pub type Interfaces = HashMap<String, NetworkTraffic>;

/// Measurement of network traffic at a certain time.
#[derive(Debug, PartialEq)]
pub struct NetworkTrafficMeasurement {
    pub precise_time_ns: u64,
    pub interfaces: Interfaces,
}

impl NetworkTrafficMeasurement {
    /// Calculate the network traffic per minute based on this measurement and a measurement in the
    /// future. It is advisable to make the next measurement roughly a minute from this one for the
    /// most reliable result.
    pub fn calculate_per_minute(
        &self,
        next_measurement: &NetworkTrafficMeasurement,
    ) -> Result<NetworkTrafficPerMinute> {
        let time_difference =
            calculate_time_difference(self.precise_time_ns, next_measurement.precise_time_ns)?;

        let mut interfaces = Interfaces::new();

        for (name, traffic) in self.interfaces.iter() {
            let next_traffic = match next_measurement.interfaces.get(name) {
                Some(interface) => interface,
                None => {
                    return Err(ProbeError::UnexpectedContent(format!(
                        "{} is not present in the next measurement",
                        name
                    )))
                }
            };
            interfaces.insert(
                name.to_string(),
                NetworkTraffic {
                    received: super::time_adjusted(
                        "received",
                        next_traffic.received,
                        traffic.received,
                        time_difference,
                    )?,
                    transmitted: super::time_adjusted(
                        "transmitted",
                        next_traffic.transmitted,
                        traffic.transmitted,
                        time_difference,
                    )?,
                },
            );
        }

        Ok(NetworkTrafficPerMinute { interfaces })
    }
}

/// Network traffic in bytes.
#[derive(Debug, PartialEq)]
pub struct NetworkTraffic {
    pub received: u64,
    pub transmitted: u64,
}

/// Network traffic for a certain minute, calculated based on two measurements.
#[derive(Debug, PartialEq)]
pub struct NetworkTrafficPerMinute {
    pub interfaces: Interfaces,
}

#[cfg(target_os = "linux")]
pub fn read() -> Result<NetworkTrafficMeasurement> {
    os::read()
}

#[cfg(target_os = "linux")]
mod os {
    use std::io::{self, BufRead};
    use std::path::Path;

    use super::super::{file_to_buf_reader, parse_u64, path_to_string, precise_time_ns, Result};
    use super::{Interfaces, NetworkTraffic, NetworkTrafficMeasurement};
    use crate::error::ProbeError;

    #[inline]
    pub fn read() -> Result<NetworkTrafficMeasurement> {
        read_and_parse_network(&Path::new("/proc/net/dev"))
    }

    #[inline]
    pub fn read_and_parse_network(path: &Path) -> Result<NetworkTrafficMeasurement> {
        let reader = file_to_buf_reader(path)?;

        let precise_time_ns = precise_time_ns();

        let line_result: io::Result<Vec<String>> = reader.lines().collect();
        let lines = line_result.map_err(|e| ProbeError::IO(e, path_to_string(path)))?;
        let positions = get_positions(lines[1].as_ref())?;

        let mut interfaces = Interfaces::new();
        for line in &lines[2..] {
            let segments: Vec<&str> = line.split_whitespace().collect();
            let name = segments[0].trim_matches(':').to_owned();

            if segments.len() < positions.transmit_bytes {
                return Err(ProbeError::UnexpectedContent(format!(
                    "Expected at least {} items, had {} for '{}'",
                    positions.transmit_bytes,
                    segments.len(),
                    name
                )));
            }

            let traffic = NetworkTraffic {
                received: parse_u64(segments[positions.receive_bytes])?,
                transmitted: parse_u64(segments[positions.transmit_bytes])?,
            };

            interfaces.insert(name, traffic);
        }

        Ok(NetworkTrafficMeasurement {
            precise_time_ns,
            interfaces,
        })
    }

    #[derive(Debug, PartialEq)]
    pub struct Positions {
        pub receive_bytes: usize,
        pub transmit_bytes: usize,
    }

    /// Get the positions of the `bytes` field for both the receive and transmit segment
    #[inline]
    pub fn get_positions(header_line: &str) -> Result<Positions> {
        let groups: Vec<&str> = header_line.split('|').collect();
        if groups.len() != 3 {
            return Err(ProbeError::UnexpectedContent(
                "Incorrect number of segments".to_owned(),
            ));
        }
        let receive_group: Vec<&str> = groups[1].split_whitespace().collect();
        let transmit_group: Vec<&str> = groups[2].split_whitespace().collect();

        let receive_pos = receive_group
            .iter()
            .position(|&e| e == "bytes")
            .ok_or_else(|| {
                ProbeError::UnexpectedContent("bytes field not found for receive".to_string())
            })?;
        let transmit_pos = transmit_group
            .iter()
            .position(|&e| e == "bytes")
            .ok_or_else(|| {
                ProbeError::UnexpectedContent("bytes field not found for transmit".to_string())
            })?;

        // We start with 1 here because the first (name) segment always has one column.
        Ok(Positions {
            receive_bytes: 1 + receive_pos,
            transmit_bytes: 1 + receive_group.len() + transmit_pos,
        })
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::super::{precise_time_ns, ProbeError};
    use super::{Interfaces, NetworkTraffic, NetworkTrafficMeasurement};
    use std::path::Path;

    #[test]
    fn test_read_network() {
        assert!(super::read().is_ok());
        assert!(!super::read().unwrap().interfaces.is_empty());
    }

    #[test]
    fn test_read_and_parse_network() {
        let path = Path::new("fixtures/linux/network/proc_net_dev");
        let measurement = super::os::read_and_parse_network(&path).unwrap();

        assert!(measurement.precise_time_ns < precise_time_ns());

        let interfaces = measurement.interfaces;
        assert_eq!(3, interfaces.len());

        let lo = interfaces.get("lo").unwrap();
        assert_eq!(560, lo.received);
        assert_eq!(560, lo.transmitted);

        let eth0 = interfaces.get("eth0").unwrap();
        assert_eq!(254972, eth0.received);
        assert_eq!(72219, eth0.transmitted);

        let eth1 = interfaces.get("eth1").unwrap();
        assert_eq!(354972, eth1.received);
        assert_eq!(82219, eth1.transmitted);
    }

    #[test]
    fn test_read_and_parse_network_wrong_path() {
        let path = Path::new("/nonsense");
        match super::os::read_and_parse_network(&path) {
            Err(ProbeError::IO(_, _)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_network_incomplete() {
        let path = Path::new("fixtures/linux/network/proc_net_dev_incomplete");
        match super::os::read_and_parse_network(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_read_and_parse_network_garbage() {
        let path = Path::new("fixtures/linux/network/proc_net_dev_garbage");
        match super::os::read_and_parse_network(&path) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_get_positions() {
        let line = "face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed";

        assert_eq!(
            super::os::Positions {
                receive_bytes: 1,
                transmit_bytes: 9
            },
            super::os::get_positions(line).unwrap()
        )
    }

    #[test]
    fn test_get_positions_fields_missing() {
        let line = "face";

        match super::os::get_positions(line) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_get_positions_bytes_field_missing() {
        let line = "face |bates    packets errs drop fifo frame compressed multicast|bates    packets errs drop fifo colls carrier compressed";

        match super::os::get_positions(line) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_calculate_per_minute_full_minute() {
        let mut interfaces1 = Interfaces::new();
        interfaces1.insert(
            "eth0".to_string(),
            NetworkTraffic {
                received: 1000,
                transmitted: 1000,
            },
        );
        interfaces1.insert(
            "eth1".to_string(),
            NetworkTraffic {
                received: 2000,
                transmitted: 3000,
            },
        );
        let measurement1 = NetworkTrafficMeasurement {
            precise_time_ns: 60_000_000_000,
            interfaces: interfaces1,
        };

        let mut interfaces2 = Interfaces::new();
        interfaces2.insert(
            "eth0".to_string(),
            NetworkTraffic {
                received: 2000,
                transmitted: 2600,
            },
        );
        interfaces2.insert(
            "eth1".to_string(),
            NetworkTraffic {
                received: 3000,
                transmitted: 4600,
            },
        );
        let measurement2 = NetworkTrafficMeasurement {
            precise_time_ns: 120_000_000_000,
            interfaces: interfaces2,
        };

        let per_minute = measurement1.calculate_per_minute(&measurement2).unwrap();
        assert_eq!(2, per_minute.interfaces.len());

        let eth0 = per_minute.interfaces.get("eth0").unwrap();
        assert_eq!(1000, eth0.received);
        assert_eq!(1600, eth0.transmitted);

        let eth1 = per_minute.interfaces.get("eth0").unwrap();
        assert_eq!(1000, eth1.received);
        assert_eq!(1600, eth1.transmitted);
    }

    #[test]
    fn test_calculate_per_minute_partial_minute() {
        let mut interfaces1 = Interfaces::new();
        interfaces1.insert(
            "eth0".to_string(),
            NetworkTraffic {
                received: 1000,
                transmitted: 1000,
            },
        );
        interfaces1.insert(
            "eth1".to_string(),
            NetworkTraffic {
                received: 2000,
                transmitted: 3000,
            },
        );
        let measurement1 = NetworkTrafficMeasurement {
            precise_time_ns: 60_000_000_000,
            interfaces: interfaces1,
        };

        let mut interfaces2 = Interfaces::new();
        interfaces2.insert(
            "eth0".to_string(),
            NetworkTraffic {
                received: 2000,
                transmitted: 2600,
            },
        );
        interfaces2.insert(
            "eth1".to_string(),
            NetworkTraffic {
                received: 3000,
                transmitted: 4600,
            },
        );
        let measurement2 = NetworkTrafficMeasurement {
            precise_time_ns: 90_000_000_000,
            interfaces: interfaces2,
        };

        let per_minute = measurement1.calculate_per_minute(&measurement2).unwrap();
        assert_eq!(2, per_minute.interfaces.len());

        let eth0 = per_minute.interfaces.get("eth0").unwrap();
        assert_eq!(2000, eth0.received);
        assert_eq!(3200, eth0.transmitted);

        let eth1 = per_minute.interfaces.get("eth0").unwrap();
        assert_eq!(2000, eth1.received);
        assert_eq!(3200, eth1.transmitted);
    }

    #[test]
    fn test_calculate_per_minute_wrong_times() {
        let measurement1 = NetworkTrafficMeasurement {
            precise_time_ns: 90_000_000_000,
            interfaces: Interfaces::new(),
        };

        let measurement2 = NetworkTrafficMeasurement {
            precise_time_ns: 60_000_000_000,
            interfaces: Interfaces::new(),
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::InvalidInput(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_calculate_per_minute_values_lower() {
        let mut interfaces1 = Interfaces::new();
        interfaces1.insert(
            "eth0".to_string(),
            NetworkTraffic {
                received: 2000,
                transmitted: 3000,
            },
        );
        let measurement1 = NetworkTrafficMeasurement {
            precise_time_ns: 60_000_000_000,
            interfaces: interfaces1,
        };

        let mut interfaces2 = Interfaces::new();
        interfaces2.insert(
            "eth0".to_string(),
            NetworkTraffic {
                received: 2000,
                transmitted: 2600,
            },
        );
        let measurement2 = NetworkTrafficMeasurement {
            precise_time_ns: 120_000_000_000,
            interfaces: interfaces2,
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }

    #[test]
    fn test_calculate_per_minute_different_interfaces() {
        let mut interfaces1 = Interfaces::new();
        interfaces1.insert(
            "eth1".to_string(),
            NetworkTraffic {
                received: 2000,
                transmitted: 3000,
            },
        );
        let measurement1 = NetworkTrafficMeasurement {
            precise_time_ns: 60_000_000_000,
            interfaces: interfaces1,
        };

        let mut interfaces2 = Interfaces::new();
        interfaces2.insert(
            "eth0".to_string(),
            NetworkTraffic {
                received: 2000,
                transmitted: 2600,
            },
        );
        let measurement2 = NetworkTrafficMeasurement {
            precise_time_ns: 120_000_000_000,
            interfaces: interfaces2,
        };

        match measurement1.calculate_per_minute(&measurement2) {
            Err(ProbeError::UnexpectedContent(_)) => (),
            r => panic!("Unexpected result: {:?}", r),
        }
    }
}

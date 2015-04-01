use std::io::Command;

pub struct LoadAvgProbe;

impl LoadAvgProbe {
	pub fn probe() -> Option<f64> {
		let result = match Command::new("uptime").output() {
	        Ok(output) => output,
	        Err(e)     => {
	            return None
	        }
	    };

	    // Example of output:
	    // 21:07  up 12 days,  6:53, 3 users, load averages: 1.43 1.45 1.41
	    // We need the first matching regex (number with delimeter [])
	    // 21:07  up 12 days,  6:53, 3 users, load averages: [1.43] 1.45 1.41
	    let output = result.output.as_slice();
	    let text   = String::from_utf8_lossy(output);
	    let re     = regex!(r"([0-9]+[\.,]\d+)");
	    let caps   = match re.captures(text.as_slice()) {
	        Some(output) => output,
	        None         => {
	            return None
	        }
	    };
	    let load = caps.at(1).unwrap();

	    return load.parse::<f64>()
	}
}

extern crate alloc;

use alloc::{collections::BTreeMap, string::String};
use esp_println::println;
use serde::Deserialize;
use serde_json::from_str;

use crate::led_controller::LedController;

#[derive(Deserialize)]
pub struct SensorConfig {
	sensors: BTreeMap<String, String>,
}

const CONFIG_JSON: &str = include_str!("../sensor-config.json");

pub fn get_sensor_name(mac_address: &[u8; 6], led_controller: &mut LedController) -> String {
	// Format MAC address as string
	let mac_str = format_mac_address(mac_address);
	println!("Looking up name for MAC: {}", mac_str);

	// Parse JSON config
	match from_str::<SensorConfig>(CONFIG_JSON) {
		Ok(config) => {
			// Look up sensor name by MAC address
			match config.sensors.get(&mac_str) {
				Some(name) => name.clone(),
				None => {
					println!("ERROR: Unknown device MAC address: {}", mac_str);
					led_controller.set_color("red");
					panic!("Unknown device MAC address");
				}
			}
		}
		Err(e) => {
			println!("Failed to parse config: {:?}", e);
			led_controller.set_color("red");
			panic!("Failed to parse sensor configuration");
		}
	}
}

pub fn format_mac_address(mac_address: &[u8; 6]) -> String {
	alloc::format!(
		"{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
		mac_address[0],
		mac_address[1],
		mac_address[2],
		mac_address[3],
		mac_address[4],
		mac_address[5]
	)
}

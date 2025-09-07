extern crate alloc;

use alloc::{collections::BTreeMap, string::String};
use esp_println::println;
use serde::Deserialize;
use serde_json::from_str;

use crate::led_controller::LedController;

#[derive(Deserialize)]
pub struct SensorConfig {
	pub sensors: BTreeMap<String, SensorInfo>,
}

#[derive(Deserialize, Clone)]
pub struct SensorInfo {
	pub name: String,
	#[serde(rename = "isLedDisabled")]
	pub is_led_disabled: bool,
	#[serde(rename = "intervalInSeconds")]
	pub interval_in_seconds: u64,
}

pub const CONFIG_JSON: &str = include_str!("../sensor-config.json");

pub fn get_sensor_config(mac_address: &[u8; 6], led_controller: &mut LedController) -> SensorInfo {
	// Format MAC address as string
	let mac_str = format_mac_address(mac_address);
	println!("Looking up name for MAC: {}", mac_str);

	// Parse JSON config
	match from_str::<SensorConfig>(CONFIG_JSON) {
		Ok(config) => {
			// Look up sensor info by MAC address
			match config.sensors.get(&mac_str) {
				Some(info) => {
					// If LED is disabled, turn it off
					if info.is_led_disabled {
						led_controller.set_color("off");
					}
					info.clone()
				}
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

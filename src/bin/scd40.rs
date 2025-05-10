// the wifi & embassy integration is largely inspired and adapted from the example in esp-hal, since blocking-network-stack wasn't working with reconnects
// https://github.com/esp-rs/esp-hal/blob/ab73f43d7d05292277502bf23a1261facb03669a/examples/src/bin/wifi_embassy_dhcp.rs

#![no_std]
#![no_main]

extern crate alloc;

use alloc::string::String;
use core::net::Ipv4Addr;
use heapless::String as HString;

use embassy_executor::Spawner;
use embassy_net::{Runner, StackResources, tcp::TcpSocket};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;

use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{
	clock::CpuClock,
	delay::Delay,
	i2c::master::{Config, I2c},
	rng::Rng,
	system,
	timer::timg::TimerGroup,
};
use esp_println::println;
use esp_wifi::{
	EspWifiController, init,
	wifi::{ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent, WifiState},
};
use scd4x::Scd4x;
use serde::Serialize;

// import local modules
use frytg_sensory::{
	led_controller::LedController,
	sensor_config::{format_mac_address, get_sensor_config},
};

#[derive(Serialize)]
struct Measurement {
	serial: u64,      // serial number of the sensor
	mac: String,      // mac address of the chip
	cycle: u32,       // cycle count of the sensor
	co2: u16,         // co2 concentration
	temperature: f32, // temperature
	humidity: f32,    // humidity
}

// overall configuration
const SSID: &str = env!("WIFI_SSID");
const PASSWORD: &str = env!("WIFI_PASSWORD");
const SERVER_IP: &str = env!("SERVER_IP");
const SERVER_PORT: &str = env!("SERVER_PORT");
const ENDPOINT: &str = "/sensor/intake";
const LIGHT_SLEEP_DURATION_MS: u64 = 20_000; // seconds in microseconds
const MAX_MEASUREMENTS: u32 = 1000;

fn parse_ip(ip: &str) -> [u8; 4] {
	let mut result = [0u8; 4];
	for (idx, octet) in ip.split(".").into_iter().enumerate() {
		result[idx] = u8::from_str_radix(octet, 10).unwrap();
	}
	result
}

// this is a macro from https://github.com/esp-rs/esp-hal/blob/main/examples/src/bin/wifi_embassy_dhcp.rs
// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
	($t:ty,$val:expr) => {{
		static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
		#[deny(unused_attributes)]
		let x = STATIC_CELL.uninit().write(($val));
		x
	}};
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
	let config = esp_hal::Config::default().with_cpu_clock(CpuClock::_80MHz);
	let peripherals = esp_hal::init(config);

	// Initialize LED controller with 1 LED
	let mut led_controller = LedController::new(peripherals.GPIO19, peripherals.SPI2, peripherals.GPIO20);

	// Initialize measurement counter
	let mut measurement_count = 0;

	// Initialize heap for WiFi
	esp_alloc::heap_allocator!(size: 72 * 1024);

	// Initialize timer and delay
	let timg0 = TimerGroup::new(peripherals.TIMG0);
	let delay = Delay::new();
	let mut rng = Rng::new(peripherals.RNG);

	// Initialize WiFi
	let esp_wifi_ctrl = &*mk_static!(
		EspWifiController<'static>,
		init(timg0.timer0, rng.clone(), peripherals.RADIO_CLK).unwrap()
	);
	let (controller, interfaces) = esp_wifi::wifi::new(&esp_wifi_ctrl, peripherals.WIFI).unwrap();

	// Init MAC address before moving device
	let mac_address = interfaces.sta.mac_address();
	let sensor_info = get_sensor_config(&mac_address, &mut led_controller);
	println!("Using sensor name: {}", sensor_info.name);

	// set yellow color during startup if LED is not disabled
	if !sensor_info.is_led_disabled {
		led_controller.set_color("yellow");
	} else {
		// blink yellow if LED is disabled
		led_controller.set_color("yellow");
		led_controller.set_color("green");
		led_controller.set_color("off");
	}

	// new timer group
	let timg1 = TimerGroup::new(peripherals.TIMG1);
	esp_hal_embassy::init(timg1.timer0);

	// init dhcp
	let mut dhcp_config = embassy_net::DhcpConfig::default();
	let hostname: HString<32> = HString::try_from(sensor_info.name.as_str()).unwrap();
	dhcp_config.hostname = Some(hostname);
	let config = embassy_net::Config::dhcpv4(dhcp_config);

	// Init network stack
	let seed = (rng.random() as u64) << 32 | rng.random() as u64;
	let (stack, runner) = embassy_net::new(
		interfaces.sta,
		config,
		mk_static!(StackResources<3>, StackResources::<3>::new()),
		seed,
	);

	// spawn tasks (wifi connection and network task)
	spawner.spawn(connection(controller)).ok();
	spawner.spawn(net_task(runner)).ok();

	// wait until we have a link
	println!("Waiting to get link...");
	loop {
		if stack.is_link_up() {
			break;
		}
		Timer::after(Duration::from_millis(500)).await;
	}

	println!("Waiting to get IP address...");
	loop {
		if let Some(config) = stack.config_v4() {
			println!("Got IP: {}", config.address);
			break;
		}
		Timer::after(Duration::from_millis(500)).await;
	}

	// Initialize SCD40 over I2C
	println!("SCD40 sensor initializing...");
	let i2c = I2c::new(peripherals.I2C0, Config::default())
		.expect("Failed to create I2C")
		.with_sda(peripherals.GPIO2)
		.with_scl(peripherals.GPIO1);
	let mut scd40 = Scd4x::new(i2c, delay);
	delay.delay_millis(100u32);

	// stop any previous measurements
	match scd40.stop_periodic_measurement() {
		Ok(_) => println!("SCD40 stopped any previous measurements"),
		Err(e) => println!("SCD40 failed to stop measurements: {:?}", e),
	}

	// read serial number
	let serial_number = match scd40.serial_number() {
		Ok(serial_number) => serial_number,
		Err(e) => {
			println!("SCD40 failed to read serial number: {:?}", e);
			0
		}
	};
	println!("SCD40 serial number: {:?}", serial_number);

	// run self test
	match scd40.self_test_is_ok() {
		Ok(_) => println!("SCD40 self test passed"),
		Err(e) => println!("SCD40 self test failed: {:?}", e),
	}

	// start periodic measurement
	match scd40.start_periodic_measurement() {
		Ok(_) => println!("SCD40 started periodic measurement"),
		Err(e) => println!("SCD40 failed to start measurements: {:?}", e),
	}

	// get socket for data transfer
	let mut rx_buffer = [0u8; 1536];
	let mut tx_buffer = [0u8; 1536];

	// create remote ip
	let parsed_server_ip = parse_ip(SERVER_IP);

	// wait 5sec for first measurement
	println!("SCD40 waiting for first measurement...");
	println!("");
	delay.delay_millis(5000u32);

	// BEGIN MEASUREMENT LOOP
	loop {
		// Check if we've reached 100 measurements, if so reboot
		if measurement_count >= MAX_MEASUREMENTS {
			println!("Reached {} measurements, rebooting...", MAX_MEASUREMENTS);
			// Yellow color to indicate reboot
			if !sensor_info.is_led_disabled {
				led_controller.set_color("yellow");
			}
			system::software_reset();
		}

		// Increment measurement counter
		measurement_count += 1;
		println!("Measurement count: {}/{}", measurement_count, MAX_MEASUREMENTS);

		// set green during measurement if LED is not disabled
		if !sensor_info.is_led_disabled {
			led_controller.set_color("green");
		}

		// check if data is ready
		match scd40.data_ready_status() {
			Ok(ready) => {
				if !ready {
					println!("No data ready");
					if !sensor_info.is_led_disabled {
						led_controller.set_color("yellow");
						led_controller.set_color("red");
					}
					delay.delay_millis(5000u32);
					continue;
				}
			}
			Err(e) => {
				println!("Failed to check data ready status: {:?}", e);
				if !sensor_info.is_led_disabled {
					led_controller.set_color("red");
					led_controller.set_color("yellow");
				}
				delay.delay_millis(5000u32);
				continue;
			}
		}

		// get measurement
		let measurement = match scd40.measurement() {
			Ok(measurement) => measurement,
			Err(err) => {
				if !sensor_info.is_led_disabled {
					led_controller.set_color("red");
				}
				match err {
					scd4x::Error::Crc => println!("Couldn't read sensor: CRC mismatch"),
					scd4x::Error::I2c(_) => println!("Couldn't read sensor: i2c mismatch"),
					scd4x::Error::Internal => println!("Couldn't read sensor: sensirion internal"),
					scd4x::Error::SelfTest => println!("Couldn't read sensor: self-test failure"),
					scd4x::Error::NotAllowed => println!("Couldn't read sensor: not allowed"),
				}
				continue;
			}
		};

		// set blue during data transfer if LED is not disabled
		if !sensor_info.is_led_disabled {
			led_controller.set_color("blue");
		}

		// create socket
		let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
		socket.set_timeout(Some(embassy_time::Duration::from_secs(10)));
		let remote_endpoint = (
			Ipv4Addr::new(
				parsed_server_ip[0],
				parsed_server_ip[1],
				parsed_server_ip[2],
				parsed_server_ip[3],
			),
			SERVER_PORT.parse::<u16>().unwrap(),
		);

		let r = socket.connect(remote_endpoint).await;
		if let Err(e) = r {
			println!("Failed to open socket: {:?}", e);
			if !sensor_info.is_led_disabled {
				led_controller.set_color("red");
			}
			delay.delay_millis(1000u32);
			continue;
		}
		println!("Socket connected");

		// turn measurements into json
		let measurement = Measurement {
			serial: serial_number,
			mac: format_mac_address(&mac_address),
			cycle: measurement_count,
			co2: measurement.co2,
			temperature: measurement.temperature,
			humidity: measurement.humidity,
			// rssi: controller.rssi(),
		};
		let json_data = serde_json::to_string(&measurement).unwrap();
		println!("json_data: {}", json_data);

		// prepare json data request
		let full_endpoint = alloc::format!("{}/{}", ENDPOINT, sensor_info.name);
		let http_request = String::from_utf8_lossy(
			alloc::format!(
				"POST {} HTTP/1.1\r\n\
                    Host: {}\r\n\
                    Content-Type: application/json\r\n\
                    Content-Length: {}\r\n\
                    \r\n\
                    {}",
				full_endpoint,
				SERVER_IP,
				json_data.len(),
				json_data
			)
			.as_bytes(),
		)
		.into_owned();

		// send json data to server
		let mut buf = [0; 1024];
		let r = socket.write_all(http_request.as_bytes()).await;
		if let Err(e) = r {
			println!("write error: {:?}", e);
			continue;
		}
		let n = match socket.read(&mut buf).await {
			Ok(0) => 0,
			Ok(n) => n,
			Err(e) => {
				println!("read error: {:?}", e);
				continue;
			}
		};
		println!("{}", core::str::from_utf8(&buf[..n]).unwrap());

		// turn off led if not disabled
		if !sensor_info.is_led_disabled {
			led_controller.set_color("off");
		}
		println!("Entering light sleep");
		println!("");

		// sleep
		delay.delay_millis(LIGHT_SLEEP_DURATION_MS as u32);
	}
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
	println!("spawning connection task");
	loop {
		match esp_wifi::wifi::wifi_state() {
			WifiState::StaConnected => {
				// wait until we're no longer connected
				controller.wait_for_event(WifiEvent::StaDisconnected).await;
				Timer::after(Duration::from_millis(5000)).await
			}
			_ => {}
		}
		if !matches!(controller.is_started(), Ok(true)) {
			println!("Wifi not started, starting...");
			let client_config = Configuration::Client(ClientConfiguration {
				ssid: SSID.try_into().unwrap(),
				password: PASSWORD.try_into().unwrap(),
				..Default::default()
			});
			controller.set_configuration(&client_config).unwrap();
			println!("Wifi starting...");
			controller.start_async().await.unwrap();
			println!("Wifi started!");
		}
		println!("Wifi connecting...");

		match controller.connect_async().await {
			Ok(_) => println!("Wifi connected!"),
			Err(e) => {
				println!("Wifi connection failed: {e:?}");
				Timer::after(Duration::from_millis(5000)).await
			}
		}
	}
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
	println!("spawning net task");
	runner.run().await
}

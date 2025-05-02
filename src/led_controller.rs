use esp_hal::{
	Blocking,
	delay::Delay,
	gpio::{GpioPin, Level, Output, OutputConfig},
	peripherals::SPI2,
	spi::{
		Mode,
		master::{Config, Spi},
	},
	time::Rate,
};
use esp_println::println;
use smart_leds::{RGB8, SmartLedsWrite};
use ws2812_spi as ws2812;

pub struct LedController<'a> {
	ws: ws2812::Ws2812<Spi<'a, Blocking>>,
	data: [RGB8; 1],
}

impl LedController<'_> {
	pub fn new(power_pin: GpioPin<19>, spi: SPI2, mosi: GpioPin<20>) -> Self {
		println!("Initializing LED controller");

		// Set GPIO19 to high power
		let mut power_pin = Output::new(power_pin, Level::High, OutputConfig::default());
		power_pin.set_high();
		println!("GPIO19 set to high power");

		// Configure SPI for WS2812
		// Using SPI2 with GPIO20 as MOSI (data out)
		let spi = Spi::new(
			spi,
			Config::default().with_mode(Mode::_0).with_frequency(Rate::from_mhz(3)),
		)
		.unwrap()
		.with_mosi(mosi);

		println!("SPI initialized");

		// Initialize WS2812 driver
		let ws = ws2812::Ws2812::new(spi);
		println!("WS2812 driver initialized");

		// Initialize LED data buffer
		const NUM_LEDS: usize = 1;
		let data: [RGB8; NUM_LEDS] = [RGB8::default(); NUM_LEDS];

		LedController { ws, data }
	}

	pub fn set_to_color(&mut self, color: RGB8) {
		self.data = [color; 1];
		self.update_leds();
	}

	pub fn set_color(&mut self, color: &str) {
		match color {
			"yellow" => self.set_to_color(RGB8 { r: 150, g: 50, b: 5 }),
			"red" => self.set_to_color(RGB8 { r: 150, g: 0, b: 0 }),
			"green" => self.set_to_color(RGB8 { r: 0, g: 60, b: 0 }),
			"blue" => self.set_to_color(RGB8 { r: 0, g: 0, b: 30 }),
			"off" => self.set_to_color(RGB8 { r: 0, g: 0, b: 0 }),
			_ => self.set_to_color(RGB8 { r: 0, g: 0, b: 0 }),
		}
		let delay2 = Delay::new();
		delay2.delay_millis(1000u32);
	}

	pub fn update_leds(&mut self) {
		match self.ws.write(self.data.iter().cloned()) {
			Ok(_) => {
				// println!("LED update successful: {:?}", self.data),
			}
			Err(e) => {
				println!("LED update error: {:?}", e);
			}
		}
	}
}

# Sensory Firmware

This is the firmware for an ESP32 microcontroller written in Embedded Rust. It connects to sensor(s) and sends data to a local server.

For the most part this was a learning exercise for me to get more familiar with Embedded Rust and the ESP32 microcontroller. I have several sensors using this setup running at home.

## ESP32

This is currently setup to run on the ESP32-C6 microcontroller. I am using the [NanoC6 Dev Kit](https://shop.m5stack.com/products/m5stack-nanoc6-dev-kit) from M5Stack, which is useful dev board with a tiny footprint, LED, USB-C, and a few other features.

The primary sensor is the [SCD40](https://sensirion.com/products/catalog/SCD40) from Sensirion, which also has a useful [M5Stack board](https://shop.m5stack.com/products/co2-unit-with-temperature-and-humidity-sensor-scd40) that can be connected to the ESP32-C6 via the M5Stack grove/ JST connector. It communicates over I2C.

## Running the firmware

You will need to have the [`just`](https://github.com/casey/just) and [`sops`](https://github.com/getsops/sops) commands installed. It will use `.env.sops.yaml` or whatever is configured in the `SOPS_ENV_FILE` environment variable.

Run directly on the ESP32 microcontroller:

```bash
just run scd40
```

Build for release:

```bash
just release scd40
```

Flash to the ESP32 microcontroller:

```bash
just flash scd40
```

## Kudos

This project uses snippets and was inspired by projects and their examples like [`esp-rs/esp-hal`](https://github.com/esp-rs/esp-hal).

## Server Endpoint

The server endpoint is a simple HTTP server that accepts POST requests with JSON data.

```bash
curl -X POST http://localhost:8081/sensor/intake \
  -H "Content-Type: application/json" \
  -d '{"serial":111111111111111,"mac":"11:11:11:11:11:11","cycle":1,"co2":555,"temperature":22.48188,"humidity":48.8426}'
```

```json
{
  "serial": 111111111111111,
  "mac": "11:11:11:11:11:11",
  "cycle": 1,
  "co2": 555,
  "temperature": 22.48188,
  "humidity": 48.8426
}
```

## Docs

- `embassy`: [embassy.dev](https://embassy.dev) / [embassy.dev/book](https://embassy.dev/book/)
- `esp-rs` docs overview [docs.espressif.com/projects/rust/](https://docs.espressif.com/projects/rust/)
- `esp-hal` [docs.espressif.com/projects/rust/esp-hal/1.0.0-beta.0](https://docs.espressif.com/projects/rust/esp-hal/1.0.0-beta.0/index.html)
  - ESP32-C6: [docs.espressif.com/projects/rust/esp-hal/1.0.0-beta.0/esp32c6/esp_hal](https://docs.espressif.com/projects/rust/esp-hal/1.0.0-beta.0/esp32c6/esp_hal/index.html)
- `esp-wifi` [docs.espressif.com/projects/rust/esp-wifi/0.13.0](https://docs.espressif.com/projects/rust/esp-wifi/0.13.0/index.html)
  - ESP32-C6: [docs.espressif.com/projects/rust/esp-wifi/0.13.0/esp32c6/esp_wifi](https://docs.espressif.com/projects/rust/esp-wifi/0.13.0/esp32c6/esp_wifi/index.html)
- `scd4x` [docs.rs/scd4x/0.4.0/scd4x/](https://docs.rs/scd4x/0.4.0/scd4x/) / [github.com/hauju/scd4x-rs](https://github.com/hauju/scd4x-rs)
- `smoltcp` [docs.rs/smoltcp/latest/smoltcp/index.html](https://docs.rs/smoltcp/latest/smoltcp/index.html)
- M5Stack NanoC6: [docs.m5stack.com/en/core/M5NanoC6](https://docs.m5stack.com/en/core/M5NanoC6)

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

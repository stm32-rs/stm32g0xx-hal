# `stm32g0xx-hal`

_stm32g0xx-hal_ contains a multi device hardware abstraction on top of the
peripheral access API for the STMicro STM32G0 series microcontrollers. The
selection of the MCU is done by feature gates, typically specified by board
support crates.

## G0 hardware overview

Feature | Desc | G030 | G070 | G031 | G041 | G071 | G081
-- | -- | -- | -- | -- | -- | -- | --
TIM1 | 16bit up/down | 1 | 1 | 1 | 1 | 1 | 1
TIM2 | 32bit up/down |   |   | 1 | 1 | 1 | 1
TIM3 | 16bit up/down | 1 | 1 | 1 | 1 | 1 | 1
TIM6 | 16bit up |   | 1 |   |   | 1 | 1
TIM7 | 16bit up |   | 1 |   |   | 1 | 1
TIM14 | 16bit up | 1 | 1 | 1 | 1 | 1 | 1
TIM15 | 16bit up |   | 1 |   |   | 1 | 1
TIM16 | 16bit up | 1 | 1 | 1 | 1 | 1 | 1
TIM17 | 16bit up | 1 | 1 | 1 | 1 | 1 | 1
LPTIM1 | 16bit up |   |   | 1 | 1 | 1 | 1
LPTIM2 | 16bit up |   |   | 1 | 1 | 1 | 1
COMP1 | Comparator |   |   |   |   | 1 | 1
COMP2 | Comparator |   |   |   |   | 1 | 1
DAC |   |   |   |   |   | 1 | 1
UART1 |   | 1 | 1 | 1 | 1 | 1 | 1
UART2 |   | 1* | 1 | 1* | 1* | 1 | 1
UART3 |   |   | 1 |   |   | 1 | 1
UART4 |   |   | 1 |   |   | 1 | 1
LPUART |   |   |   | 1 | 1 | 1 | 1
AES |   |   |   |   |  1  |   | 1
RNG |   |   |   |   |  1  |   | 1
UCPD1 | USB C Power Delivery |   |   |   |   | 1 | 1
UCPD2 | USB C Power Delivery |   |   |   |   | 1 | 1
VREFBUF |   |   |   | 1 | 1 | 1 | 1
TS_CAL2 | Tsense calibration val @ 130 C |   |   | 1 | 1 | 1 | 1
DMA Channels |  | 5 | 7 | 5 | 5 | 7 | 7
CEC | HDMI control |   |   |   |   | 1 | 1

## Usage

This crate will eventually contain support for multiple microcontrollers in the
stm32g0 family. Which specific microcontroller you want to build for has to be
specified with a feature, for example `stm32g070`.

### Building an Example

If you are compiling the crate on its own for development or running examples,
specify your microcontroller on the command line. For example:

```
cargo build --example blinky --features stm32g071
```

### Using as a Dependency

When using this crate as a dependency in your project, the microcontroller can
be specified as part of the `Cargo.toml` definition.

```
[dependencies.stm32g0xx-hal]
version = "0.1.1"
features = ["rt", "stm32g081"]
```

## Documentation

The documentation can be found at [docs.rs](https://docs.rs/stm32g0xx-hal/).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

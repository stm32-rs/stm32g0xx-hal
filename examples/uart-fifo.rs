// #![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use core::fmt::Write;

use hal::prelude::*;
use hal::serial::*;
use hal::stm32;
use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();
    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut usart1 = dp
        .USART1
        .usart(
            gpioa.pa9,
            gpioa.pa10,
            FullConfig::default()
                .baudrate(115200.bps())
                .fifo_enable()
                .rx_fifo_enable_interrupt()
                .rx_fifo_threshold(FifoThreshold::FIFO_4_BYTES),
            &mut rcc,
        )
        .unwrap();

    writeln!(usart1, "Hello USART1\r\n").unwrap();

    let (mut tx1, mut rx1) = usart1.split();

    let mut cnt = 0;
    loop {
        if rx1.fifo_threshold_reached() {
            loop {
                match rx1.read() {
                    Err(nb::Error::WouldBlock) => {
                        // no more data available in fifo
                        break;
                    }
                    Err(nb::Error::Other(_err)) => {
                        // Handle other error Overrun, Framing, Noise or Parity
                    }
                    Ok(byte) => {
                        writeln!(tx1, "{}: {}\n", cnt, byte).unwrap();
                        cnt += 1;
                    }
                }
            }
        }
    }
}

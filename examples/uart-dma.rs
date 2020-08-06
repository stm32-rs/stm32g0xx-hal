#![no_main]
#![no_std]

extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use core::fmt::Write;

use hal::prelude::*;
use hal::serial::*;
use hal::stm32;
use hal::dma::{self, Channel, Target};

use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();
    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut usart1 = dp.USART1.usart(
        gpioa.pa9,
        gpioa.pa10,
        FullConfig::default().baudrate(115200.bps()), &mut rcc).unwrap();

    writeln!(usart1, "Hello without DMA\r\n").unwrap();

    let tx_buffer: [u8; 16] = *b"Hello with DMA!\n";

    let (mut tx, _rx) = usart1.split();

    let mut dma = dp.DMA.split(&mut rcc, dp.DMAMUX);

    let usart = unsafe { &(*stm32::USART1::ptr()) };
    let tx_data_register_addr = &usart.tdr as *const _ as u32;
    let tx_dma_buf_addr : u32 = tx_buffer.as_ptr() as u32;

    dma.ch1.set_direction(dma::Direction::FromMemory);
    dma.ch1.set_memory_address(tx_dma_buf_addr, true);
    dma.ch1.set_peripheral_address(tx_data_register_addr, false);
    dma.ch1.set_transfer_length(tx_buffer.len() as u16);

    dma.ch1.select_peripheral(tx.dmamux());

    tx.enable_dma();
    dma.ch1.enable();

    loop {
        continue;
    }
}

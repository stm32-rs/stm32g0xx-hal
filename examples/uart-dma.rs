#![no_main]
#![no_std]

extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate stm32g0xx_hal as hal;

use core::fmt::Write;

use hal::dma::{self, Channel, Event, Target};
use hal::prelude::*;
use hal::serial::*;
use hal::stm32;

use rt::entry;

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);

    let mut led = gpioa.pa5.into_push_pull_output();

    let mut usart1 = dp
        .USART1
        .usart(
            gpioa.pa9,
            gpioa.pa10,
            FullConfig::default().baudrate(115200.bps()),
            &mut rcc,
        )
        .unwrap();

    writeln!(usart1, "Hello without DMA\r\n").unwrap();

    let tx_buffer: [u8; 16] = *b"Hello with DMA!\n";

    let (mut tx, _rx) = usart1.split();

    let mut dma = dp.DMA.split(&mut rcc, dp.DMAMUX);

    // Setup DMA for USART1 TX with dma channel 1.
    let usart = unsafe { &(*stm32::USART1::ptr()) };
    let tx_data_register_addr = &usart.tdr as *const _ as u32;
    let tx_dma_buf_addr: u32 = tx_buffer.as_ptr() as u32;

    dma.ch1.set_direction(dma::Direction::FromMemory);
    dma.ch1.set_memory_address(tx_dma_buf_addr, true);
    dma.ch1.set_peripheral_address(tx_data_register_addr, false);
    dma.ch1.set_transfer_length(tx_buffer.len() as u16);

    // Configure dmamux for dma ch1 and usart tx
    dma.ch1.select_peripheral(tx.dmamux());

    dma.ch1.listen(Event::TransferComplete);

    tx.enable_dma();
    dma.ch1.enable();

    // Create a second buffer to send after the first dma transfer has completed
    let mut tx_buffer2: [u8; 23] = *b"Transfer complete {0}!\n";
    let tx_dma_buf_addr: u32 = tx_buffer2.as_ptr() as u32;

    let mut delay = dp.TIM1.delay(&mut rcc);

    loop {
        if dma.ch1.event_occurred(Event::TransferComplete) {
            dma.ch1.clear_event(Event::TransferComplete);

            // update the char between '{ }' in tx_buffer2
            tx_buffer2[19] += 1;

            // wrap around to ascii value 33 == '!', so that we only use printable characters.
            if tx_buffer2[19] > 126 {
                tx_buffer2[19] = 33;
            }

            dma.ch1.disable();

            led.toggle().unwrap();

            dma.ch1.set_memory_address(tx_dma_buf_addr, true);
            dma.ch1.set_transfer_length(tx_buffer2.len() as u16);
            dma.ch1.enable();
        }

        delay.delay(500.ms());
    }
}

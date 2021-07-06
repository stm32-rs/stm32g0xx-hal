// has been tested on nucleo-32 with STM32G031
// command build: cargo build --example adc_ext_trig_double_dma_serial --features stm32g031
// command run: cargo run --example adc_ext_trig_double_dma_serial --features stm32g031

#![deny(warnings)]
#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
use cortex_m_semihosting::hprintln;

extern crate nb;
extern crate panic_halt;
extern crate stm32g0;
extern crate stm32g0xx_hal as hal;

use hal::analog::adc;
use hal::prelude::*;
use hal::serial::*;
use hal::stm32;
use rt::entry;

use core::cell::RefCell;
use cortex_m::interrupt::Mutex;

use crate::hal::stm32::{interrupt, Interrupt};
use hal::analog::adc::{InjTrigSource, Precision, SampleTime}; //, VTemp

use hal::dma::{self, Channel, Target};

use crate::hal::analog::adc::DmaMode;
use crate::hal::analog::adc::InjectMode;

// Make dma globally available
static G_DMA: Mutex<RefCell<Option<hal::dma::Channels>>> = Mutex::new(RefCell::new(None));

const BUFFER_SIZE: u16 = 4;
// Make the buffer pointer globally available
static G_DMA_BUFFER_ADDR: Mutex<RefCell<Option<u32>>> = Mutex::new(RefCell::new(None));

#[interrupt]
unsafe fn DMA_CHANNEL1() {
    static mut DMA: Option<hal::dma::Channels> = None;
    static mut DMA_BUFFER_ADDR: Option<u32> = None;

    let dma_ch = DMA.get_or_insert_with(|| {
        cortex_m::interrupt::free(|cs| {
            // Move dma here, leaving a None in its place
            G_DMA.borrow(cs).replace(None).unwrap()
        })
    });

    let dma_buf_addr = DMA_BUFFER_ADDR.get_or_insert_with(|| {
        cortex_m::interrupt::free(|cs| {
            // Move dma buffer pointer here, leaving a None in its place
            G_DMA_BUFFER_ADDR.borrow(cs).replace(None).unwrap()
        })
    });

    let tx_dma_buf_first_addr: u32 = *dma_buf_addr;
    let tx_dma_buf_second_addr: u32 = *dma_buf_addr + (BUFFER_SIZE) as u32;
    // Address is in byte, value in 2Bytes, this is why second dma buffer ist added with BUFFER_SIZE
    // and not BUFFER_SIZE/2

    let dma = &(*stm32g0::stm32g031::DMA::ptr());
    let htif1 = dma.isr.read().htif1().bit();
    let tcif1 = dma.isr.read().tcif1().bit();
    // set the global clear bit of DMA channel1
    dma.ifcr.write(|w| w.cgif1().set_bit());

    dma_ch.ch2.disable();
    dma_ch.ch2.set_transfer_length(BUFFER_SIZE as u16);
    if htif1 == true {
        dma_ch.ch2.set_memory_address(tx_dma_buf_first_addr, true);
        dma_ch.ch2.enable();
        // hprintln!("DMA_CHANNEL1 half transfer compleated {:?} {:?}", htif1, tx_dma_buf_first_addr).unwrap();
    } else if tcif1 == true {
        dma_ch.ch2.set_memory_address(tx_dma_buf_second_addr, true);
        dma_ch.ch2.enable();
        // hprintln!("DMA_CHANNEL1 transfer compleated {:?}  {:?}", tcif1, tx_dma_buf_second_addr).unwrap();
    }
}

#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");
    let mut rcc = dp.RCC.constrain();

    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut timer = dp.TIM2.timer(&mut rcc);

    let usart1 = dp
        .USART1
        .usart(
            gpioa.pa9,                                                  // TX: pa9, => CN3 Pin-D5
            gpioa.pa10,                                                 // RX: pa10, => CN3 Pin-D4
            FullConfig::default().baudrate(460800.bps()).fifo_enable(), // enable fifo, so that dma can fill it fast, otherwise it may not finish before ch1 is requested again
            &mut rcc,
        )
        .unwrap();

    // DMA example
    //==================================================
    let adc_buffer1: [u16; BUFFER_SIZE as usize] = [0; BUFFER_SIZE as usize];

    let mut dma = dp.DMA.split(&mut rcc, dp.DMAMUX);

    let adc_ptr = unsafe { &(*stm32g0::stm32g031::ADC::ptr()) };
    let adc_data_register_addr = &adc_ptr.dr as *const _ as u32;

    let adc_buffer1_addr: u32 = adc_buffer1.as_ptr() as u32;

    dma.ch1.set_word_size(dma::WordSize::BITS16);
    dma.ch1.set_direction(dma::Direction::FromPeripheral);
    dma.ch1.set_memory_address(adc_buffer1_addr, true);
    dma.ch1
        .set_peripheral_address(adc_data_register_addr, false);
    dma.ch1.set_transfer_length(adc_buffer1.len() as u16);

    hprintln!("adc_data_register_addr {:?}", adc_buffer1_addr).unwrap(); // will output addr in dec
                                                                         // in gdb read the data bytes with:  x /32xh 0x???????   (last is addr in hex)
                                                                         // or put addr in dec format:   x /32xh 536878092
                                                                         // https://sourceware.org/gdb/current/onlinedocs/gdb/Memory.html

    // dma ch1 reads from ADC register into memory
    dma.ch1
        .select_peripheral(stm32g0xx_hal::dmamux::DmaMuxIndex::ADC);
    // The dma continuesly fills the buffer, when its full, it starts over again
    dma.ch1.set_circular_mode(true);

    // Enabel dma irq for half and full buffer, when reached, so that the second dma ch2 can be started
    dma.ch1.listen(hal::dma::Event::HalfTransfer);
    dma.ch1.listen(hal::dma::Event::TransferComplete);

    let (mut tx, mut _rx) = usart1.split();

    let usart = unsafe { &(*stm32::USART1::ptr()) };
    let tx_data_register_addr = &usart.tdr as *const _ as u32;

    dma.ch2.set_direction(dma::Direction::FromMemory);
    dma.ch2.set_peripheral_address(tx_data_register_addr, false);
    dma.ch2.set_word_size(hal::dma::WordSize::BITS8);

    dma.ch2.select_peripheral(tx.dmamux());

    tx.enable_dma();

    // start dma transfer
    dma.ch1.enable();

    cortex_m::interrupt::free(|cs| *G_DMA.borrow(cs).borrow_mut() = Some(dma));
    cortex_m::interrupt::free(|cs| {
        *G_DMA_BUFFER_ADDR.borrow(cs).borrow_mut() = Some(adc_buffer1_addr)
    });

    //==================================================
    // Set up adc

    let mut adc = dp.ADC.constrain(&mut rcc);
    adc.set_sample_time(SampleTime::T_80);
    adc.set_precision(Precision::B_12);
    let mut pa3 = gpioa.pa5.into_analog();
    let u_raw: u32 = adc.read(&mut pa3).expect("adc read failed");
    let u = u_raw.saturating_sub(32) as f32 / 4_096_f32 * 3.3;
    hprintln!("u: {:.4} V ", u).unwrap();

    adc.set_oversampling_ratio(adc::OversamplingRatio::X_16);
    adc.set_oversampling_shift(4);
    adc.oversampling_enable(true);
    adc.prepare_injected(&mut pa3, InjTrigSource::TRG_2);
    adc.start_injected();

    // Enable timer to trigger external sources in mms value of cr2
    // 011: Compare Pulse - The trigger output send a positive pulse when the CC1IF flag is to be
    // set (even if it was already high), as soon as a capture or a compare match occurred.
    // ouput is (TRGO)
    // according to reference manual chapter 22.4.2
    // this is only available on timer TIM2, TIM3, TIM4 and TIM1
    unsafe {
        // get pointer of timer 2
        let tim = &(*stm32g0::stm32g031::TIM2::ptr());
        //
        tim.cr2.modify(|_, w| w.mms().bits(3 as u8));
    }

    // enable dma to be called, when adc is ready to read
    adc.dma_enable(true);
    adc.dma_circualr_mode(true);

    // don't enabel the timer bevor the dma
    // Set up a timer expiring after
    timer.start(50.us());
    timer.listen();

    //enable DMA_CHANNEL1 interrupt
    unsafe {
        cortex_m::peripheral::NVIC::unmask(Interrupt::DMA_CHANNEL1);
    }

    loop {}
}

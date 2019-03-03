#![no_std]
#![no_main]
#![allow(non_snake_case)]
#![deny(warnings)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting;
extern crate panic_semihosting;
extern crate rtfm;
#[macro_use]
extern crate stm32g0xx_hal as hal;

use hal::exti::Event;
use hal::gpio::gpioa::PA5;
use hal::gpio::{Output, PushPull, SignalEdge};
use hal::prelude::*;
use hal::stm32;
use hal::timer::Timer;
use rtfm::app;

#[app(device = hal::stm32)]
const APP: () = {
    static mut EXTI: stm32::EXTI = ();
    static mut TIMER: Timer<stm32::TIM17> = ();
    static mut LED: PA5<Output<PushPull>> = ();

    #[init]
    fn init() {
        hal::debug::init();

        let mut rcc = device.RCC.constrain();
        let gpioa = device.GPIOA.split(&mut rcc);
        let gpioc = device.GPIOC.split(&mut rcc);

        let mut timer = device.TIM17.timer(&mut rcc);
        timer.start(3.hz());
        timer.listen();

        gpioc.pc13.listen(SignalEdge::Falling, &mut device.EXTI);

        LED = gpioa.pa5.into_push_pull_output();
        TIMER = timer;
        EXTI = device.EXTI;
    }

    #[interrupt(binds = TIM17, resources = [TIMER, LED])]
    fn on_timer_tick() {
        resources.LED.toggle();
        resources.TIMER.clear_irq();
    }

    #[interrupt(binds = EXTI4_15, resources = [EXTI])]
    fn on_button_click() {
        println!("{}", resources.EXTI.is_pending(Event::GPIO13, SignalEdge::Falling));
        resources.EXTI.unpend(Event::GPIO13);
    }
};

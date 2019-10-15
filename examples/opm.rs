#![no_std]
#![no_main]
#![deny(warnings)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate rtfm;
extern crate stm32g0xx_hal as hal;

use hal::exti::Event;
use hal::gpio::gpioa::PA5;
use hal::gpio::{Output, PushPull, SignalEdge};
use hal::prelude::*;
use hal::rcc::{Config, SysClockSrc};
use hal::stm32;
use hal::opm::{Opm, C1};
use rtfm::app;

#[app(device = hal::stm32)]
const APP: () = {
    static mut EXTI: stm32::EXTI = ();
    static mut OPM: Opm<stm32::TIM14, C1> = ();
    static mut LED: PA5<Output<PushPull>> = ();

    #[init]
    fn init() {
        let mut rcc = device.RCC.freeze(Config::new(SysClockSrc::PLL));

        let gpioa = device.GPIOA.split(&mut rcc);
        let mut opm = device.TIM14.opm(gpioa.pa4, 50.ms(), &mut rcc);
        opm.enable();

        let gpioc = device.GPIOC.split(&mut rcc);
        gpioc.pc13.listen(SignalEdge::Falling, &mut device.EXTI);

        OPM = opm;
        LED = gpioa.pa5.into_push_pull_output();
        EXTI = device.EXTI;
    }

    #[interrupt(binds = EXTI4_15, resources = [EXTI, LED, OPM])]
    fn on_button_click() {
        resources.EXTI.unpend(Event::GPIO13);
        resources.LED.toggle().unwrap();
        resources.OPM.generate();
    }
};

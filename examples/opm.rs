#![no_std]
#![no_main]
#![deny(warnings)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

use hal::exti::Event;
use hal::gpio::gpioa::PA5;
use hal::gpio::{Output, PushPull, SignalEdge};
use hal::prelude::*;
use hal::rcc;
use hal::stm32;
use hal::timer::opm::Opm;
use rtic::app;

#[app(device = hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        exti: stm32::EXTI,
        led: PA5<Output<PushPull>>,
        opm: Opm<stm32::TIM3>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let mut rcc = ctx.device.RCC.freeze(rcc::Config::pll());
        let mut exti = ctx.device.EXTI;

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpiob = ctx.device.GPIOB.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        let led = gpioa.pa5.into_push_pull_output();
        gpioc.pc13.listen(SignalEdge::Falling, &mut exti);

        let opm = ctx.device.TIM3.opm(4.ms(), &mut rcc);

        let mut opm_ch1 = opm.bind_pin(gpioa.pa6);
        let mut opm_ch2 = opm.bind_pin(gpioa.pa7);
        let mut opm_ch3 = opm.bind_pin(gpiob.pb0);
        let mut opm_ch4 = opm.bind_pin(gpiob.pb1);

        let max_delay = opm_ch2.get_max_delay();

        opm_ch2.set_delay(max_delay / 2);
        opm_ch3.set_delay(max_delay / 4);
        opm_ch4.set_delay(max_delay / 8);

        opm_ch1.enable();
        opm_ch2.enable();
        opm_ch3.enable();
        opm_ch4.enable();

        init::LateResources { opm, exti, led }
    }

    #[task(binds = EXTI4_15, resources = [exti, led, opm])]
    fn button_click(ctx: button_click::Context) {
        ctx.resources.led.toggle().unwrap();
        ctx.resources.opm.generate();
        ctx.resources.exti.unpend(Event::GPIO13);
    }
};

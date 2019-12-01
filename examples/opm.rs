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
use hal::rcc;
use hal::stm32;
use hal::timer::opm::{Channel1, Opm};
use rtfm::app;

#[app(device = hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        exti: stm32::EXTI,
        led: PA5<Output<PushPull>>,
        opm: Opm<stm32::TIM14, Channel1>,
    }

    #[init]
     fn init(ctx: init::Context) -> init::LateResources {
        let mut rcc = ctx.device.RCC.freeze(rcc::Config::pll());
        let mut exti = ctx.device.EXTI;

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        let led = gpioa.pa5.into_push_pull_output();
        gpioc.pc13.listen(SignalEdge::Falling, &mut exti);

        let mut opm = ctx.device.TIM14.opm(gpioa.pa4, 5.ms(), &mut rcc);
        opm.enable();

        init::LateResources {
            opm,
            exti,
            led,
        }
    }

    #[task(binds = EXTI4_15, resources = [exti, led, opm])]
    fn button_click(ctx: button_click::Context) {
        ctx.resources.led.toggle().unwrap();
        ctx.resources.opm.generate();
        ctx.resources.exti.unpend(Event::GPIO13);
    }
};

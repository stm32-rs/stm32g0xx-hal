#![no_std]
#![no_main]
#![deny(warnings)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

use hal::exti::Event;
use hal::gpio::gpioa::PA5;
use hal::gpio::{Output, PushPull, SignalEdge};
use hal::power::{LowPowerMode, PowerMode};
use hal::prelude::*;
use hal::rcc::{self, Prescaler};
use hal::stm32;
use rtic::app;

#[app(device = hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        exti: stm32::EXTI,
        led: PA5<Output<PushPull>>,
    }

    #[init]
    fn init(mut ctx: init::Context) -> init::LateResources {
        let mut rcc = ctx.device.RCC.freeze(rcc::Config::hsi(Prescaler::Div16));
        let mut exti = ctx.device.EXTI;

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        let led = gpioa.pa5.into_push_pull_output();
        let button = gpioc.pc13.listen(SignalEdge::Falling, &mut exti);

        let mut power = ctx.device.PWR.constrain(&mut rcc);
        power.set_mode(PowerMode::LowPower(LowPowerMode::StopMode2));

        if button.is_high().unwrap() {
            ctx.core.SCB.set_sleepdeep();
        }

        init::LateResources { exti, led }
    }

    #[task(binds = EXTI4_15, resources = [exti, led])]
    fn button_click(ctx: button_click::Context) {
        ctx.resources.led.toggle().unwrap();
        ctx.resources.exti.unpend(Event::GPIO13);
    }
};

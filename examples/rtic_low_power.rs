#![no_std]
#![no_main]
#![deny(warnings)]

extern crate panic_probe;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

use defmt_rtt as _;
use hal::exti::Event;
use hal::gpio::*;
use hal::power::{LowPowerMode, PowerMode};
use hal::prelude::*;
use hal::rcc::{self, Prescaler};
use hal::stm32;

#[rtic::app(device = hal::stm32, peripherals = true)]
mod app {
    use super::*;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        exti: stm32::EXTI,
        led: PA15<Output<PushPull>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let mut rcc = ctx.device.RCC.freeze(rcc::Config::hsi(Prescaler::Div16));
        let mut exti = ctx.device.EXTI;

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let led = gpioa.pa15.into_push_pull_output();
        let mut button = gpioa.pa2.into_pull_up_input().listen(SignalEdge::Falling, &mut exti);

        let mut power = ctx.device.PWR.constrain(&mut rcc);
        power.set_mode(PowerMode::UltraLowPower(LowPowerMode::StopMode2));

        if button.is_high().unwrap() {
            let mut scb = ctx.core.SCB;
            scb.set_sleepdeep();
        }

        (Shared {}, Local { exti, led })
    }

    #[task(binds = EXTI2, local = [exti, led])]
    fn button_click(ctx: button_click::Context) {
        ctx.local.led.toggle().unwrap();
        ctx.local.exti.unpend(Event::GPIO2);
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}

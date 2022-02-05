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

#[rtic::app(device = hal::stm32, peripherals = true)]
mod app {
    use super::*;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        exti: stm32::EXTI,
        led: PA5<Output<PushPull>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut rcc = ctx.device.RCC.freeze(rcc::Config::hsi(Prescaler::Div16));
        let mut exti = ctx.device.EXTI;

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        let led = gpioa.pa5.into_push_pull_output();
        let button = gpioc.pc13.listen(SignalEdge::Falling, &mut exti);

        let mut power = ctx.device.PWR.constrain(&mut rcc);
        power.set_mode(PowerMode::UltraLowPower(LowPowerMode::StopMode2));

        if button.is_high().unwrap() {
            let mut scb = ctx.core.SCB;
            scb.set_sleepdeep();
        }

        (Shared {}, Local { exti, led }, init::Monotonics())
    }

    #[task(binds = EXTI4_15, local = [exti, led])]
    fn button_click(ctx: button_click::Context) {
        ctx.local.led.toggle().unwrap();
        ctx.local.exti.unpend(Event::GPIO13);
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}

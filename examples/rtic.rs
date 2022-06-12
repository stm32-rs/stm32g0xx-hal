#![no_std]
#![no_main]
#![deny(warnings)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

use cortex_m_semihosting::hprintln;
use hal::exti::Event;
use hal::gpio::gpioa::PA5;
use hal::gpio::{Output, PushPull, SignalEdge};
use hal::prelude::*;
use hal::rtc::Rtc;
use hal::stm32;
use hal::time::*;
use hal::timer::Timer;

#[rtic::app(device = hal::stm32, peripherals = true)]
mod app {
    use super::*;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        exti: stm32::EXTI,
        timer: Timer<stm32::TIM17>,
        led: PA5<Output<PushPull>>,
        rtc: Rtc,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut rcc = ctx.device.RCC.constrain();
        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        let mut timer = ctx.device.TIM17.timer(&mut rcc);
        timer.start(3.hz());
        timer.listen();

        let mut exti = ctx.device.EXTI;
        gpioc.pc13.listen(SignalEdge::Falling, &mut exti);

        let mut rtc = ctx.device.RTC.constrain(&mut rcc);
        rtc.set_date(&Date::new(2019.year(), 11.month(), 24.day()));
        rtc.set_time(&Time::new(21.hours(), 15.minutes(), 10.seconds(), false));

        (
            Shared {},
            Local {
                timer,
                rtc,
                exti,
                led: gpioa.pa5.into_push_pull_output(),
            },
            init::Monotonics(),
        )
    }

    #[task(binds = TIM17, local = [led, timer])]
    fn timer_tick(ctx: timer_tick::Context) {
        ctx.local.led.toggle().unwrap();
        ctx.local.timer.clear_irq();
    }

    #[task(binds = EXTI4_15, local = [exti, rtc])]
    fn button_click(ctx: button_click::Context) {
        let date = ctx.local.rtc.get_date();
        let time = ctx.local.rtc.get_time();
        hprintln!("Button pressed @ {:?} {:?}", date, time).unwrap();
        ctx.local.exti.unpend(Event::GPIO13);
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }
}

#![no_std]
#![no_main]
#![deny(warnings)]

extern crate panic_probe;
extern crate stm32g0xx_hal as hal;

use defmt_rtt as _;
use hal::exti::Event;
use hal::gpio::*;
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
        led: PA15<Output<PushPull>>,
        rtc: Rtc,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let mut rcc = ctx.device.RCC.constrain();
        let gpioa = ctx.device.GPIOA.split(&mut rcc);

        let mut timer = ctx.device.TIM17.timer(&mut rcc);
        timer.start(Hertz::Hz(3).into_duration());
        timer.listen();

        let mut exti = ctx.device.EXTI;
        gpioa
            .pa2
            .into_pull_up_input()
            .listen(SignalEdge::Falling, &mut exti);

        let mut rtc = ctx.device.RTC.constrain(&mut rcc);
        rtc.set_date(&Date::new(2019.year(), 11.month(), 24.day()));
        rtc.set_time(&Time::new(21.hours(), 15.minutes(), 10.secs(), false));

        (
            Shared {},
            Local {
                timer,
                rtc,
                exti,
                led: gpioa.pa15.into_push_pull_output(),
            },
        )
    }

    #[task(binds = TIM17, local = [led, timer])]
    fn timer_tick(ctx: timer_tick::Context) {
        ctx.local.led.toggle().unwrap();
        ctx.local.timer.clear_irq();
    }

    #[task(binds = EXTI2_3, local = [exti, rtc])]
    fn button_click(ctx: button_click::Context) {
        let date = ctx.local.rtc.get_date();
        let time = ctx.local.rtc.get_time();
        defmt::info!(
            "Button pressed @ {:?} {:?}",
            defmt::Debug2Format(&date),
            defmt::Debug2Format(&time)
        );
        ctx.local.exti.unpend(Event::GPIO2);
    }
}

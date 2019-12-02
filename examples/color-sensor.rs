// TCS3210 programmable color light-to-frequency converter example
#![no_std]
#![no_main]
#![deny(warnings)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;
extern crate rtfm;
extern crate stm32g0xx_hal as hal;

use core::fmt::Write;

use hal::exti::Event;
use hal::gpio::gpioa::*;
use hal::gpio::{Output, PushPull, SignalEdge};
use hal::prelude::*;
use hal::rcc;
use hal::serial::{self, Serial};
use hal::stm32;
use hal::timer::Timer;
use rtfm::app;

pub enum ColorChannel {
    R,
    G,
    B,
    A,
}

pub struct Color {
    r: u32,
    g: u32,
    b: u32,
    a: u32,
}

impl Default for Color {
    fn default() -> Color {
        Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
}

#[app(device = hal::stm32, peripherals = true)]
const APP: () = {
    struct Resources {
        counter: u32,
        color: Color,
        channel: ColorChannel,
        exti: stm32::EXTI,
        s2: PA9<Output<PushPull>>,
        s3: PA7<Output<PushPull>>,
        led: PA5<Output<PushPull>>,
        uart: Serial<stm32::USART2>,
        timer: Timer<stm32::TIM16>,
        log_timer: Timer<stm32::TIM17>,
    }

    #[init]
    fn init(ctx: init::Context) -> init::LateResources {
        let mut rcc = ctx.device.RCC.freeze(rcc::Config::pll());
        let mut exti = ctx.device.EXTI;

        let gpioa = ctx.device.GPIOA.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        gpioa.pa1.listen(SignalEdge::Falling, &mut exti);
        gpioc.pc13.listen(SignalEdge::Falling, &mut exti);

        let led = gpioa.pa5.into_push_pull_output();
        let mut s0 = gpioa.pa4.into_push_pull_output();
        let mut s1 = gpioa.pa0.into_push_pull_output();
        let s2 = gpioa.pa9.into_push_pull_output();
        let s3 = gpioa.pa7.into_push_pull_output();

        s0.set_high().unwrap();
        s1.set_low().unwrap();

        let mut timer = ctx.device.TIM16.timer(&mut rcc);
        timer.start(8.hz());
        timer.listen();

        let mut log_timer = ctx.device.TIM17.timer(&mut rcc);
        log_timer.start(2.hz());
        log_timer.listen();

        let uart = ctx
            .device
            .USART2
            .usart(gpioa.pa2, gpioa.pa3, serial::Config::default(), &mut rcc)
            .unwrap();

        init::LateResources {
            uart,
            exti,
            led,
            timer,
            log_timer,
            s2,
            s3,
            counter: 0,
            channel: ColorChannel::A,
            color: Color::default(),
        }
    }

    #[task(binds = EXTI0_1, resources = [exti, counter])]
    fn on_pulse(ctx: on_pulse::Context) {
        *ctx.resources.counter += 1;
        ctx.resources.exti.unpend(Event::GPIO1);
    }
    #[task(binds = EXTI4_15, resources = [exti, counter])]
    fn button_click(ctx: button_click::Context) {
        *ctx.resources.counter = 0;
        ctx.resources.exti.unpend(Event::GPIO13);
    }

    #[task(binds = TIM16, resources = [led, timer, counter, channel, color, s2, s3])]
    fn timer_tick(ctx: timer_tick::Context) {
        match *ctx.resources.channel {
            ColorChannel::R => {
                ctx.resources.color.r = *ctx.resources.counter;
                ctx.resources.s2.set_high().unwrap();
                ctx.resources.s3.set_high().unwrap();
                *ctx.resources.channel = ColorChannel::G;
            }
            ColorChannel::G => {
                ctx.resources.color.g = *ctx.resources.counter;
                ctx.resources.s2.set_low().unwrap();
                ctx.resources.s3.set_high().unwrap();
                *ctx.resources.channel = ColorChannel::B;
            }
            ColorChannel::B => {
                ctx.resources.color.b = *ctx.resources.counter;
                ctx.resources.s2.set_high().unwrap();
                ctx.resources.s3.set_low().unwrap();
                *ctx.resources.channel = ColorChannel::A;
            }
            ColorChannel::A => {
                ctx.resources.color.a = *ctx.resources.counter;
                ctx.resources.s2.set_low().unwrap();
                ctx.resources.s3.set_low().unwrap();
                *ctx.resources.channel = ColorChannel::R;
            }
        }
        *ctx.resources.counter = 0;
        ctx.resources.led.toggle().unwrap();
        ctx.resources.timer.clear_irq();
    }

    #[task(binds = TIM17, resources = [log_timer, uart, color])]
    fn log_timer_tick(ctx: log_timer_tick::Context) {
        writeln!(
            ctx.resources.uart,
            "RGBA: {}, {}, {}, {}\r",
            ctx.resources.color.r,
            ctx.resources.color.g,
            ctx.resources.color.b,
            ctx.resources.color.a
        )
        .unwrap();
        ctx.resources.log_timer.clear_irq();
    }
};

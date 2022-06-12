#![no_std]
#![no_main]
#![deny(warnings)]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_halt;
extern crate rtic;
extern crate stm32g0xx_hal as hal;

use hal::exti::Event;
use hal::gpio::SignalEdge;
use hal::prelude::*;
use hal::rcc;
use hal::stm32;
use hal::time::*;
use hal::timer::pwm::PwmPin;
use hal::timer::{self, Timer};
use infrared::protocols::nec::NecCommand;
use infrared::{protocols::Nec, Sender};
use rtic::app;

const IR_SAMPLERATE: Hertz = Hertz::kHz(20);
const STROBE_COMMAND: NecCommand = NecCommand {
    addr: 0,
    cmd: 15,
    repeat: false,
};

type IrPin = PwmPin<stm32::TIM17, timer::Channel1>;
type IrTimer = Timer<stm32::TIM16>;

#[app(device = hal::stm32, peripherals = true)]
mod app {
    use super::*;

    #[shared]
    struct Shared {
        transmitter: Sender<Nec, IrPin>,
    }

    #[local]
    struct Local {
        timer: IrTimer,
        exti: stm32::EXTI,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut rcc = ctx.device.RCC.freeze(rcc::Config::pll());

        let gpiob = ctx.device.GPIOB.split(&mut rcc);
        let gpioc = ctx.device.GPIOC.split(&mut rcc);

        gpioc.pc13.listen(SignalEdge::Falling, &mut ctx.device.EXTI);

        let mut timer = ctx.device.TIM16.timer(&mut rcc);
        timer.start(IR_SAMPLERATE.into_duration());
        timer.listen();

        let carrier_timer = ctx.device.TIM17.pwm(38.kHz(), &mut rcc);
        let mut ir_pin = carrier_timer.bind_pin(gpiob.pb9);
        ir_pin.set_duty(ir_pin.get_max_duty() / 2);
        let transmitter = Sender::new(IR_SAMPLERATE.raw(), ir_pin);

        (
            Shared { transmitter },
            Local {
                timer,
                exti: ctx.device.EXTI,
            },
            init::Monotonics(),
        )
    }

    #[task(binds = TIM16, local = [timer], shared = [transmitter])]
    fn timer_tick(mut ctx: timer_tick::Context) {
        ctx.shared.transmitter.lock(|tx| tx.tick());
        ctx.local.timer.clear_irq();
    }

    #[task(binds = EXTI4_15, local = [exti], shared = [transmitter])]
    fn button_click(mut ctx: button_click::Context) {
        ctx.shared
            .transmitter
            .lock(|tx| tx.load(&STROBE_COMMAND).expect("failed to send IR command"));
        ctx.local.exti.unpend(Event::GPIO13);
    }
}

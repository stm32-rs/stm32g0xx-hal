use crate::stm32::DMAMUX;

/// Extension trait to split a DMA peripheral into independent channels
pub trait DmaMuxExt {
    /// The type to split the DMA into
    type Channels;

    /// Split the DMA into independent channels
    fn split(self) -> Self::Channels;
}

pub enum DmaMuxIndex {
    dmamux_req_gen0 = 0,
    dmamux_req_gen1 = 1,
    dmamux_req_gen2 = 2,
    dmamux_req_gen3 = 3,
    ADC = 5,

    #[cfg(any(feature = "stm32g041", feature = "stm32g081"))]
    AES_IN = 6,
    #[cfg(any(feature = "stm32g041", feature = "stm32g081"))]
    AES_OUT = 7,
    #[cfg(feature = "stm32g0x1")]
    DAC_Channel1 = 8,
    #[cfg(feature = "stm32g0x1")]
    DAC_Channel2 = 9,

    I2C1_RX = 10,
    I2C1_TX = 11,
    I2C2_RX = 12,
    I2C2_TX = 13,

    #[cfg(feature = "stm32g0x1")]
    LPUART_RX = 14,
    #[cfg(feature = "stm32g0x1")]
    LPUART_TX = 15,

    SPI1_RX = 16,
    SPI1_TX = 17,
    SPI2_RX = 18,
    SPI2_TX = 19,

    TIM1_CH1 = 20,
    TIM1_CH2 = 21,
    TIM1_CH3 = 22,
    TIM1_CH4 = 23,
    TIM1_TRIG_COM = 24,
    TIM1_UP = 25,

    #[cfg(feature = "stm32g0x1")]
    TIM2_CH1 = 26,
    #[cfg(feature = "stm32g0x1")]
    TIM2_CH2 = 27,
    #[cfg(feature = "stm32g0x1")]
    TIM2_CH3 = 28,
    #[cfg(feature = "stm32g0x1")]
    TIM2_CH4 = 29,
    #[cfg(feature = "stm32g0x1")]
    TIM2_TRIG = 30,
    #[cfg(feature = "stm32g0x1")]
    TIM2_UP = 31,

    TIM3_CH1 = 32,
    TIM3_CH2 = 33,
    TIM3_CH3 = 34,
    TIM3_CH4 = 35,
    TIM3_TRIG = 36,
    TIM3_UP = 37,
    #[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
    TIM6_UP = 38,
    #[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
    TIM7_UP = 39,
    TIM15_CH1 = 40,
    TIM15_CH2 = 41,
    TIM15_TRIG_COM = 42,
    TIM15_UP = 43,
    TIM16_CH1 = 44,
    TIM16_COM = 45,
    TIM16_UP = 46,
    TIM17_CH1 = 47,
    TIM17_COM = 48,
    TIM17_UP = 49,

    USART1_RX = 50,
    USART1_TX = 51,
    USART2_RX = 52,
    USART2_TX = 53,
    #[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
    USART3_RX = 54,
    #[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
    USART3_TX = 55,
    #[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
    USART4_RX = 56,
    #[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
    USART4_TX = 57,

    #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
    UCPD1_RX = 58,
    #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
    UCPD1_TX = 59,
    #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
    UCPD2_RX = 60,
    #[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
    UCPD2_TX = 61,
}

impl DmaMuxIndex {
    pub fn val(self) -> u8 {
        self as u8
    }
}

pub enum DmaMuxTriggerSync {
    EXTI_LINE0 = 0,
    EXTI_LINE1 = 1,
    EXTI_LINE2 = 2,
    EXTI_LINE3 = 3,
    EXTI_LINE4 = 4,
    EXTI_LINE5 = 5,
    EXTI_LINE6 = 6,
    EXTI_LINE7 = 7,
    EXTI_LINE8 = 8,
    EXTI_LINE9 = 9,
    EXTI_LINE10 = 10,
    EXTI_LINE11 = 11,
    EXTI_LINE12 = 12,
    EXTI_LINE13 = 13,
    EXTI_LINE14 = 14,
    EXTI_LINE15 = 15,
    dmamux_evt0 = 16,
    dmamux_evt1 = 17,
    dmamux_evt2 = 18,
    dmamux_evt3 = 19,

    #[cfg(feature = "stm32g0x1")]
    LPTIM1_OUT = 20,
    #[cfg(feature = "stm32g0x1")]
    LPTIM2_OUT = 21,

    TIM14_OC = 22,
}
impl DmaMuxTriggerSync {
    pub fn val(self) -> u8 {
        self as u8
    }
}

pub trait DmaMuxChannel {
    fn select_peripheral(&mut self, index: DmaMuxIndex);
}

pub struct Channel<const N: usize> {
    _0: (),
}

impl<const N: usize> DmaMuxChannel for Channel<N> {
    fn select_peripheral(&mut self, index: DmaMuxIndex) {
        let reg = unsafe { &(*DMAMUX::ptr()).ccr(N) };
        reg.write(|w| unsafe { w.dmareq_id().bits(index.val()).ege().set_bit() });
    }
}

#[cfg(any(
    feature = "stm32g070",
    feature = "stm32g071",
    feature = "stm32g081",
    feature = "stm32g0b1",
    feature = "stm32g0c1",
))]
/// DMAMUX channels
pub struct Channels {
    pub ch0: Channel<0>,
    pub ch1: Channel<1>,
    pub ch2: Channel<2>,
    pub ch3: Channel<3>,
    pub ch4: Channel<4>,
    pub ch5: Channel<5>,
    pub ch6: Channel<6>,
}

#[cfg(any(feature = "stm32g030", feature = "stm32g031", feature = "stm32g041"))]
/// DMAMUX channels
pub struct Channels {
    pub ch0: Channel<0>,
    pub ch1: Channel<1>,
    pub ch2: Channel<2>,
    pub ch3: Channel<3>,
    pub ch4: Channel<4>,
}

impl DmaMuxExt for DMAMUX {
    type Channels = Channels;

    fn split(self) -> Self::Channels {
        Channels {
            ch0: Channel::<0> { _0: () },
            ch1: Channel::<1> { _0: () },
            ch2: Channel::<2> { _0: () },
            ch3: Channel::<3> { _0: () },
            ch4: Channel::<4> { _0: () },
            #[cfg(any(
                feature = "stm32g070",
                feature = "stm32g071",
                feature = "stm32g081",
                feature = "stm32g0b1",
                feature = "stm32g0c1",
            ))]
            ch5: Channel::<5> { _0: () },
            #[cfg(any(
                feature = "stm32g070",
                feature = "stm32g071",
                feature = "stm32g081",
                feature = "stm32g0b1",
                feature = "stm32g0c1",
            ))]
            ch6: Channel::<6> { _0: () },
        }
    }
}

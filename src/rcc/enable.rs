use super::*;

macro_rules! bus_enable {
    ($PER:ident => $en:ident) => {
        impl Enable for crate::stm32::$PER {
            #[inline(always)]
            fn enable(rcc: &mut Rcc) {
                Self::Bus::enr(rcc).modify(|_, w| w.$en().set_bit());
            }
            #[inline(always)]
            fn disable(rcc: &mut Rcc) {
                Self::Bus::enr(rcc).modify(|_, w| w.$en().clear_bit());
            }
        }
    };
}
macro_rules! bus_smenable {
    ($PER:ident => $smen:ident) => {
        impl SMEnable for crate::stm32::$PER {
            #[inline(always)]
            fn sleep_mode_enable(rcc: &mut Rcc) {
                Self::Bus::smenr(rcc).modify(|_, w| w.$smen().set_bit());
            }
            #[inline(always)]
            fn sleep_mode_disable(rcc: &mut Rcc) {
                Self::Bus::smenr(rcc).modify(|_, w| w.$smen().clear_bit());
            }
        }
    };
}
macro_rules! bus_reset {
    ($PER:ident => $rst:ident) => {
        impl Reset for crate::stm32::$PER {
            #[inline(always)]
            fn reset(rcc: &mut Rcc) {
                Self::Bus::rstr(rcc).modify(|_, w| w.$rst().set_bit());
                Self::Bus::rstr(rcc).modify(|_, w| w.$rst().clear_bit());
            }
        }
    };
}

macro_rules! bus {
    ($($PER:ident => ($busX:ty, $($en:ident)?, $($smen:ident)?, $($rst:ident)?),)+) => {
        $(
            impl crate::Sealed for crate::stm32::$PER {}
            impl RccBus for crate::stm32::$PER {
                type Bus = $busX;
            }
            $(bus_enable!($PER => $en);)?
            $(bus_smenable!($PER => $smen);)?
            $(bus_reset!($PER => $rst);)?
        )+
    }
}

bus! {
    CRC => (AHB, crcen, crcsmen, crcrst),
    FLASH => (AHB, flashen, flashsmen, flashrst),
    DMA => (AHB, dmaen, dmasmen, dmarst),

    DBG => (APB1, dbgen, dbgsmen, dbgrst),
    I2C1 => (APB1, i2c1en, i2c1smen, i2c1rst),
    I2C2 => (APB1, i2c2en, i2c2smen, i2c2rst),
    PWR => (APB1, pwren, pwrsmen, pwrrst),

    SPI2 => (APB1, spi2en, spi2smen, spi2rst),
    TIM3 => (APB1, tim3en, tim3smen, tim3rst),
    USART2 => (APB1, usart2en, usart2smen, usart2rst),
    WWDG => (APB1, wwdgen, wwdgsmen,),

    ADC => (APB2, adcen, adcsmen, adcrst),
    SPI1 => (APB2, spi1en, spi1smen, spi1rst),
    TIM1 => (APB2, tim1en, tim1smen, tim1rst),
    TIM14 => (APB2, tim14en, tim14smen, tim14rst),
    TIM16 => (APB2, tim16en, tim16smen, tim16rst),
    TIM17 => (APB2, tim17en, tim17smen, tim17rst),
    USART1 => (APB2, usart1en, usart1smen, usart1rst),

    GPIOA => (IOP, iopaen, iopasmen, ioparst),
    GPIOB => (IOP, iopben, iopbsmen, iopbrst),
    GPIOC => (IOP, iopcen, iopcsmen, iopcrst),
    GPIOD => (IOP, iopden, iopdsmen, iopdrst),
    GPIOF => (IOP, iopfen, iopfsmen, iopfrst),
}

#[cfg(any(feature = "stm32g041", feature = "stm32g081"))]
bus! {
    AES => (AHB, aesen, aessmen, aesrst),
    RNG => (AHB, rngen, rngsmen, rngrst),
}

#[cfg(any(feature = "stm32g071", feature = "stm32g081"))]
bus! {
    HDMI_CEC => (APB1, cecen, cecsmen, cecrst),
    DAC => (APB1, dac1en, dac1smen, dac1rst),
    UCPD1 => (APB1, ucpd1en, ucpd1smen, ucpd1rst),
    UCPD2 => (APB1, ucpd2en, ucpd2smen, ucpd2rst),
}

#[cfg(feature = "stm32g0x1")]
bus! {
    LPTIM1 => (APB1, lptim1en, lptim1smen, lptim1rst),
    LPTIM2 => (APB1, lptim2en, lptim2smen, lptim2rst),
    LPUART => (APB1, lpuart1en, lpuart1smen, lpuart1rst),
    TIM2 => (APB1, tim2en, tim2smen, tim2rst),
}

#[cfg(any(feature = "stm32g070", feature = "stm32g071", feature = "stm32g081"))]
bus! {
    TIM6 => (APB1, tim6en, tim6smen, tim6rst),
    TIM7 => (APB1, tim7en, tim7smen, tim7rst),
    USART3 => (APB1, usart3en, usart3smen, usart3rst),
    USART4 => (APB1, usart4en, usart4smen, usart4rst),
    TIM15 => (APB2, tim15en, tim15smen, tim15rst),
}

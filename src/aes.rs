//! Interface to the AES peripheral.
use crate::{
    pac,
    rcc::{Enable, Rcc, Reset},
};
use core::{convert::TryInto, marker::PhantomData};
use nb::block;
use void::Void;

/// Used to identify encryption mode
pub struct Encrypt;

/// Used to identify decryption mode
pub struct Decrypt;

/// A 128-bit block
///
/// The AES peripheral processes 128 bits at a time, so this represents one unit
/// of processing.
pub type Block = [u8; 16];

#[derive(Debug, Copy, Clone)]
pub enum Key {
    Key128([u32; 4]),
    Key256([u32; 8]),
}

impl Key {
    pub fn try_from_slice(key: &[u8]) -> Result<Self, Error> {
        match key.len() {
            16 => {
                let mut res = [0; 4];
                for (idx, w) in res.iter_mut().enumerate() {
                    let i = idx * 4;
                    let word = key[i..i + 4].try_into().unwrap();
                    *w = u32::from_be_bytes(word);
                }
                Ok(Self::Key128(res))
            }
            32 => {
                let mut res = [0; 8];
                for (idx, w) in res.iter_mut().enumerate() {
                    let i = idx * 4;
                    let word = key[i..i + 4].try_into().unwrap();
                    *w = u32::from_be_bytes(word);
                }
                Ok(Self::Key256(res))
            }
            _ => Err(Error::InvalidKeySize),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Busy,
    InvalidKeySize,
}

/// Entry point to the AES API
pub struct AES {
    rb: pac::AES,
}

impl AES {
    /// Initialize the AES peripheral
    pub fn new(aes: pac::AES, rcc: &mut Rcc) -> Self {
        pac::AES::enable(rcc);
        pac::AES::reset(rcc);
        aes.cr().write(|w| {
            w.en()
                .clear_bit()
                .dmaouten()
                .clear_bit()
                .dmainen()
                .clear_bit()
                .errie()
                .clear_bit()
                .ccfie()
                .clear_bit()
                .ccfc()
                .set_bit()
                .errc()
                .set_bit()
        });
        Self { rb: aes }
    }

    pub fn ecb_encrypt(self, key: Key) -> Stream {
        self.enable(
            ECB {
                _mode: PhantomData::<Encrypt>,
            },
            key,
        )
    }

    pub fn ecb_decrypt(self, key: Key) -> Stream {
        self.enable(
            ECB {
                _mode: PhantomData::<Decrypt>,
            },
            key,
        )
    }

    pub fn cbc_encrypt(self, key: Key, init_vector: [u32; 4]) -> Stream {
        self.enable(
            CBC {
                _mode: PhantomData::<Encrypt>,
                init_vector,
            },
            key,
        )
    }

    pub fn cbc_decrypt(self, key: Key, init_vector: [u32; 4]) -> Stream {
        self.enable(
            CBC {
                _mode: PhantomData::<Decrypt>,
                init_vector,
            },
            key,
        )
    }

    pub fn ctr(self, key: Key, init_vector: [u32; 3]) -> Stream {
        self.enable(CTR { init_vector }, key)
    }

    fn enable<M>(self, mode: M, key: Key) -> Stream
    where
        M: Mode,
    {
        mode.enable(&self.rb, key);
        self.rb.cr().modify(|_, w| w.en().set_bit());
        Stream { aes: self }
    }
}

/// An active encryption/decryption stream
///
/// You can get an instance of this struct by calling [`AES::enable`].
pub struct Stream {
    aes: AES,
}

impl Stream {
    /// Processes one block of data
    ///
    /// Writes one block of data to the AES peripheral, wait until it is
    /// processed then reads the processed block and returns it.
    pub fn process(&mut self, input: &Block) -> Result<Block, Error> {
        self.write(input)?;
        let output = block!(self.read()).unwrap();
        Ok(output)
    }

    /// Disable the AES peripheral
    pub fn disable(self) -> AES {
        self.aes.rb.cr().modify(|_, w| w.en().clear_bit());
        self.aes
    }

    fn write(&mut self, block: &Block) -> Result<(), Error> {
        if self.aes.rb.sr().read().wrerr().bit_is_set() {
            return Err(Error::Busy);
        }
        for i in 0..4 {
            let i = i * 4;
            let word = &block[i..i + 4];
            let word = word.try_into().unwrap();
            let word = u32::from_be_bytes(word);
            self.aes.rb.dinr().write(|w| unsafe { w.bits(word) });
        }
        Ok(())
    }

    fn read(&mut self) -> nb::Result<Block, Void> {
        if self.aes.rb.sr().read().ccf().bit_is_clear() {
            return Err(nb::Error::WouldBlock);
        }
        let mut block = [0; 16];
        for i in 0..4 {
            let i = i * 4;
            let word = self.aes.rb.doutr().read().bits();
            let word = word.to_be_bytes();
            (block[i..i + 4]).copy_from_slice(&word);
        }
        self.aes.rb.cr().modify(|_, w| w.ccfc().set_bit());
        Ok(block)
    }
}

/// Implemented for all chaining modes
pub trait Mode {
    fn enable(&self, rb: &pac::AES, key: Key);

    fn write_key(&self, rb: &pac::AES, key: Key) {
        match key {
            Key::Key128(key) => unsafe {
                rb.cr().modify(|_, w| w.keysize().bit(false));
                rb.keyr0().write(|w| w.bits(key[0]));
                rb.keyr1().write(|w| w.bits(key[1]));
                rb.keyr2().write(|w| w.bits(key[2]));
                rb.keyr3().write(|w| w.bits(key[3]));
            },
            Key::Key256(key) => unsafe {
                rb.cr().modify(|_, w| w.keysize().bit(true));
                rb.keyr0().write(|w| w.bits(key[0]));
                rb.keyr1().write(|w| w.bits(key[1]));
                rb.keyr2().write(|w| w.bits(key[2]));
                rb.keyr3().write(|w| w.bits(key[3]));
                rb.keyr4().write(|w| w.bits(key[4]));
                rb.keyr5().write(|w| w.bits(key[5]));
                rb.keyr6().write(|w| w.bits(key[6]));
                rb.keyr7().write(|w| w.bits(key[7]));
            },
        }
    }

    fn derive_key(&self, rb: &pac::AES, key: Key) {
        rb.cr().modify(|_, w| unsafe { w.mode().bits(0b01) });
        Self::write_key(self, rb, key);
        rb.cr().modify(|_, w| w.en().set_bit());
        while rb.sr().read().ccf().bit_is_clear() {}
        rb.cr().modify(|_, w| w.ccfc().set_bit());
    }
}

/// The ECB chaining mode
pub struct ECB<Mode> {
    _mode: PhantomData<Mode>,
}

impl Mode for ECB<Encrypt> {
    fn enable(&self, rb: &pac::AES, key: Key) {
        Self::write_key(self, rb, key);
        rb.cr()
            .modify(|_, w| unsafe { w.mode().bits(0b00).chmod10().bits(0b00).chmod2().bit(false) });
    }
}

impl Mode for ECB<Decrypt> {
    fn enable(&self, rb: &pac::AES, key: Key) {
        Self::derive_key(self, rb, key);
        rb.cr()
            .modify(|_, w| unsafe { w.mode().bits(0b10).chmod10().bits(0b00).chmod2().bit(false) });
    }
}

/// The CBC chaining mode
pub struct CBC<Mode> {
    init_vector: [u32; 4],
    _mode: PhantomData<Mode>,
}

impl Mode for CBC<Encrypt> {
    fn enable(&self, rb: &pac::AES, key: Key) {
        Self::write_key(self, rb, key);
        rb.ivr3().write(|w| unsafe { w.bits(self.init_vector[0]) });
        rb.ivr2().write(|w| unsafe { w.bits(self.init_vector[1]) });
        rb.ivr1().write(|w| unsafe { w.bits(self.init_vector[2]) });
        rb.ivr0().write(|w| unsafe { w.bits(self.init_vector[3]) });
        rb.cr()
            .modify(|_, w| unsafe { w.chmod10().bits(0b01).chmod2().bit(false).mode().bits(0b00) });
    }
}

impl Mode for CBC<Decrypt> {
    fn enable(&self, rb: &pac::AES, key: Key) {
        Self::derive_key(self, rb, key);
        rb.ivr3().write(|w| unsafe { w.bits(self.init_vector[0]) });
        rb.ivr2().write(|w| unsafe { w.bits(self.init_vector[1]) });
        rb.ivr1().write(|w| unsafe { w.bits(self.init_vector[2]) });
        rb.ivr0().write(|w| unsafe { w.bits(self.init_vector[3]) });
        rb.cr()
            .modify(|_, w| unsafe { w.chmod10().bits(0b01).chmod2().bit(false).mode().bits(0b10) });
    }
}

/// The CTR chaining mode
pub struct CTR {
    init_vector: [u32; 3],
}

impl Mode for CTR {
    fn enable(&self, rb: &pac::AES, key: Key) {
        Self::write_key(self, rb, key);
        rb.ivr3().write(|w| unsafe { w.bits(self.init_vector[0]) });
        rb.ivr2().write(|w| unsafe { w.bits(self.init_vector[1]) });
        rb.ivr1().write(|w| unsafe { w.bits(self.init_vector[2]) });
        rb.ivr0().write(|w| unsafe { w.bits(0x0001) });
        rb.cr()
            .modify(|_, w| unsafe { w.chmod10().bits(0b10).chmod2().bit(false).mode().bits(0b00) });
    }
}

pub trait AesExt {
    fn constrain(self, rcc: &mut Rcc) -> AES;
}

impl AesExt for pac::AES {
    fn constrain(self, rcc: &mut Rcc) -> AES {
        AES::new(self, rcc)
    }
}

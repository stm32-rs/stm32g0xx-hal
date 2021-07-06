mod traits;

use crate::stm32::FLASH;
use core::convert::TryInto;
use core::mem;
use cortex_m::interrupt;
pub use traits::{Error, FlashPage, Read, Result, WriteErase};

/// The first address of flash memory
pub const FLASH_START: usize = 0x0800_0000;
pub const FLASH_END: usize = 0x0801_FFFF;

/// The size of a Flash memory page, in bytes
pub const PAGE_SIZE: u32 = 2048;
/// How many Flash memory pages there are
pub const NUM_PAGES: u32 = 64;

const FLASH_KEY1: u32 = 0x4567_0123;
const FLASH_KEY2: u32 = 0xCDEF_89AB;

impl FlashPage {
    /// This gives the starting address of a flash page in physical address
    pub const fn to_address(&self) -> usize {
        FLASH_START + self.0 * PAGE_SIZE as usize
    }
}

pub trait FlashExt {
    /// Unlocks Flash memory for erasure and writing
    fn unlock(self) -> core::result::Result<UnlockedFlash, FLASH>;
}

impl FlashExt for FLASH {
    fn unlock(self) -> core::result::Result<UnlockedFlash, FLASH> {
        // Wait, while the memory interface is busy.
        while self.sr.read().bsy().bit_is_set() {}

        // Unlock flash
        self.keyr.write(|w| unsafe { w.keyr().bits(FLASH_KEY1) });
        self.keyr.write(|w| unsafe { w.keyr().bits(FLASH_KEY2) });

        // Verify success
        if self.cr.read().lock().bit_is_clear() {
            Ok(UnlockedFlash { f: self })
        } else {
            Err(self)
        }
    }
}

/// Handle for an unlocked flash on which operations can be performed
pub struct UnlockedFlash {
    f: FLASH,
}

impl UnlockedFlash {
    /// Consumes the unlocked flash instance returning the locked one
    pub fn lock(self) -> FLASH {
        self.f.cr.modify(|_, w| w.lock().set_bit());
        self.f
    }
}

impl Read for UnlockedFlash {
    type NativeType = u8;

    fn read_native(&self, address: usize, array: &mut [Self::NativeType]) {
        let mut address = address as *const Self::NativeType;

        for data in array {
            unsafe {
                *data = core::ptr::read(address);
                address = address.add(1);
            }
        }
    }

    fn read(&self, address: usize, buf: &mut [u8]) {
        self.read_native(address, buf);
    }
}

impl WriteErase for UnlockedFlash {
    type NativeType = u64;

    fn status(&self) -> Result {
        let sr = self.f.sr.read();

        if sr.bsy().bit_is_set() {
            return Err(Error::Busy);
        }

        if sr.pgaerr().bit_is_set() || sr.progerr().bit_is_set() || sr.wrperr().bit_is_set() {
            return Err(Error::Illegal);
        }

        Ok(())
    }

    fn erase_page(&mut self, page: FlashPage) -> Result {
        if page.0 >= NUM_PAGES as usize {
            return Err(Error::PageOutOfRange);
        }

        // Wait, while the memory interface is busy.
        while self.f.sr.read().bsy().bit_is_set() {}

        self.clear_errors();

        // We absoluty can't have any access to Flash while preparing the
        // erase, or the process will be interrupted. This includes any
        // access to the vector table or interrupt handlers that might be
        // caused by an interrupt.
        interrupt::free(|_| {
            self.f.cr.modify(|_, w| unsafe {
                w.per().set_bit().pnb().bits(page.0 as u8).strt().set_bit()
            });
        });

        let result = self.wait();
        self.f.cr.modify(|_, w| w.per().clear_bit());

        result
    }

    fn write_native(&mut self, address: usize, array: &[Self::NativeType]) -> Result {
        // Wait, while the memory interface is busy.
        while self.f.sr.read().bsy().bit_is_set() {}

        // Enable Flash programming
        self.clear_errors();
        self.f.cr.modify(|_, w| w.pg().set_bit());

        // It is only possible to program a double word (2 x 32-bit data).
        let mut address = address as *mut u32;

        for &word in array {
            // We absoluty can't have any access to Flash while preparing the
            // write, or the process will be interrupted. This includes any
            // access to the vector table or interrupt handlers that might be
            // caused by an interrupt.
            interrupt::free(|_| {
                // Safe, because we've verified the valididty of `address`.
                unsafe {
                    address.write_volatile(word as u32);
                    address.offset(1).write_volatile((word >> 32) as u32);

                    address = address.add(2);
                }
            });

            self.wait()?;

            if self.f.sr.read().eop().bit_is_set() {
                self.f.sr.modify(|_, w| w.eop().clear_bit());
            }
        }

        self.f.cr.modify(|_, w| w.pg().clear_bit());

        Ok(())
    }

    fn write(&mut self, address: usize, data: &[u8]) -> Result {
        let address_offset = address % mem::align_of::<Self::NativeType>();
        let unaligned_size = (mem::size_of::<Self::NativeType>() - address_offset)
            % mem::size_of::<Self::NativeType>();

        if unaligned_size > 0 {
            let unaligned_data = &data[..unaligned_size];
            // Handle unaligned address data, make it into a native write
            let mut data = 0xffff_ffff_ffff_ffffu64;
            for b in unaligned_data {
                data = (data >> 8) | ((*b as Self::NativeType) << 56);
            }

            let unaligned_address = address - address_offset;
            let native = &[data];
            self.write_native(unaligned_address, native)?;
        }

        // Handle aligned address data
        let aligned_data = &data[unaligned_size..];
        let mut aligned_address = if unaligned_size > 0 {
            address - address_offset + mem::size_of::<Self::NativeType>()
        } else {
            address
        };

        let mut chunks = aligned_data.chunks_exact(mem::size_of::<Self::NativeType>());

        for exact_chunk in &mut chunks {
            // Write chunks
            let native = &[Self::NativeType::from_ne_bytes(
                exact_chunk.try_into().unwrap(),
            )];
            self.write_native(aligned_address, native)?;
            aligned_address += mem::size_of::<Self::NativeType>();
        }

        let rem = chunks.remainder();

        if !rem.is_empty() {
            let mut data = 0xffff_ffff_ffff_ffffu64;
            // Write remainder
            for b in rem.iter().rev() {
                data = (data << 8) | *b as Self::NativeType;
            }

            let native = &[data];
            self.write_native(aligned_address, native)?;
        }

        Ok(())
    }
}

impl UnlockedFlash {
    fn clear_errors(&mut self) {
        self.f.sr.modify(|_, w| {
            w.progerr()
                .set_bit()
                .pgserr()
                .set_bit()
                .rderr()
                .set_bit()
                .optverr()
                .set_bit()
                .sizerr()
                .set_bit()
                .pgaerr()
                .set_bit()
                .wrperr()
                .set_bit()
        });
    }

    fn wait(&self) -> Result {
        while self.f.sr.read().bsy().bit_is_set() {}
        self.status()
    }
}

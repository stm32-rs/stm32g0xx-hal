use cortex_m::interrupt;
use crate::stm32::FLASH;

/// The first address of flash memory
pub const FLASH_START: u32 = 0x0800_0000;
pub const FLASH_END: u32 = 0x0801_FFFF;

/// The size of a Flash memory page, in bytes
pub const PAGE_SIZE: u32 = 2048;
/// How many Flash memory pages there are
pub const NUM_PAGES: u32 = 64;

pub trait FlashExt {
    fn unlock(&mut self) -> Result<()>;
    fn is_unlocked(&self) -> bool;
    fn erase_page(&mut self, page: u32) -> Result<()>;
    fn write_double_word(&mut self, address: u32, word: u64) -> Result<()>;
    fn read(&mut self, address: u32, length: usize) -> Result<&'static [u8]>;
}

impl FlashExt for FLASH {
    /// Unlocks Flash memory for erasure and writing.
    fn unlock(&mut self) -> Result<()> {
        // Wait, while the memory interface is busy.
        while self.sr.read().bsy().bit_is_set() {}

        // Unlock flash
        self.keyr.write(|w| unsafe { w.keyr().bits(0x4567_0123)} );
        self.keyr.write(|w| unsafe { w.keyr().bits(0xcdef_89ab)} );

        // Verify success
        verify_unlocked()
    }

    /// Returns true if Flash is unlocked and can be erased and programmed.
    fn is_unlocked(&self) -> bool {
        self.cr.read().lock().bit_is_clear()
    }

    /// Erases a page of Flash memory
    ///
    /// Attention:
    /// Flash must be unlocked with `unlock` before `erase_page` can be used.
    /// Make sure that your program is not executed from the
    /// same Flash bank that you are going to erase.
    fn erase_page(&mut self, page: u32) -> Result<()> {
        verify_unlocked()?;
        verify_page(page)?;

        // Wait, while the memory interface is busy.
        while self.sr.read().bsy().bit_is_set() {}

        reset_errors();
        self.cr.modify(|_, w| unsafe {
            w
            .per().set_bit()
            .pnb().bits(page as u8)
            .strt().set_bit()
        });

        // Wait for operation to complete
        while self.sr.read().bsy().bit_is_set() {}
        self.cr.modify(|_, w| w.per().clear_bit());

        check_errors()
    }

    /// Writes a double word to Flash memory
    ///
    /// Attention:
    /// Flash must be unlocked with `unlock` before `write_double_word` can be used.
    /// Page where address points to must be erased using `erase_page` before writing.
    /// If you use this method to write to Flash memory, the address must have
    /// been erased before, otherwise this method will return an error.
    fn write_double_word(&mut self, address: u32, word: u64) -> Result<()> {
        verify_unlocked()?;
        verify_address(address)?;

        // Wait, while the memory interface is busy.
        while self.sr.read().bsy().bit_is_set() {}

        // Enable Flash programming
        reset_errors();
        self.cr.modify(|_,w| w.pg().set_bit());

        // It is only possible to program a double word (2 x 32-bit data).
        let address = address as *mut u32;

        // We absoluty can't have any access to Flash while preparing the
        // write, or the process will be interrupted. This includes any
        // access to the vector table or interrupt handlers that might be
        // caused by an interrupt.
        interrupt::free(|_| {
            // Safe, because we've verified the valididty of `address`.
            unsafe {
                address.write_volatile(word as u32);
                address.offset(1).write_volatile((word >> 32) as u32);
            }
        });

        // Wait for operation to complete
        while self.sr.read().bsy().bit_is_set() {}
        self.cr.modify(|_, w| w.pg().clear_bit());
        check_errors()
    }

    /// Read from flash.
    /// Returns a &[u8] if the address and length are valid.
    /// Length must be a multiple of 4.
    fn read(&mut self, address: u32, length: usize) -> Result<&'static [u8]> {
        verify_address(address)?;
        verify_length(length)?;
        let address = address as *const _;
        unsafe {
            Ok(core::slice::from_raw_parts::<'static, u8>(address, length))
        }
    }
}

/// Make sure Flash is unlocked
fn verify_unlocked() -> Result<()> {
    let flash = unsafe { &(*FLASH::ptr()) };

    if flash.cr.read().lock().bit_is_clear() {
        Ok(())
    } else {
        Err(Error::NotUnlocked)
    }
}

/// Make sure address points to Flash area
fn verify_address(address: u32) -> Result<()> {
    // -8 because it is only possible to program a double word (64 bit, 8 bytes)
    if FLASH_START <= address && address <= FLASH_END - 8 {
        Ok(())
    } else {
        Err(Error::InvalidAddress)
    }
}

/// Make sure that length is multiple of 4.
fn verify_length(length: usize) -> Result<()> {
    if (length & 3) == 0 {
        Ok(())
    } else {
        Err(Error::InvalidLength)
    }
}

/// Make sure that device supports this flash page
fn verify_page(page: u32) -> Result<()> {
    if page < NUM_PAGES {
        Ok(())
    } else {
        Err(Error::InvalidPage)
    }
}

fn reset_errors() {
    let flash = unsafe { &(*FLASH::ptr()) };

    flash.sr.modify(|_, w|
        w
        .progerr().set_bit()
        .pgserr().set_bit()
        .rderr().set_bit()
        .optverr().set_bit()
        .sizerr().set_bit()
        .pgaerr().set_bit()
        .wrperr().set_bit()
    );
}

fn check_errors() -> Result<()> {
    let flash = unsafe { &(*FLASH::ptr()) };
    let sr = flash.sr.read();

    if sr.progerr().bit_is_set() {
        return Err(Error::NotProgrammed);
    }
    if sr.pgserr().bit_is_set() {
        return Err(Error::InvalidProgrammingSequence);
    }
    if sr.rderr().bit_is_set() {
        return Err(Error::ReadProtection);
    }
    if sr.optverr().bit_is_set() {
        return Err(Error::ConfigMismatch);
    }
    if sr.sizerr().bit_is_set() {
        return Err(Error::InvalidSize);
    }
    if sr.pgaerr().bit_is_set() {
        return Err(Error::InvalidAlignment);
    }
    if sr.wrperr().bit_is_set() {
        return Err(Error::WriteProtection);
    }

    Ok(())
}

type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    /// Invalid flash address
    InvalidAddress,

    /// Invalid page
    InvalidPage,

    /// Length is not multiple of 4
    InvalidLength,

    /// Flash is locked
    NotUnlocked,

    /// Failed to write memory that was not erased
    ///
    /// See NOTZEROERR bit in SR register.
    NotProgrammed,

    /// Programming sequence error
    ///
    /// See PGSERR bit in SR register.
    InvalidProgrammingSequence,

    /// Attempted to read protected memory
    ///
    /// See RDERR bit in SR register.
    ReadProtection,

    /// Configuration mismatch
    ///
    /// See OPTVERR bit in SR register.
    ConfigMismatch,

    /// Size of data to program is not correct
    ///
    /// See SIZERR bit in SR register.
    InvalidSize,

    /// Incorrect alignment when programming half-page
    ///
    /// See PGAERR bit in SR register.
    InvalidAlignment,

    /// Attempted to write to protected memory
    ///
    /// See WRPERR in SR register.
    WriteProtection,
}

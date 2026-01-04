/// The Bus trait abstracts the memory and I/O address space.
/// Components like the CPU use this trait to read/write without
/// knowing whether they're accessing RAM, ROM, or memory-mapped I/O.
///
/// This trait is generic across all 6502-based systems. Platform-specific
/// differences (Atari 800, C64, Apple II, etc.) are handled in the
/// implementation of this trait.
pub trait Bus {
    /// Read a byte from the given address
    fn read(&mut self, addr: u16) -> u8;

    /// Write a byte to the given address
    fn write(&mut self, addr: u16, val: u8);

    /// Read a 16-bit word (little-endian) from the given address
    fn read_word(&mut self, addr: u16) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    /// Write a 16-bit word (little-endian) to the given address
    fn write_word(&mut self, addr: u16, val: u16) {
        let lo = (val & 0xff) as u8;
        let hi = (val >> 8) as u8;
        self.write(addr, lo);
        self.write(addr.wrapping_add(1), hi);
    }
}

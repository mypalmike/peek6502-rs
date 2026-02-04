// ROM Overlay Controller for Atari 800XL/130XE
//
// Controls ROM overlays via PORTB ($D301):
// - Bit 0: OS ROM at $C000-$CFFF, $D800-$FFFF (1=enabled, 0=disabled/RAM visible)
// - Bit 1: BASIC ROM at $A000-$BFFF (0=enabled, 1=disabled/RAM visible)
// - Bit 7: Self-test ROM at $5000-$57FF (0=enabled, 1=disabled/RAM visible)
//
// When ROM is overlaid:
//   - Reads come from ROM
//   - Writes are ignored (ROM is read-only)
// When ROM is disabled:
//   - Reads and writes go to underlying RAM

use crate::memory_region::RomRegion;

pub struct RomOverlayController {
    // OS ROM at $C000-$CFFF, $D800-$FFFF
    os_rom: Option<RomRegion>,
    os_rom_enabled: bool,

    // BASIC ROM at $A000-$BFFF
    basic_rom: Option<RomRegion>,
    basic_rom_enabled: bool,

    // Self-test ROM at $5000-$57FF
    selftest_rom: Option<RomRegion>,
    selftest_rom_enabled: bool,

    // Current PORTB value
    portb: u8,
}

impl RomOverlayController {
    pub fn new() -> Self {
        RomOverlayController {
            os_rom: None,
            os_rom_enabled: true,  // Default: OS ROM enabled
            basic_rom: None,
            basic_rom_enabled: false,  // Default: BASIC ROM disabled
            selftest_rom: None,
            selftest_rom_enabled: false,  // Default: Self-test disabled
            portb: 0xFD,  // Default PORTB: OS on, BASIC off, self-test off
        }
    }

    /// Add OS ROM overlay
    pub fn set_os_rom(&mut self, rom_data: Vec<u8>) {
        // OS ROM covers $C000-$CFFF and $D800-$FFFF (with I/O hole at $D000-$D7FF)
        self.os_rom = Some(RomRegion::new(0xC000, rom_data, 50));
    }

    /// Add BASIC ROM overlay
    pub fn set_basic_rom(&mut self, rom_data: Vec<u8>) {
        self.basic_rom = Some(RomRegion::new(0xA000, rom_data, 50));
    }

    /// Add self-test ROM overlay
    pub fn set_selftest_rom(&mut self, rom_data: Vec<u8>) {
        self.selftest_rom = Some(RomRegion::new(0x5000, rom_data, 50));
    }

    /// Update PORTB and adjust ROM overlays
    pub fn update_portb(&mut self, portb: u8) {
        self.portb = portb;

        // Bit 0: OS ROM enable (1=enabled, 0=disabled)
        self.os_rom_enabled = (portb & 0x01) != 0;

        // Bit 1: BASIC disable (0=enabled, 1=disabled)
        self.basic_rom_enabled = (portb & 0x02) == 0;

        // Bit 7: Self-test disable (0=enabled, 1=disabled)
        self.selftest_rom_enabled = (portb & 0x80) == 0;
    }

    /// Read from ROM if overlay is enabled and contains address
    pub fn read(&self, addr: u16) -> Option<u8> {
        // Check OS ROM ($C000-$CFFF, $D800-$FFFF, excluding I/O at $D000-$D7FF)
        if self.os_rom_enabled {
            if let Some(ref rom) = self.os_rom {
                if rom.contains(addr) && !(addr >= 0xD000 && addr <= 0xD7FF) {
                    return Some(rom.read(addr));
                }
            }
        }

        // Check BASIC ROM ($A000-$BFFF)
        if self.basic_rom_enabled {
            if let Some(ref rom) = self.basic_rom {
                if rom.contains(addr) {
                    return Some(rom.read(addr));
                }
            }
        }

        // Check self-test ROM ($5000-$57FF)
        if self.selftest_rom_enabled {
            if let Some(ref rom) = self.selftest_rom {
                if rom.contains(addr) {
                    return Some(rom.read(addr));
                }
            }
        }

        None  // No ROM overlay active at this address
    }

    /// Check if write should be blocked by ROM overlay
    /// Returns true if ROM is overlaid (write should be ignored)
    pub fn is_write_blocked(&self, addr: u16) -> bool {
        // OS ROM blocks writes
        if self.os_rom_enabled {
            if let Some(ref rom) = self.os_rom {
                if rom.contains(addr) && !(addr >= 0xD000 && addr <= 0xD7FF) {
                    return true;
                }
            }
        }

        // BASIC ROM blocks writes
        if self.basic_rom_enabled {
            if let Some(ref rom) = self.basic_rom {
                if rom.contains(addr) {
                    return true;
                }
            }
        }

        // Self-test ROM blocks writes
        if self.selftest_rom_enabled {
            if let Some(ref rom) = self.selftest_rom {
                if rom.contains(addr) {
                    return true;
                }
            }
        }

        false  // No ROM overlay, write can proceed to RAM
    }
}

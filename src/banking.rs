// Banking controller for memory bank switching
//
// Handles platform-specific banking schemes:
// - Atari 800: No banking
// - Atari 800XL: No RAM banking (ROM overlays handled by RomOverlayController)
// - Atari 130XE: RAM banking at $4000-$7FFF (future implementation)

use crate::memory_region::BankedRegion;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BankingScheme {
    None,      // No banking (Atari 800, 800XL)
    Atari130XE,   // 130XE extended RAM banking (future)
}

pub struct BankController {
    pub scheme: BankingScheme,
    pub portb_value: u8,

    // Atari 800XL banked regions
    // Bank 0 = ROM visible, Bank 1 = RAM visible (ROM disabled)
    pub os_rom_bank: Option<BankedRegion>,    // $C000-$FFFF
    pub basic_rom_bank: Option<BankedRegion>, // $A000-$BFFF
}

impl BankController {
    pub fn new(scheme: BankingScheme) -> Self {
        BankController {
            scheme,
            portb_value: 0x00,  // Default: ROMs enabled on boot (bits 0-1 low)
            os_rom_bank: None,
            basic_rom_bank: None,
        }
    }

    // Future: 130XE extended RAM banking methods will go here

    /// Read from a banked region if it contains the address and is visible
    /// Returns None if ROM is disabled (should fall through to RAM)
    pub fn read(&self, addr: u16) -> Option<u8> {
        if self.scheme == BankingScheme::None {
            return None;
        }

        // Check BASIC ROM first (lower address)
        if let Some(ref basic_bank) = self.basic_rom_bank {
            if basic_bank.contains(addr) {
                // Only return ROM data if bank 0 is active (ROM enabled)
                if basic_bank.active_bank == 0 {
                    return Some(basic_bank.read(addr));
                } else {
                    return None;  // Bank 1 (RAM) - fall through to main RAM
                }
            }
        }

        // Check OS ROM
        if let Some(ref os_bank) = self.os_rom_bank {
            if os_bank.contains(addr) {
                if os_bank.active_bank == 0 {
                    return Some(os_bank.read(addr));
                } else {
                    return None;
                }
            }
        }

        None
    }

    /// Write to a banked region
    /// Returns true if write was handled (even if ignored due to ROM)
    pub fn write(&mut self, addr: u16, _val: u8) -> bool {
        if self.scheme == BankingScheme::None {
            return false;
        }

        // Check BASIC ROM region
        if let Some(ref mut basic_bank) = self.basic_rom_bank {
            if basic_bank.contains(addr) {
                // Writes to ROM are ignored, but we return true (handled)
                // Writes when RAM is visible fall through to main RAM
                return basic_bank.active_bank == 0;
            }
        }

        // Check OS ROM region
        if let Some(ref mut os_bank) = self.os_rom_bank {
            if os_bank.contains(addr) {
                return os_bank.active_bank == 0;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_banking() {
        let controller = BankController::new(BankingScheme::None);
        assert_eq!(controller.read(0xC000), None);
        // No banking means all reads/writes return None/false
    }
}

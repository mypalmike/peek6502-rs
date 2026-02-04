// Banking controller for memory bank switching
//
// Handles platform-specific banking schemes:
// - Atari 800: No banking
// - Atari 800XL: PORTB ($D301) controls OS ROM and BASIC ROM visibility

use crate::memory_region::BankedRegion;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BankingScheme {
    None,      // No banking (Atari 800)
    AtariXL,   // 800XL PORTB banking
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

    /// Add OS ROM banking region (800XL only)
    /// Creates a banked region with ROM in bank 0, RAM in bank 1
    pub fn add_os_rom_banking(&mut self, os_rom_data: Vec<u8>) {
        if self.scheme != BankingScheme::AtariXL {
            return;
        }

        // Bank 0 = OS ROM, Bank 1 = RAM (invisible, falls through to main RAM)
        let ram_bank = vec![0; os_rom_data.len()];  // Placeholder, won't be used
        let banks = vec![os_rom_data, ram_bank];

        self.os_rom_bank = Some(BankedRegion::new(
            0xC000,
            0xFFFF,
            banks,
            60,  // Higher priority than base ROM, lower than I/O
            false,
        ));

        // Start with ROM enabled (PORTB bit 0 = 0)
        self.update_portb(0x00);
    }

    /// Add BASIC ROM banking region (800XL only)
    pub fn add_basic_rom(&mut self, basic_rom_data: Vec<u8>) {
        if self.scheme != BankingScheme::AtariXL {
            return;
        }

        // Bank 0 = BASIC ROM, Bank 1 = RAM (invisible)
        let ram_bank = vec![0; basic_rom_data.len()];
        let banks = vec![basic_rom_data, ram_bank];

        self.basic_rom_bank = Some(BankedRegion::new(
            0xA000,
            0xBFFF,
            banks,
            60,
            false,
        ));

        // Start with BASIC enabled (PORTB bit 1 = 0)
        self.update_portb(0x00);
    }

    /// Update PORTB and switch banks accordingly
    /// PORTB bit 0: 0 = OS ROM enabled, 1 = OS ROM disabled (show RAM)
    /// PORTB bit 1: 0 = BASIC ROM enabled, 1 = BASIC ROM disabled (show RAM)
    pub fn update_portb(&mut self, portb: u8) {
        if self.scheme != BankingScheme::AtariXL {
            return;
        }

        self.portb_value = portb;

        // OS ROM banking (bit 0)
        if let Some(ref mut os_bank) = self.os_rom_bank {
            if portb & 0x01 == 0 {
                os_bank.set_bank(0);  // ROM visible
            } else {
                os_bank.set_bank(1);  // RAM visible (actually invisible - falls through)
            }
        }

        // BASIC ROM banking (bit 1)
        if let Some(ref mut basic_bank) = self.basic_rom_bank {
            if portb & 0x02 == 0 {
                basic_bank.set_bank(0);  // ROM visible
            } else {
                basic_bank.set_bank(1);  // RAM visible (invisible)
            }
        }
    }

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
    pub fn write(&mut self, addr: u16, val: u8) -> bool {
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
        let mut controller = BankController::new(BankingScheme::None);
        controller.update_portb(0x00);
        assert_eq!(controller.read(0xC000), None);
        assert_eq!(controller.write(0xC000, 0x42), false);
    }

    #[test]
    fn test_xl_os_banking() {
        let mut controller = BankController::new(BankingScheme::AtariXL);

        // Add OS ROM with test data
        let os_rom = vec![0xAA; 0x4000];  // 16KB
        controller.add_os_rom_banking(os_rom);

        // Initially PORTB = 0x00, ROM should be visible
        controller.update_portb(0x00);
        assert_eq!(controller.read(0xC000), Some(0xAA));

        // Disable OS ROM (bit 0 = 1)
        controller.update_portb(0x01);
        assert_eq!(controller.read(0xC000), None);  // Should fall through to RAM

        // Re-enable OS ROM
        controller.update_portb(0x00);
        assert_eq!(controller.read(0xC000), Some(0xAA));
    }

    #[test]
    fn test_xl_basic_banking() {
        let mut controller = BankController::new(BankingScheme::AtariXL);

        // Add BASIC ROM with test data
        let basic_rom = vec![0xBB; 0x2000];  // 8KB
        controller.add_basic_rom(basic_rom);

        // Initially ROM visible
        controller.update_portb(0x00);
        assert_eq!(controller.read(0xA000), Some(0xBB));

        // Disable BASIC (bit 1 = 1)
        controller.update_portb(0x02);
        assert_eq!(controller.read(0xA000), None);  // Fall through to RAM

        // Re-enable BASIC
        controller.update_portb(0x00);
        assert_eq!(controller.read(0xA000), Some(0xBB));
    }

    #[test]
    fn test_xl_combined_banking() {
        let mut controller = BankController::new(BankingScheme::AtariXL);

        let os_rom = vec![0xAA; 0x4000];
        let basic_rom = vec![0xBB; 0x2000];
        controller.add_os_rom_banking(os_rom);
        controller.add_basic_rom(basic_rom);

        // Both ROMs enabled
        controller.update_portb(0x00);
        assert_eq!(controller.read(0xA000), Some(0xBB));
        assert_eq!(controller.read(0xC000), Some(0xAA));

        // Disable BASIC only
        controller.update_portb(0x02);
        assert_eq!(controller.read(0xA000), None);
        assert_eq!(controller.read(0xC000), Some(0xAA));

        // Disable OS only
        controller.update_portb(0x01);
        assert_eq!(controller.read(0xA000), Some(0xBB));
        assert_eq!(controller.read(0xC000), None);

        // Disable both
        controller.update_portb(0x03);
        assert_eq!(controller.read(0xA000), None);
        assert_eq!(controller.read(0xC000), None);
    }
}

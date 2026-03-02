// Page table based memory system for O(1) address lookup
//
// Replaces the cascade of MemoryMap, RomOverlayController, BankController,
// and cart_rom with a single 256-entry page table.

use crate::bus::ReadBus;

/// Identifies which hardware chip handles I/O for a memory page
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChipId {
    Gtia,
    Pokey,
    Antic,
    Pia,
}

/// What a page maps to for reads or writes
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PageMapping {
    Ram,
    Rom { block_index: u8 },
    Io { chip: ChipId },
    Unmapped,
}

/// A block of ROM data stored in the page table
pub struct RomBlock {
    pub name: &'static str,
    pub data: Vec<u8>,
    pub base_addr: u16,
}

pub struct PageTable {
    pub ram: [u8; 0x10000],
    rom_blocks: Vec<RomBlock>,
    read_map: [PageMapping; 256],
    write_map: [PageMapping; 256],

    // ROM overlay tracking (800XL/130XE PORTB-controlled)
    os_rom_index: Option<u8>,
    basic_rom_index: Option<u8>,
    selftest_rom_index: Option<u8>,
    cart_rom_index: Option<u8>,
    cart_base_page: u8,
    cart_page_count: u8,
    os_rom_enabled: bool,
    basic_rom_enabled: bool,
    selftest_rom_enabled: bool,
}

impl PageTable {
    pub fn new() -> Self {
        PageTable {
            ram: [0; 0x10000],
            rom_blocks: Vec::new(),
            read_map: [PageMapping::Ram; 256],
            write_map: [PageMapping::Ram; 256],
            os_rom_index: None,
            basic_rom_index: None,
            selftest_rom_index: None,
            cart_rom_index: None,
            cart_base_page: 0,
            cart_page_count: 0,
            os_rom_enabled: false,
            basic_rom_enabled: false,
            selftest_rom_enabled: false,
        }
    }

    /// Add a ROM block and return its index. Does NOT map it yet.
    pub fn add_rom(&mut self, name: &'static str, data: Vec<u8>, base_addr: u16) -> u8 {
        let index = self.rom_blocks.len() as u8;
        self.rom_blocks.push(RomBlock { name, data, base_addr });
        index
    }

    /// Map a ROM block's pages into the read map (writes become Unmapped).
    /// Skips pages that are mapped to I/O.
    pub fn map_rom(&mut self, block_index: u8) {
        let block = &self.rom_blocks[block_index as usize];
        let base_page = (block.base_addr >> 8) as usize;
        let page_count = (block.data.len() + 255) / 256;

        for i in 0..page_count {
            let page = base_page + i;
            if page < 256 {
                // Don't override I/O mappings
                if self.read_map[page] != (PageMapping::Io { chip: ChipId::Gtia })
                    && self.read_map[page] != (PageMapping::Io { chip: ChipId::Pokey })
                    && self.read_map[page] != (PageMapping::Io { chip: ChipId::Antic })
                    && self.read_map[page] != (PageMapping::Io { chip: ChipId::Pia })
                {
                    self.read_map[page] = PageMapping::Rom { block_index };
                    self.write_map[page] = PageMapping::Unmapped;
                }
            }
        }
    }

    /// Unmap a ROM block's pages back to Ram.
    /// Skips pages that are mapped to I/O.
    pub fn unmap_rom(&mut self, block_index: u8) {
        let block = &self.rom_blocks[block_index as usize];
        let base_page = (block.base_addr >> 8) as usize;
        let page_count = (block.data.len() + 255) / 256;

        for i in 0..page_count {
            let page = base_page + i;
            if page < 256 {
                if let PageMapping::Rom { block_index: idx } = self.read_map[page] {
                    if idx == block_index {
                        self.read_map[page] = PageMapping::Ram;
                        self.write_map[page] = PageMapping::Ram;
                    }
                }
            }
        }
    }

    /// Map consecutive pages to an I/O chip.
    pub fn map_io(&mut self, start_page: u8, count: u8, chip: ChipId) {
        for i in 0..count {
            let page = start_page.wrapping_add(i) as usize;
            self.read_map[page] = PageMapping::Io { chip };
            self.write_map[page] = PageMapping::Io { chip };
        }
    }

    /// Load and map cartridge ROM.
    pub fn map_cart(&mut self, data: Vec<u8>, base_addr: u16) {
        let idx = self.add_rom("cart", data, base_addr);
        self.cart_rom_index = Some(idx);
        self.cart_base_page = (base_addr >> 8) as u8;
        self.cart_page_count = ((self.rom_blocks[idx as usize].data.len() + 255) / 256) as u8;
        self.map_rom(idx);
    }

    /// Eject cartridge ROM, unmapping its pages back to Ram.
    pub fn eject_cart(&mut self) {
        if let Some(idx) = self.cart_rom_index.take() {
            self.unmap_rom(idx);
            self.cart_page_count = 0;
        }
    }

    /// Set the OS ROM block index (for PORTB-controlled overlay).
    pub fn set_os_rom(&mut self, block_index: u8) {
        self.os_rom_index = Some(block_index);
    }

    /// Set the BASIC ROM block index (for PORTB-controlled overlay).
    pub fn set_basic_rom(&mut self, block_index: u8) {
        self.basic_rom_index = Some(block_index);
    }

    /// Set the self-test ROM block index (for PORTB-controlled overlay).
    pub fn set_selftest_rom(&mut self, block_index: u8) {
        self.selftest_rom_index = Some(block_index);
    }

    /// Update ROM overlays based on PIA PORTB value (800XL/130XE).
    /// - Bit 0: OS ROM enable (1=enabled, 0=disabled/RAM visible)
    /// - Bit 1: BASIC disable (0=enabled, 1=disabled/RAM visible)
    /// - Bit 7: Self-test disable (0=enabled, 1=disabled/RAM visible)
    pub fn update_portb(&mut self, portb: u8) {
        let os_enabled = (portb & 0x01) != 0;
        let basic_enabled = (portb & 0x02) == 0;
        let selftest_enabled = (portb & 0x80) == 0;

        // OS ROM
        if os_enabled != self.os_rom_enabled {
            self.os_rom_enabled = os_enabled;
            if let Some(idx) = self.os_rom_index {
                if os_enabled {
                    self.map_rom(idx);
                } else {
                    self.unmap_rom(idx);
                }
            }
        }

        // BASIC ROM
        if basic_enabled != self.basic_rom_enabled {
            self.basic_rom_enabled = basic_enabled;
            if let Some(idx) = self.basic_rom_index {
                if basic_enabled {
                    self.map_rom(idx);
                } else {
                    self.unmap_rom(idx);
                }
            }
        }

        // Self-test ROM
        if selftest_enabled != self.selftest_rom_enabled {
            self.selftest_rom_enabled = selftest_enabled;
            if let Some(idx) = self.selftest_rom_index {
                if selftest_enabled {
                    self.map_rom(idx);
                } else {
                    self.unmap_rom(idx);
                }
            }
        }
    }

    /// O(1) read. Returns None for I/O pages (caller handles chip dispatch).
    #[inline]
    pub fn read_byte(&self, addr: u16) -> Option<u8> {
        let page = (addr >> 8) as usize;
        match self.read_map[page] {
            PageMapping::Ram => Some(self.ram[addr as usize]),
            PageMapping::Rom { block_index } => {
                let block = &self.rom_blocks[block_index as usize];
                let offset = addr.wrapping_sub(block.base_addr) as usize;
                if offset < block.data.len() {
                    Some(block.data[offset])
                } else {
                    Some(0xFF)
                }
            }
            PageMapping::Io { .. } => None,
            PageMapping::Unmapped => Some(0xFF),
        }
    }

    /// O(1) write. Returns false for I/O pages (caller handles chip dispatch).
    /// Rom and Unmapped writes are silently ignored.
    #[inline]
    pub fn write_byte(&mut self, addr: u16, val: u8) -> bool {
        let page = (addr >> 8) as usize;
        match self.write_map[page] {
            PageMapping::Ram => {
                self.ram[addr as usize] = val;
                true
            }
            PageMapping::Io { .. } => false,
            PageMapping::Rom { .. } | PageMapping::Unmapped => true, // silently ignored
        }
    }

    /// Expose page type for SystemBus I/O dispatch.
    #[inline]
    pub fn read_page_type(&self, addr: u16) -> PageMapping {
        self.read_map[(addr >> 8) as usize]
    }

    /// Expose write page type for SystemBus I/O dispatch.
    #[inline]
    pub fn write_page_type(&self, addr: u16) -> PageMapping {
        self.write_map[(addr >> 8) as usize]
    }

    /// Convenience: read directly from RAM (bypasses page table).
    #[inline]
    pub fn read_ram(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    /// Convenience: write directly to RAM (bypasses page table).
    #[inline]
    pub fn write_ram(&mut self, addr: u16, val: u8) {
        self.ram[addr as usize] = val;
    }

    /// Check if a cartridge is loaded.
    pub fn has_cart(&self) -> bool {
        self.cart_rom_index.is_some()
    }
}

/// ReadBus implementation for immutable memory reads (used by ANTIC for DMA).
/// I/O pages return 0xFF (ANTIC DMA doesn't trigger chip side effects).
impl ReadBus for PageTable {
    fn read(&self, addr: u16) -> u8 {
        self.read_byte(addr).unwrap_or(0xFF)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_read_write() {
        let mut pt = PageTable::new();
        pt.write_byte(0x1234, 0x42);
        assert_eq!(pt.read_byte(0x1234), Some(0x42));
    }

    #[test]
    fn test_rom_read_only() {
        let mut pt = PageTable::new();
        let rom_data = vec![0xAA; 256];
        let idx = pt.add_rom("test", rom_data, 0xF000);
        pt.map_rom(idx);

        // Read should return ROM data
        assert_eq!(pt.read_byte(0xF000), Some(0xAA));

        // Write should be silently ignored
        assert_eq!(pt.write_byte(0xF000, 0x55), true);
        assert_eq!(pt.read_byte(0xF000), Some(0xAA)); // unchanged
    }

    #[test]
    fn test_io_page() {
        let mut pt = PageTable::new();
        pt.map_io(0xD0, 1, ChipId::Gtia);

        // I/O reads return None (caller handles)
        assert_eq!(pt.read_byte(0xD000), None);
        assert_eq!(pt.read_page_type(0xD000), PageMapping::Io { chip: ChipId::Gtia });

        // I/O writes return false (caller handles)
        assert_eq!(pt.write_byte(0xD000, 0x42), false);
    }

    #[test]
    fn test_rom_does_not_override_io() {
        let mut pt = PageTable::new();
        pt.map_io(0xD0, 1, ChipId::Gtia);

        // ROM spanning the I/O page should not override it
        let rom_data = vec![0xBB; 0x4000]; // 16KB from $C000-$FFFF
        let idx = pt.add_rom("os", rom_data, 0xC000);
        pt.map_rom(idx);

        // $D0xx should still be I/O
        assert_eq!(pt.read_page_type(0xD000), PageMapping::Io { chip: ChipId::Gtia });
        // $C0xx should be ROM
        assert_eq!(pt.read_byte(0xC000), Some(0xBB));
    }

    #[test]
    fn test_rom_overlay_portb() {
        let mut pt = PageTable::new();

        // Set up I/O
        pt.map_io(0xD0, 1, ChipId::Gtia);

        // Add OS ROM at $C000 (16KB)
        let os_data = vec![0xCC; 0x4000];
        let os_idx = pt.add_rom("os", os_data, 0xC000);
        pt.set_os_rom(os_idx);

        // Add BASIC ROM at $A000 (8KB)
        let basic_data = vec![0xBB; 0x2000];
        let basic_idx = pt.add_rom("basic", basic_data, 0xA000);
        pt.set_basic_rom(basic_idx);

        // Initially disabled
        assert_eq!(pt.read_byte(0xC000), Some(0)); // RAM (zero)
        assert_eq!(pt.read_byte(0xA000), Some(0)); // RAM (zero)

        // Enable OS ROM (bit 0 = 1), disable BASIC (bit 1 = 1)
        pt.update_portb(0xFF);
        assert_eq!(pt.read_byte(0xC000), Some(0xCC)); // OS ROM

        // Enable BASIC (bit 1 = 0)
        pt.update_portb(0xFD);
        assert_eq!(pt.read_byte(0xA000), Some(0xBB)); // BASIC ROM

        // Disable OS ROM (bit 0 = 0)
        pt.update_portb(0xFC);
        assert_eq!(pt.read_byte(0xC000), Some(0)); // back to RAM
    }

    #[test]
    fn test_cartridge() {
        let mut pt = PageTable::new();

        // Map cartridge at $A000 (8KB)
        let cart_data = vec![0xDD; 0x2000];
        pt.map_cart(cart_data, 0xA000);

        assert_eq!(pt.read_byte(0xA000), Some(0xDD));
        assert_eq!(pt.write_byte(0xA000, 0x00), true); // silently ignored
        assert_eq!(pt.read_byte(0xA000), Some(0xDD));

        // Eject
        pt.eject_cart();
        assert_eq!(pt.read_byte(0xA000), Some(0)); // back to RAM
    }

    #[test]
    fn test_readbus_io_returns_ff() {
        let mut pt = PageTable::new();
        pt.map_io(0xD0, 1, ChipId::Gtia);

        // ReadBus should return 0xFF for I/O pages
        assert_eq!(ReadBus::read(&pt, 0xD000), 0xFF);
    }

    #[test]
    fn test_ram_helpers() {
        let mut pt = PageTable::new();
        pt.write_ram(0x0100, 0x42);
        assert_eq!(pt.read_ram(0x0100), 0x42);

        // Also accessible via ram array
        assert_eq!(pt.ram[0x0100], 0x42);
    }

    #[test]
    fn test_unmap_rom_only_affects_own_pages() {
        let mut pt = PageTable::new();

        let rom1 = vec![0xAA; 0x1000]; // 4KB at $E000
        let idx1 = pt.add_rom("rom1", rom1, 0xE000);
        pt.map_rom(idx1);

        let rom2 = vec![0xBB; 0x1000]; // 4KB at $F000
        let idx2 = pt.add_rom("rom2", rom2, 0xF000);
        pt.map_rom(idx2);

        // Unmap rom1 should not affect rom2
        pt.unmap_rom(idx1);
        assert_eq!(pt.read_byte(0xE000), Some(0)); // back to RAM
        assert_eq!(pt.read_byte(0xF000), Some(0xBB)); // still ROM
    }
}

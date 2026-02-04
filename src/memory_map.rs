// Memory map manager for flexible memory region configuration
//
// This provides a priority-based memory region system that replaces hardcoded
// address matching. Different machines can have different memory layouts by
// configuring different region sets.

use crate::memory_region::MemoryRegionType;

pub struct MemoryMap {
    regions: Vec<MemoryRegionType>,
    // Optional optimization: page cache for fast lookups
    // page_cache: [Option<usize>; 256],
}

impl MemoryMap {
    pub fn new() -> Self {
        MemoryMap {
            regions: Vec::new(),
        }
    }

    /// Add a memory region and maintain priority ordering
    pub fn add_region(&mut self, region: MemoryRegionType) {
        self.regions.push(region);
        // Sort by priority (highest first)
        self.regions.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Find the highest-priority region containing this address
    pub fn find_region(&self, addr: u16) -> Option<&MemoryRegionType> {
        self.regions.iter().find(|r| r.contains(addr))
    }

    /// Find the highest-priority region containing this address (mutable)
    pub fn find_region_mut(&mut self, addr: u16) -> Option<&mut MemoryRegionType> {
        self.regions.iter_mut().find(|r| r.contains(addr))
    }

    /// Read from the highest-priority region that contains this address
    /// Returns None for I/O regions (they don't store data)
    pub fn read(&self, addr: u16) -> Option<u8> {
        if let Some(region) = self.find_region(addr) {
            region.read(addr)
        } else {
            None
        }
    }

    /// Write to the highest-priority region that contains this address
    /// Returns true if a region handled the write
    pub fn write(&mut self, addr: u16, val: u8) -> bool {
        if let Some(region) = self.find_region_mut(addr) {
            region.write(addr, val)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_region::{RamRegion, RomRegion, IoRegion, ChipType, MemoryRegionType};

    #[test]
    fn test_ram_region() {
        let mut map = MemoryMap::new();
        map.add_region(MemoryRegionType::Ram(RamRegion::new(0x0000, 0xFFFF, 0)));

        // Write and read
        map.write(0x1234, 0x42);
        assert_eq!(map.read(0x1234), Some(0x42));
    }

    #[test]
    fn test_rom_region() {
        let mut map = MemoryMap::new();
        let rom_data = vec![0x01, 0x02, 0x03, 0x04];
        map.add_region(MemoryRegionType::Rom(RomRegion::new(0xF000, rom_data, 10)));

        // Read ROM
        assert_eq!(map.read(0xF000), Some(0x01));
        assert_eq!(map.read(0xF001), Some(0x02));

        // Write to ROM (ignored)
        map.write(0xF000, 0xFF);
        assert_eq!(map.read(0xF000), Some(0x01));  // Unchanged
    }

    #[test]
    fn test_priority_ordering() {
        let mut map = MemoryMap::new();

        // Add RAM (low priority)
        map.add_region(MemoryRegionType::Ram(RamRegion::new(0x0000, 0xFFFF, 0)));

        // Add ROM overlay (high priority)
        let rom_data = vec![0xAA; 256];
        map.add_region(MemoryRegionType::Rom(RomRegion::new(0xF000, rom_data, 50)));

        // Write to RAM at ROM location
        map.write(0xF000, 0x42);

        // Should read ROM (higher priority), not RAM
        assert_eq!(map.read(0xF000), Some(0xAA));

        // Read from non-ROM location should get RAM
        map.write(0x1000, 0x55);
        assert_eq!(map.read(0x1000), Some(0x55));
    }

    #[test]
    fn test_io_region() {
        let mut map = MemoryMap::new();

        // Add RAM
        map.add_region(MemoryRegionType::Ram(RamRegion::new(0x0000, 0xFFFF, 0)));

        // Add I/O region (high priority)
        map.add_region(MemoryRegionType::Io(IoRegion::new(0xD000, 0xD0FF, ChipType::Gtia, 100)));

        // I/O region should be found
        if let Some(region) = map.find_region(0xD000) {
            match region {
                MemoryRegionType::Io(io) => {
                    assert_eq!(io.chip_type, ChipType::Gtia);
                }
                _ => panic!("Expected I/O region"),
            }
        } else {
            panic!("I/O region not found");
        }

        // I/O regions return None for read (no data storage)
        assert_eq!(map.read(0xD000), None);
    }
}

// Memory region types for flexible memory mapping
//
// This module provides the building blocks for configurable memory maps.
// Different 6502-based systems (Atari 800, 800XL, C64, Apple II) can use
// these region types to define their specific memory layouts.

/// Identifies which hardware chip handles I/O for a memory region
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChipType {
    Gtia,
    Pokey,
    Antic,
    Pia,
}

/// RAM region - writable memory
#[derive(Clone, Debug)]
pub struct RamRegion {
    pub start: u16,
    pub end: u16,
    pub data: Vec<u8>,
    pub priority: u8,
}

impl RamRegion {
    pub fn new(start: u16, end: u16, priority: u8) -> Self {
        let size = (end as usize - start as usize) + 1;
        RamRegion {
            start,
            end,
            data: vec![0; size],
            priority,
        }
    }

    pub fn contains(&self, addr: u16) -> bool {
        addr >= self.start && addr <= self.end
    }

    pub fn read(&self, addr: u16) -> u8 {
        if self.contains(addr) {
            self.data[(addr - self.start) as usize]
        } else {
            0xFF
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        if self.contains(addr) {
            self.data[(addr - self.start) as usize] = val;
        }
    }
}

/// ROM region - read-only memory
#[derive(Clone, Debug)]
pub struct RomRegion {
    pub start: u16,
    pub end: u16,
    pub data: Vec<u8>,
    pub priority: u8,
}

impl RomRegion {
    pub fn new(start: u16, data: Vec<u8>, priority: u8) -> Self {
        // Validate ROM fits in address space
        let end_addr = start as usize + data.len();
        if end_addr > 0x10000 {
            panic!("ROM region at ${:04X} with size {} bytes extends beyond address space (end=${:05X})",
                   start, data.len(), end_addr);
        }

        let end = (end_addr - 1) as u16;
        RomRegion {
            start,
            end,
            data,
            priority,
        }
    }

    pub fn contains(&self, addr: u16) -> bool {
        addr >= self.start && addr <= self.end
    }

    pub fn read(&self, addr: u16) -> u8 {
        if self.contains(addr) {
            self.data[(addr - self.start) as usize]
        } else {
            0xFF
        }
    }

    pub fn write(&mut self, _addr: u16, _val: u8) {
        // ROM writes are ignored
    }
}

/// Banked region - multiple banks of memory, only one visible at a time
#[derive(Clone, Debug)]
pub struct BankedRegion {
    pub start: u16,
    pub end: u16,
    pub banks: Vec<Vec<u8>>,  // Multiple banks of data
    pub active_bank: usize,    // Which bank is currently visible
    pub priority: u8,
    pub writable: bool,        // Can this banked region be written to?
}

impl BankedRegion {
    pub fn new(start: u16, end: u16, banks: Vec<Vec<u8>>, priority: u8, writable: bool) -> Self {
        BankedRegion {
            start,
            end,
            banks,
            active_bank: 0,
            priority,
            writable,
        }
    }

    pub fn contains(&self, addr: u16) -> bool {
        addr >= self.start && addr <= self.end
    }

    pub fn read(&self, addr: u16) -> u8 {
        if self.contains(addr) && self.active_bank < self.banks.len() {
            let offset = (addr - self.start) as usize;
            if offset < self.banks[self.active_bank].len() {
                return self.banks[self.active_bank][offset];
            }
        }
        0xFF
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        if self.writable && self.contains(addr) && self.active_bank < self.banks.len() {
            let offset = (addr - self.start) as usize;
            if offset < self.banks[self.active_bank].len() {
                self.banks[self.active_bank][offset] = val;
            }
        }
    }

    pub fn set_bank(&mut self, bank: usize) {
        if bank < self.banks.len() {
            self.active_bank = bank;
        }
    }
}

/// I/O region - routes to hardware chip, doesn't store data
#[derive(Clone, Copy, Debug)]
pub struct IoRegion {
    pub start: u16,
    pub end: u16,
    pub chip_type: ChipType,
    pub priority: u8,
}

impl IoRegion {
    pub fn new(start: u16, end: u16, chip_type: ChipType, priority: u8) -> Self {
        IoRegion {
            start,
            end,
            chip_type,
            priority,
        }
    }

    pub fn contains(&self, addr: u16) -> bool {
        addr >= self.start && addr <= self.end
    }
}

/// Unified memory region type
#[derive(Clone, Debug)]
pub enum MemoryRegionType {
    Ram(RamRegion),
    Rom(RomRegion),
    Banked(BankedRegion),
    Io(IoRegion),
}

impl MemoryRegionType {
    pub fn contains(&self, addr: u16) -> bool {
        match self {
            MemoryRegionType::Ram(r) => r.contains(addr),
            MemoryRegionType::Rom(r) => r.contains(addr),
            MemoryRegionType::Banked(r) => r.contains(addr),
            MemoryRegionType::Io(r) => r.contains(addr),
        }
    }

    pub fn priority(&self) -> u8 {
        match self {
            MemoryRegionType::Ram(r) => r.priority,
            MemoryRegionType::Rom(r) => r.priority,
            MemoryRegionType::Banked(r) => r.priority,
            MemoryRegionType::Io(r) => r.priority,
        }
    }

    pub fn read(&self, addr: u16) -> Option<u8> {
        match self {
            MemoryRegionType::Ram(r) => Some(r.read(addr)),
            MemoryRegionType::Rom(r) => Some(r.read(addr)),
            MemoryRegionType::Banked(r) => Some(r.read(addr)),
            MemoryRegionType::Io(_) => None,  // I/O doesn't store data
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) -> bool {
        match self {
            MemoryRegionType::Ram(r) => {
                r.write(addr, val);
                true
            }
            MemoryRegionType::Rom(r) => {
                r.write(addr, val);  // Ignored but returns true (handled)
                true
            }
            MemoryRegionType::Banked(r) => {
                r.write(addr, val);
                true
            }
            MemoryRegionType::Io(_) => false,  // I/O handled elsewhere
        }
    }
}

// Machine-specific configuration for different 6502 systems
//
// Each machine type has different memory layouts, ROM sizes, and banking schemes.
// This module centralizes those differences into declarative configurations.

use crate::banking::BankingScheme;
use crate::rom_scanner::RomDatabase;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MachineType {
    Atari800,
    Atari800XL,
}

pub struct MachineConfig {
    pub machine_type: MachineType,
    pub ram_size: usize,
    pub os_rom_path: Option<String>,
    pub os_rom_start: u16,
    pub os_rom_size: usize,
    pub basic_rom_path: Option<String>,
    pub basic_rom_size: usize,
    pub banking_scheme: BankingScheme,
}

impl MachineConfig {
    /// Atari 800 configuration
    /// - 48KB RAM ($0000-$BFFF)
    /// - OS-B ROM at $D800-$FFFF (10KB)
    /// - No BASIC ROM
    /// - No banking
    pub fn atari_800(rom_db: &RomDatabase) -> Self {
        let os_rom_path = rom_db.get_atari800_os();

        if os_rom_path.is_none() {
            eprintln!("Warning: Atari 800 OS-B ROM not found in roms/ directory");
            eprintln!("  Expected MD5: 4177f386a3bac989a981d3fe3388cb6c");
        }

        MachineConfig {
            machine_type: MachineType::Atari800,
            ram_size: 48 * 1024,  // 48KB
            os_rom_path,
            os_rom_start: 0xD800,
            os_rom_size: 10 * 1024,  // 10KB
            basic_rom_path: None,  // Atari 800 has no built-in BASIC
            basic_rom_size: 0,
            banking_scheme: BankingScheme::None,
        }
    }

    /// Atari 800XL configuration
    /// - 64KB RAM ($0000-$FFFF)
    /// - XL OS ROM at $C000-$FFFF (16KB)
    /// - BASIC ROM at $A000-$BFFF (8KB)
    /// - PORTB banking enabled
    pub fn atari_800xl(rom_db: &RomDatabase) -> Self {
        let os_rom_path = rom_db.get_atari800xl_os();
        let basic_rom_path = rom_db.get_basic();

        if os_rom_path.is_none() {
            eprintln!("Warning: Atari 800XL OS ROM not found in roms/ directory");
            eprintln!("  Expected MD5: 06daac977823773a3eea3422fd26a703");
        }

        if basic_rom_path.is_none() {
            eprintln!("Warning: BASIC ROM not found in roms/ directory");
            eprintln!("  Expected MD5 (Rev C): 0bac0c6a50104045d902df4503a4c30b");
            eprintln!("  Expected MD5 (Rev B): 04ea6a4e386601445ca5bfc8e37fb620");
            eprintln!("  Expected MD5 (Rev A): a4dc52536d526ecc51ea857b9fa2b90f");
        }

        MachineConfig {
            machine_type: MachineType::Atari800XL,
            ram_size: 64 * 1024,  // 64KB
            os_rom_path,
            os_rom_start: 0xC000,
            os_rom_size: 16 * 1024,  // 16KB
            basic_rom_path,
            basic_rom_size: 8 * 1024,  // 8KB
            banking_scheme: BankingScheme::None,  // No RAM banking on 800XL
        }
    }

    /// Get configuration for a specific machine type
    pub fn for_machine(machine_type: MachineType, rom_db: &RomDatabase) -> Self {
        match machine_type {
            MachineType::Atari800 => Self::atari_800(rom_db),
            MachineType::Atari800XL => Self::atari_800xl(rom_db),
        }
    }
}

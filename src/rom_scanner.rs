// ROM scanner - identifies ROM files by MD5 checksum
//
// Scans the roms/ directory and builds a database of known ROM files
// based on their MD5 signatures. This allows the emulator to automatically
// find the correct ROM files regardless of their filenames.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Known ROM types identified by MD5 checksum
#[derive(Debug, Clone, PartialEq)]
pub enum RomType {
    Atari800OsB,      // OS-B for Atari 800
    Atari800XlOs,     // OS for Atari 800XL
    BasicRevA,        // BASIC Rev A
    BasicRevB,        // BASIC Rev B
    BasicRevC,        // BASIC Rev C
}

/// ROM database - maps MD5 checksums to ROM types and file paths
pub struct RomDatabase {
    /// Map of MD5 hash -> (ROM type, file path)
    roms: HashMap<String, (RomType, PathBuf)>,
}

impl RomDatabase {
    /// Known ROM MD5 signatures
    const KNOWN_ROMS: &'static [(&'static str, RomType)] = &[
        ("4177f386a3bac989a981d3fe3388cb6c", RomType::Atari800OsB),
        ("06daac977823773a3eea3422fd26a703", RomType::Atari800XlOs),
        ("a4dc52536d526ecc51ea857b9fa2b90f", RomType::BasicRevA),
        ("04ea6a4e386601445ca5bfc8e37fb620", RomType::BasicRevB),
        ("0bac0c6a50104045d902df4503a4c30b", RomType::BasicRevC),
    ];

    /// Scan the roms/ directory and build the ROM database
    pub fn scan(roms_dir: &str) -> Self {
        let mut roms = HashMap::new();

        // Check if roms directory exists
        if !std::path::Path::new(roms_dir).exists() {
            eprintln!("Warning: ROMs directory '{}' not found", roms_dir);
            return RomDatabase { roms };
        }

        // Read all files in the roms directory
        match fs::read_dir(roms_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() {
                            if let Some((rom_type, md5_hash)) = Self::identify_rom(&entry.path()) {
                                println!("  Found: {:?} - {}", rom_type, entry.path().display());
                                roms.insert(md5_hash, (rom_type, entry.path()));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Could not read ROMs directory '{}': {}", roms_dir, e);
            }
        }

        RomDatabase { roms }
    }

    /// Identify a ROM file by computing its MD5 and checking against known ROMs
    fn identify_rom(path: &std::path::Path) -> Option<(RomType, String)> {
        // Read the file
        let data = fs::read(path).ok()?;

        // Compute MD5
        let digest = md5::compute(&data);
        let md5_hash = format!("{:x}", digest);

        // Check if this is a known ROM
        for (known_hash, rom_type) in Self::KNOWN_ROMS {
            if md5_hash == *known_hash {
                return Some((rom_type.clone(), md5_hash));
            }
        }

        None
    }

    /// Find a ROM file by type
    pub fn find_rom(&self, rom_type: &RomType) -> Option<&PathBuf> {
        self.roms
            .values()
            .find(|(rt, _)| rt == rom_type)
            .map(|(_, path)| path)
    }

    /// Get OS ROM path for Atari 800
    pub fn get_atari800_os(&self) -> Option<String> {
        self.find_rom(&RomType::Atari800OsB)
            .map(|p| p.to_string_lossy().to_string())
    }

    /// Get OS ROM path for Atari 800XL
    pub fn get_atari800xl_os(&self) -> Option<String> {
        self.find_rom(&RomType::Atari800XlOs)
            .map(|p| p.to_string_lossy().to_string())
    }

    /// Get BASIC ROM path (prefers Rev C, falls back to Rev B, then Rev A)
    pub fn get_basic(&self) -> Option<String> {
        // Prefer BASIC Rev C (latest)
        if let Some(path) = self.find_rom(&RomType::BasicRevC) {
            return Some(path.to_string_lossy().to_string());
        }
        // Fall back to Rev B
        if let Some(path) = self.find_rom(&RomType::BasicRevB) {
            return Some(path.to_string_lossy().to_string());
        }
        // Fall back to Rev A
        self.find_rom(&RomType::BasicRevA)
            .map(|p| p.to_string_lossy().to_string())
    }

    /// Check if we have the required ROMs for a given machine
    pub fn has_atari800_roms(&self) -> bool {
        self.get_atari800_os().is_some()
    }

    pub fn has_atari800xl_roms(&self) -> bool {
        self.get_atari800xl_os().is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rom_database_empty_dir() {
        let db = RomDatabase::scan("nonexistent_dir");
        assert!(db.roms.is_empty());
    }
}

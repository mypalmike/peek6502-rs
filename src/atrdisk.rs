// ATR Disk Image Handler
//
// Represents a single ATR disk image and handles sector reads.
// ATR is the standard Atari 8-bit disk image format.

use std::fs;
use crate::sio::{SioDevice, SioResponse};

pub struct AtrDisk {
    image_data: Vec<u8>,      // Entire ATR file in memory
    pub sector_size: u16,     // 128 or 256 (from ATR header)
    pub sector_count: u16,    // Total sectors on disk
    header_offset: usize,     // Where sector data starts (after 16-byte header)
    device_id: u8,            // SIO device ID (e.g., $31 for D1:)
}

impl AtrDisk {
    /// Load an ATR disk image from a file
    /// device_id: SIO device ID (e.g., 0x31 for D1:, 0x32 for D2:, etc.)
    pub fn new(path: &str, device_id: u8) -> Result<Self, String> {
        let data = fs::read(path)
            .map_err(|e| format!("Failed to read ATR file: {}", e))?;

        // Validate minimum size (16-byte header + at least one sector)
        if data.len() < 16 + 128 {
            return Err("ATR file too small".to_string());
        }

        // Validate magic number ($0296 little-endian)
        if data[0] != 0x96 || data[1] != 0x02 {
            return Err(format!(
                "Invalid ATR magic number: {:02X}{:02X} (expected 9602)",
                data[1], data[0]
            ));
        }

        // Parse sector size from header (bytes 4-5, little-endian)
        // 0 = 256, 128 = 128, 256 = 256
        let sector_size_raw = data[4] as u16 | ((data[5] as u16) << 8);
        let sector_size = match sector_size_raw {
            0 => 256,
            128 => 128,
            256 => 256,
            _ => return Err(format!("Invalid sector size: {}", sector_size_raw)),
        };

        // Calculate total disk size from header (bytes 2-3, in paragraphs)
        let paragraphs = data[2] as u16 | ((data[3] as u16) << 8);
        let data_size = (paragraphs as usize) * 16;

        // Calculate sector count
        let sector_count = Self::calculate_sector_count(data_size, sector_size);

        Ok(AtrDisk {
            image_data: data,
            sector_size,
            sector_count,
            header_offset: 16,
            device_id,
        })
    }

}

// Implement SioDevice trait
impl SioDevice for AtrDisk {
    fn device_id(&self) -> u8 {
        self.device_id
    }

    fn handle_command(&mut self, cmd: u8, aux1: u8, aux2: u8) -> SioResponse {
        match cmd {
            0x53 => self.get_status(),
            0x52 => self.read_sector(aux1, aux2),
            _ => SioResponse::Nak
        }
    }

    fn name(&self) -> String {
        format!("D{}: (ATR disk)", self.device_id - 0x30)
    }
}

// Private implementation methods
impl AtrDisk {
    /// Get Status command ($53)
    fn get_status(&self) -> SioResponse {
        // Status byte 0: Drive status bits
        // Bit 3 = 1: write protected
        // Bit 4 = 1: drive active/ready
        // Bit 5 = 1: double density (256-byte sectors)
        let mut status_byte_0 = 0x18;  // Drive active + write protected
        if self.sector_size == 256 {
            status_byte_0 |= 0x20;  // Set double density bit
        }

        // Status data (4 bytes)
        // Note: SioController will add the 'C' result byte and checksum
        let status_data = vec![
            status_byte_0,  // Drive status
            0xFF,           // Hardware status (FDC OK)
            0x60,           // Format timeout (bit 7 = 0)
            0x00,           // Unused
        ];

        SioResponse::CompleteWithData(status_data)
    }

    /// Read Sector command ($52)
    fn read_sector(&self, aux1: u8, aux2: u8) -> SioResponse {
        // Combine aux bytes into sector number (little-endian)
        let sector = aux1 as u16 | ((aux2 as u16) << 8);

        // Validate sector number
        if sector == 0 || sector > self.sector_count {
            return SioResponse::Error;
        }

        // Calculate offset and size for this sector
        let offset = self.sector_offset(sector);
        let size = self.get_sector_size(sector);

        // Check bounds
        if offset + size > self.image_data.len() {
            return SioResponse::Error;
        }

        // Extract sector data
        // Note: SioController will add the 'C' result byte and checksum
        let sector_data = self.image_data[offset..offset + size].to_vec();

        SioResponse::CompleteWithData(sector_data)
    }

    /// Calculate file offset for a sector number
    fn sector_offset(&self, sector: u16) -> usize {
        // Sectors are 1-based
        // Sectors 1-3 are always 128 bytes (boot sectors)
        // Sectors 4+ use the disk's sector size

        if sector <= 3 {
            // Boot sectors: 128 bytes each
            self.header_offset + ((sector - 1) as usize * 128)
        } else {
            // Data sectors
            let boot_size = 3 * 128;  // First 3 sectors
            let data_sectors = sector - 4;  // How many data sectors past sector 3
            self.header_offset + boot_size + (data_sectors as usize * self.sector_size as usize)
        }
    }

    /// Get the size of a specific sector
    fn get_sector_size(&self, sector: u16) -> usize {
        if sector <= 3 {
            128  // Boot sectors always 128 bytes
        } else {
            self.sector_size as usize
        }
    }

    /// Calculate sector count from disk size and sector size
    fn calculate_sector_count(data_size: usize, sector_size: u16) -> u16 {
        if sector_size == 128 {
            // Single density: all sectors are 128 bytes
            (data_size / 128) as u16
        } else {
            // Double density: 3 boot sectors (128 bytes) + remaining (256 bytes)
            let boot_size = 3 * 128;
            if data_size <= boot_size {
                (data_size / 128) as u16
            } else {
                3 + ((data_size - boot_size) / 256) as u16
            }
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sector_offset_sd() {
        // Create a minimal valid ATR header for single density (128 byte sectors)
        let mut data = vec![0; 16 + 720 * 128];  // SD disk (720 sectors)
        data[0] = 0x96;  // Magic
        data[1] = 0x02;
        data[2] = 0x80;  // Paragraphs low ($5A80 = 720 * 128 / 16)
        data[3] = 0x5A;  // Paragraphs high
        data[4] = 0x80;  // Sector size = 128
        data[5] = 0x00;

        let disk = AtrDisk {
            image_data: data,
            sector_size: 128,
            sector_count: 720,
            header_offset: 16,
            device_id: 0x31,  // D1:
        };

        assert_eq!(disk.sector_offset(1), 16);           // First sector
        assert_eq!(disk.sector_offset(2), 16 + 128);     // Second sector
        assert_eq!(disk.sector_offset(3), 16 + 256);     // Third sector
        assert_eq!(disk.sector_offset(4), 16 + 384);     // Fourth sector
    }

    #[test]
    fn test_sector_offset_dd() {
        // Create a minimal valid ATR header for double density (256 byte sectors)
        let mut data = vec![0; 16 + 384 + 717 * 256];  // DD disk (720 sectors)
        data[0] = 0x96;  // Magic
        data[1] = 0x02;
        data[2] = 0xD0;  // Paragraphs low
        data[3] = 0x2C;  // Paragraphs high
        data[4] = 0x00;  // Sector size = 256 (encoded as 0)
        data[5] = 0x01;

        let disk = AtrDisk {
            image_data: data,
            sector_size: 256,
            sector_count: 720,
            header_offset: 16,
            device_id: 0x31,  // D1:
        };

        assert_eq!(disk.sector_offset(1), 16);           // First boot sector (128)
        assert_eq!(disk.sector_offset(2), 16 + 128);     // Second boot sector (128)
        assert_eq!(disk.sector_offset(3), 16 + 256);     // Third boot sector (128)
        assert_eq!(disk.sector_offset(4), 16 + 384);     // First data sector (256)
        assert_eq!(disk.sector_offset(5), 16 + 384 + 256); // Second data sector (256)
    }

    #[test]
    fn test_calculate_sector_count() {
        // Single density: 720 sectors * 128 bytes = 92160 bytes
        assert_eq!(AtrDisk::calculate_sector_count(92160, 128), 720);

        // Double density: 3 * 128 + 717 * 256 = 384 + 183552 = 183936 bytes
        assert_eq!(AtrDisk::calculate_sector_count(183936, 256), 720);
    }
}

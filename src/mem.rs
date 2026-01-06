use std::fs::File;
use std::io::Read;

pub struct Mem {
    pub ram: [u8; 0x10000], // 64K // 1024],
    pub rom: [u8; 0x10000],
    pub split: u16,
}

impl Mem {
    pub fn new(split: u16, run_func_tests: bool) -> Mem {
        let mut new_mem = Mem {
            ram: [0x00_u8; 0x10000],
            rom: [0x00_u8; 0x10000],
            split: split,
        };

        // Initialize with test code.
        if run_func_tests {
            let f = File::open("6502_functional_test.bin");
            let mut f = match f {
                Ok(file) => file,
                Err(_error) => panic!("Could not open test file"),
            };

            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer).expect("Failed to read test file");

            new_mem.ram[0..0x10000].copy_from_slice(&buffer);
        } else {
            // Load Atari OS ROM at $C000-$FFFF (16KB)
            new_mem.load_os_rom();
        }

        new_mem
    }

    /// Load Atari OS ROM into memory at $C000-$FFFF
    fn load_os_rom(&mut self) {
        println!("ROM loading...");
        if let Ok(mut f) = File::open("roms/Atari OS-B NTSC.ROM") {
            let mut buffer = Vec::new();
            if f.read_to_end(&mut buffer).is_ok() && buffer.len() == 0x2800 {
                // Atari 800 OS B ROM is 10240 bytes, load at $D800-$FFFF
                self.rom[0xD800..0x10000].copy_from_slice(&buffer[0..0x2800]);
                println!("ROM loaded");
            } else {
                println!("Buffer issue loading ROM, buffer len was {}", buffer.len());
            }
        } else {
            println!("Failed to load ROM");
        }
        // If ROM file not found, continue with empty ROM (font rendering will fail gracefully)
    }

    pub fn get_byte(&self, addr: u16) -> u8 {
        if addr < self.split || self.split == 0 {
            self.ram[addr as usize]
        } else {
            self.rom[addr as usize]
        }
    }

    pub fn get_word(&self, addr: u16) -> u16 {
        let addr1 = self.get_byte(addr) as u16;
        let addr2 = (self.get_byte(addr + 1) as u16) << 8;
        addr1 | addr2
    }

    pub fn set_byte(&mut self, addr: u16, val: u8) {
        if addr >= self.split && self.split != 0 {
            panic!("Can't write to ROM");
        }

        self.ram[addr as usize] = val;
    }

    pub fn set_word(&mut self, addr: u16, val: u16) {
        let lo_byte = (val & 0xff) as u8;
        let hi_byte = (val >> 8) as u8;
        self.set_byte(addr, lo_byte);
        self.set_byte(addr + 1, hi_byte);
    }
}

use std::fs::File;
use std::io::Read;

pub struct Mem {
    ram : [u8; 0x10000], // 64K // 1024],
    rom : [u8; 1024],
}

impl Mem {
    pub fn new() -> Mem {
        let mut new_mem = Mem {
            ram : [0x00_u8; 0x10000],
            rom : [0x00_u8; 1024],
        };

        // a8000: lda #$05   ; A9 05
        // a8002: adc #$06   ; 69 06
        // a8004: tax        ; AA
        // a8005: stx $0001  ; 86 01
        // a8007: lda $0001  ; A5 01
        // a8009: jmp a8009  ; 4C 09 80
        // let commands : [u8; 12] = [0xa9, 0x05, 0x69, 0x06, 0xaa, 0x86, 0x01, 0xa5, 0x01, 0x4c, 0x09, 0x80];

        // Initialize with some test 6502 code.
        // new_mem.rom[0..].clone_from_slice(&commands);
        // new_mem.rom[0..12].copy_from_slice(&commands);

        // Initialize with test code.
        let f = File::open("6502_functional_test.bin");
        let mut f = match f {
            Ok(file) => file,
            Err(error) => panic!("Could not open test file"),
        };

        let mut buffer = Vec::new();
//        let mut buffer = [0_u8; 0x10000];
        f.read_to_end(&mut buffer);

        new_mem.ram[0..0x10000].copy_from_slice(&buffer);

        new_mem
    }

    pub fn get_byte(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
        // if addr >= 0x8000 {
        //     self.rom[(addr - 0x8000) as usize]
        // } else {
        //     self.ram[addr as usize]
        // }
    }

    pub fn get_word(&self, addr: u16) -> u16 {
        let addr1 = self.get_byte(addr) as u16;
        let addr2 = (self.get_byte(addr + 1) as u16) << 8;
        addr1 | addr2
    }

    pub fn set_byte(&mut self, addr: u16, val: u8) {
        // if addr >= 0x8000 {
        //     panic!("Can't write to ROM");
        // }

        self.ram[addr as usize] = val;
    }

    pub fn set_word(&mut self, addr: u16, val: u16) {
        let lo_byte = (val & 0xff) as u8;
        let hi_byte = (val >> 8) as u8;
        self.set_byte(addr, lo_byte);
        self.set_byte(addr + 1, hi_byte);
    }
}


// fn test_simple() {
//     // a8000: lda #$05   ; A9 05
//     // a8002: adc #$06   ; 69 06
//     // a8004: tax        ; AA
//     // a8005: stx $0001  ; 86 01
//     // a8007: lda $0001  ; A5 01
//     // a8009: jmp a8009  ; 4C 09 80




// }

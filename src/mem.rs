

pub struct Mem {
    ram : [u8; 1024],
    rom : [u8; 1024],
}

impl Mem {
    pub fn new() -> Mem {
        let mut new_mem = Mem {
            ram : [0x00; 1024], // Stupid 1K for now
            rom : [0x00; 1024],
        };

        // a8000: lda #$05   ; A9 05
        // a8002: adc #$06   ; 69 06
        // a8004: tax        ; AA
        // a8005: stx $0001  ; 86 01
        // a8007: lda $0001  ; A5 01
        // a8009: jmp a8009  ; 4C 09 80
        let commands : [u8; 12] = [0xa9, 0x05, 0x69, 0x06, 0xaa, 0x86, 0x01, 0xa5, 0x01, 0x4c, 0x09, 0x80];

        // Initialize with some test 6502 code.
        // new_mem.rom[0..].clone_from_slice(&commands);
        new_mem.rom[0..12].copy_from_slice(&commands);

        new_mem
    }

    pub fn get_byte(&self, addr : u16) -> u8 {
        if addr >= 0x8000 {
            self.rom[(addr - 0x8000) as usize]
        } else {
            self.ram[addr as usize]
        }
    }

    pub fn get_word(&self, addr : u16) -> u16 {
        let addr1 = self.get_byte(addr) as u16;
        let addr2 = (self.get_byte(addr + 1) as u16) << 8;
        addr1 | addr2
    }

    pub fn set_byte(&mut self, addr : u16, val : u8) {
        if addr >= 0x8000 {
            panic!("Can't write to ROM");
        }

        self.ram[addr as usize] = val;
    }

    // pub fn put_word(addr : u16, val : u16) {

    // }
}


// fn test_simple() {
//     // a8000: lda #$05   ; A9 05
//     // a8002: adc #$06   ; 69 06
//     // a8004: tax        ; AA
//     // a8005: stx $0001  ; 86 01
//     // a8007: lda $0001  ; A5 01
//     // a8009: jmp a8009  ; 4C 09 80


    

// }

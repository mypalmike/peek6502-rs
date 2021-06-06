use std::fs::File;
use std::io::Read;
use std::rc::Rc;
use std::cell::RefCell;

use crate::antic::Antic;
use crate::addressable::Addressable;

pub struct Mem {
    pub ram: MemorySequence,
    pub rom: MemorySequence,
    antic: Rc<RefCell<Antic>>,
}

impl Mem {
    pub fn new(antic: Rc<RefCell<Antic>>) -> Mem {
        let mut new_mem = Mem {
            ram: MemorySequence::new(54 * 1024, 0, true),
            rom: MemorySequence::new(10 * 1024, 54*1024, false),
            // ram: [0x00_u8; 0x10000],
            // rom: [0x00_u8; 0x10000],
            antic: antic,
            // split: split,
        };

        // Initialize with OS code.
        // if load_atari_osb {
        let f = File::open("ATARIOSB.ROM");
        let mut f = match f {
            Ok(file) => file,
            Err(error) => panic!("Could not open ATARIOSB.ROM"),
        };

        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer);

        new_mem.rom.init(&buffer);




        // }

        // // Initialize with test code.
        // if run_func_tests {
        //     let f = File::open("6502_functional_test.bin");
        //     let mut f = match f {
        //         Ok(file) => file,
        //         Err(error) => panic!("Could not open test file"),
        //     };

        //     let mut buffer = Vec::new();
        //     f.read_to_end(&mut buffer);

        //     new_mem.ram[0..0x10000].copy_from_slice(&buffer);
        // }

        new_mem
    }

    // pub fn tick(&mut self) {
    //     // tick(self, &mut self.antic);

    //     self.antic.tick(self);
    //     // let mem = &self;
    //     // let antic = &mut mem.antic;
    //     // antic.tick(self);
    // }
// }
}

impl Addressable for Mem {
    fn get_byte(&self, addr: u16) -> u8 {
        if addr >= 0xd400 && addr < 0xd420 {
            self.antic.borrow().get_byte(addr)
        } else if addr < 54 * 1024 {
            self.ram.get_byte(addr)
        } else {
            self.rom.get_byte(addr)
        }
    }

    fn set_byte(&mut self, addr: u16, val: u8) {
        if addr >= 0xd400 && addr < 0xd420 {
            self.antic.borrow_mut().set_byte(addr, val);
        } else if addr < 54 * 1024 {
            self.ram.set_byte(addr, val);
        } else {
            self.rom.set_byte(addr, val);
        }
    }
}

//     pub fn get_word(&self, addr: u16) -> u16 {
//         let addr1 = self.get_byte(addr) as u16;
//         let addr2 = (self.get_byte(addr + 1) as u16) << 8;
//         addr1 | addr2
//     }

//     pub fn set_word(&mut self, addr: u16, val: u16) {
//         let lo_byte = (val & 0xff) as u8;
//         let hi_byte = (val >> 8) as u8;
//         self.set_byte(addr, lo_byte);
//         self.set_byte(addr + 1, hi_byte);
//     }
// }

// pub fn mem_tick(mem: &mut Mem) { //, antic: &mut Antic) {
//     antic_tick(&mut mem.antic, mem);
//     // mem.antic.tick(mem);
// }

// pub fn antic_tick(antic: &mut Antic, mem: &mut Mem) {
//     antic.tick(mem);
// }


    // pub fn tick(&mut self) {
    //     // tick(self, &mut self.antic);

    //     self.antic.tick(self);
    //     // let mem = &self;
    //     // let antic = &mut mem.antic;
    //     // antic.tick(self);
    // }


pub struct MemorySequence {
    data: Vec<u8>,
    offset: u16,
    writable: bool,
}

impl MemorySequence {
    pub fn new(size: u32, offset: u16, writable: bool) -> MemorySequence {
        MemorySequence {
            data: vec![0_u8; size as usize],
            offset: offset,
            writable: writable,
        }
    }

    pub fn init(&mut self, buffer: &Vec<u8>) {
        self.data[0..(65535 - self.offset + 1) as usize].copy_from_slice(&buffer);
    }
}

impl Addressable for MemorySequence {
    fn get_byte(&self, addr: u16) -> u8 {
        self.data[(addr - self.offset) as usize]
    }

    fn set_byte(&mut self, addr: u16, val: u8) {
        // TODO : What happens on real hardware? Bus error?
        if !self.writable {
            panic!("Unwritable memory address {:04x}", addr);
        }

        self.data[(addr - self.offset) as usize] = val;
    }

}

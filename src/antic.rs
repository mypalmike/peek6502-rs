
use std::rc::Rc;
use std::cell::RefCell;

use crate::cpu_pins::CpuPins;
use crate::mem::Mem;
use crate::display::Display;
use crate::addressable::Addressable;

pub const FIRST: u16 = 0xd400;
pub const LAST: u16 = 0xd40f;
pub const WRAP_LAST: u16 = 0xd41f;
const WRAP_OFFSET: u16 = WRAP_LAST - LAST;

const DMACTL: u16 = 0xd400;
const CHACTL: u16 = 0xd401;
const DLISTL: u16 = 0xd402;
const DLISTH: u16 = 0xd403;
const HSCROL: u16 = 0xd404;
const VSCROL: u16 = 0xd405;
const PMBASE: u16 = 0xd407;
const CHBASE: u16 = 0xd409;
const WSYNC: u16 = 0xd40a;
const VCOUNT: u16 = 0xd40b;
const PENH: u16 = 0xd40c;
const PENV: u16 = 0xd40d;
const NMIEN: u16 = 0xd40e;
const NMIRES: u16 = 0xd40f;
const NMIST: u16 = 0xd40f;

const COLOR_CLOCKS_PER_SCANLINE: u32 = 228;
const SCANLINES: u32 = 262;
const SCANLINE_VBLANK_START: u32 = 248;

const NMI_MASK_DLI: u8 = 0x80;
const NMI_MASK_VBI: u8 = 0x40;
const NMI_MASK_RESET: u8 = 0x20;


pub struct Antic {
    // mem: RefCell<Mem>,

    // CPU-accessible registers.
    dmactl: u8,
    chactl: u8,
    dlistl: u8,
    dlisth: u8,
    hscrol: u8,
    vscrol: u8,
    pmbase: u8,
    chbase: u8,
    wsync: u8,
    vcount: u8,
    penh: u8,
    penv: u8,
    nmien: u8,
    nmires: u8,
    nmist: u8,

    cpu_pins: Rc<RefCell<CpuPins>>,
    
    // Internal registers.
    instruction: u8,
    memory_scan: u16,

    // Sequence state.
    color_clock: u32,
    scanline: u32,
}

impl Antic {
    // pub fn new(mem: RefCell<Mem>) -> Antic {
    pub fn new(cpu_pins: Rc<RefCell<CpuPins>>) -> Antic {
        Antic {
            // mem: mem,

            dmactl: 0,
            chactl: 0,
            dlistl: 0,
            dlisth: 0,
            hscrol: 0,
            vscrol: 0,
            pmbase: 0,
            chbase: 0,
            wsync: 0,
            vcount: 0,
            penh: 0,
            penv: 0,
            nmien: 0,
            nmires: 0,
            nmist: 0,

            cpu_pins: cpu_pins,

            instruction: 0,
            memory_scan: 0,

            color_clock: 0,
            scanline: 0,
        }
    }

    pub fn tick(&mut self, mem: &mut Mem, display: &mut Display) {
        // For a 1-to-1 CPU to Antic tick ratio, a tick is 2 color clocks.
        self.color_clock += 2;

        if self.color_clock >= COLOR_CLOCKS_PER_SCANLINE {
            self.color_clock = 0;
            self.scanline += 1;

            self.check_set_vblank();

        } else if self.scanline >= SCANLINES {
            self.draw_frame(mem, display);
            self.scanline = 0;
        }

        self.vcount = (self.scanline >> 1) as u8;
        // println!("antic tick scanline {}, color_clock {}", self.scanline, self.color_clock);
    }


    fn check_set_vblank(&mut self) {
        if self.scanline == SCANLINE_VBLANK_START {
            if self.nmien & NMI_MASK_VBI == NMI_MASK_VBI {
                self.cpu_pins.nmi = true;

                // TODO : Should "or" this?
                self.nmist = NMI_MASK_VBI;
            }
        }        
    }

    fn map_addr(&self, addr: u16) -> u16 {
        if addr >= FIRST && addr <= LAST {
            addr
        } else if addr >= LAST + 1 && addr <= WRAP_LAST {
            addr - WRAP_OFFSET
        } else {
            panic!("Antic get_byte out of range {}", addr);
        }
    }
// }

// impl Addressable for Antic {

    // According to some documentation, in real hardware, reading a write-only reg returns 0xff
    pub fn get_byte(&self, addr: u16) -> u8 {
        let mapped_addr = self.map_addr(addr);

        match mapped_addr {
            VCOUNT => self.vcount,
            PENH => self.penh,
            PENV => self.penv,
            NMIST => self.nmist,
            _ => 0xff,
        }
    }

    pub fn set_byte(&mut self, addr: u16, val: u8) {
        println!("Antic addr {:04x} set_byte with {:02}", addr, val);
        let mapped_addr = self.map_addr(addr);

        match mapped_addr {
            DMACTL => self.dmactl = val,
            CHACTL => self.chactl = val,
            DLISTL => self.dlistl = val,
            DLISTH => self.dlisth = val,
            HSCROL => self.hscrol = val,
            VSCROL => self.vscrol = val,
            PMBASE => self.pmbase = val,
            CHBASE => self.chbase = val,
            WSYNC => self.wsync = val,
            NMIEN => self.nmien = val,
            NMIRES => {
                self.nmist = 0;
                self.clear_nmi();  // ? Antic doc suggests this?
            },
            _ => {}, // TODO : Unused, but what do real machines do?
        }
    }

    fn initial_dl_counter(&self) -> u16 {
        ((self.dlisth as u16) << 8) | (self.dlistl as u16)
    }

    fn set_nmi(&mut self) {
        self.cpu_pins.borrow().nmi = true;
    }

    fn clear_nmi(&mut self) {
        self.cpu_pins.borrow().nmi = true;
    }

    pub fn draw_frame(&mut self, mem: &mut Mem, display: &mut Display) {
        let mut dl_counter = self.initial_dl_counter();
        let mut memory_scan_counter = 0x00_u16;
        let mut line = 0_u16;

        // Set up character set.
        // TODO: CHBASE can change during screen drawing, so this needs to be
        // managed when CHBASE changes for accurate timing.
        let mut char_addr = (self.chbase as u16) << 8;

        println!("chbase is {:04x}", (self.chbase as u16) << 8);

        for char_index in 0..0x80 {
            display.set_char_texture(char_index, mem.get_8_bytes(char_addr));
            char_addr += 8;
        }

        display.draw_charset();

        // loop {
        //     let opcode = mem.get_byte(dl_counter);
        //     dl_counter += 1;
        //     match opcode & 0x0f {  // 4 bits determine opcode
        //         0x00 => {
        //             // Blank
        //             let lines = opcode & 0x70;
        //             line += lines;
        //         },
        //         0x01 => {
        //             // Jump
        //             dl_counter = mem.get_word();
        //         },
        //         _ => {
        //             // Display mode
        //             let display_type = opcode & 0x0f;
        //             let load_msc = opcode & 0x40 == 0x40;

        //             if load_msc {
        //                 memory_scan_counter = mem.get_word(dl_counter);
        //                 dl_counter += 2;
        //             }

        //             // TODO : Deal with all graphics modes.
        //             // Assume antic graphics mode 2 for now.
        //             let curr_addr = memory_scan_counter;
        //             for ch_idx in 0..40 {
        //                 let ch = mem.get_byte(ch_idx + curr_addr) & 0x7f;
        //                 display.render_texture(ch, ch_idx * 8, line);
        //             }

        //             line += 8;
        //         }
        //     }

        //     if line >= playfield_lines {
        //         break;
        //     }
        // }
    }
}

pub fn antic_tick(antic: Rc<RefCell<Antic>>, mem: Rc<RefCell<Mem>>, display: Rc<RefCell<Display>>) {
    antic.borrow_mut().tick(&mut mem.borrow_mut(), &mut display.borrow_mut());
}

use std::collections::HashSet;
use std::io;

use crate::mem::Mem;
use crate::cpu::Cpu;

extern crate hex;


#[derive(Clone, Copy, Debug)]
pub enum Op {
    ADC, AND, ASL, BCC, BCS, BEQ, BIT, BMI, BNE, BPL, BRK, BVC, BVS, CLC,
    CLD, CLI, CLV, CMP, CPX, CPY, DEC, DEX, DEY, EOR, HLT, INC, INX, INY, JMP,
    JSR, LDA, LDX, LDY, LSR, NOP, ORA, PHA, PHP, PLA, PLP, ROL, ROR, RTI,
    RTS, SBC, SEC, SED, SEI, SLO, STA, STX, STY, TAX, TAY, TSX, TXA, TXS, TYA,
    // Unofficial
    ALR, ANC, ARR, AHX, AXS, DCP, ISC, LAS, LAX, RLA, RRA, SAX, SHX, SHY, SRE, TAS, XAA,
}

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    ABS, ABX, ABY, IMM, IMP, IND, IZX, IZY, REL, ZP, ZPA, ZPX, ZPY,
}

pub struct Debugger {
    show_state: bool,
    show_disassembly: bool,
    running: bool,
    breakpoints: HashSet<u16>,
    opcodes: [(Op, Mode); 256],
}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger {
            show_state: false,
            show_disassembly: false,
            running: false,
            breakpoints: HashSet::new(),
            opcodes: [
                // 0x00 - 0x0F
                (Op::BRK, Mode::IMP), (Op::ORA, Mode::IZX), (Op::HLT, Mode::IMP), (Op::SLO, Mode::IZX),
                (Op::NOP, Mode::ZP),  (Op::ORA, Mode::ZP),  (Op::ASL, Mode::ZP),  (Op::SLO, Mode::ZP),
                (Op::PHP, Mode::IMP), (Op::ORA, Mode::IMM), (Op::ASL, Mode::IMP), (Op::ANC, Mode::IMM),
                (Op::NOP, Mode::ABS), (Op::ORA, Mode::ABS), (Op::ASL, Mode::ABS), (Op::SLO, Mode::ABS),

                // 0x10 - 0x1F
                (Op::BPL, Mode::REL), (Op::ORA, Mode::IZY), (Op::HLT, Mode::IMP), (Op::SLO, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::ORA, Mode::ZPX), (Op::ASL, Mode::ZPX), (Op::SLO, Mode::ZPX),
                (Op::CLC, Mode::IMP), (Op::ORA, Mode::ABY), (Op::NOP, Mode::IMP), (Op::SLO, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::ORA, Mode::ABX), (Op::ASL, Mode::ABX), (Op::SLO, Mode::ABX),

                // 0x20 - 0x2F
                (Op::JSR, Mode::ABS), (Op::AND, Mode::IZX), (Op::HLT, Mode::IMP), (Op::RLA, Mode::IZX),
                (Op::BIT, Mode::ZP),  (Op::AND, Mode::ZP),  (Op::ROL, Mode::ZP),  (Op::RLA, Mode::ZP),
                (Op::PLP, Mode::IMP), (Op::AND, Mode::IMM), (Op::ROL, Mode::IMP), (Op::ANC, Mode::IMM),
                (Op::BIT, Mode::ABS), (Op::AND, Mode::ABS), (Op::ROL, Mode::ABS), (Op::RLA, Mode::ABS),

                // 0x30 - 0x3F
                (Op::BMI, Mode::REL), (Op::AND, Mode::IZY), (Op::HLT, Mode::IMP), (Op::RLA, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::AND, Mode::ZPX), (Op::ROL, Mode::ZPX), (Op::RLA, Mode::ZPX),
                (Op::SEC, Mode::IMP), (Op::AND, Mode::ABY), (Op::NOP, Mode::IMP), (Op::RLA, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::AND, Mode::ABX), (Op::ROL, Mode::ABX), (Op::RLA, Mode::ABX),

                // 0x40 - 0x4F
                (Op::RTI, Mode::IMP), (Op::EOR, Mode::IZX), (Op::HLT, Mode::IMP), (Op::SRE, Mode::IZX),
                (Op::NOP, Mode::ZP),  (Op::EOR, Mode::ZP),  (Op::LSR, Mode::ZP),  (Op::SRE, Mode::ZP),
                (Op::PHA, Mode::IMP), (Op::EOR, Mode::IMM), (Op::LSR, Mode::IMP), (Op::ALR, Mode::IMM),
                (Op::JMP, Mode::ABS), (Op::EOR, Mode::ABS), (Op::LSR, Mode::ABS), (Op::SRE, Mode::ABS),

                // 0x50 - 0x5F
                (Op::BVC, Mode::REL), (Op::EOR, Mode::IZY), (Op::HLT, Mode::IMP), (Op::SRE, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::EOR, Mode::ZPX), (Op::LSR, Mode::ZPX), (Op::SRE, Mode::ZPX),
                (Op::CLI, Mode::IMP), (Op::EOR, Mode::ABY), (Op::NOP, Mode::IMP), (Op::SRE, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::EOR, Mode::ABX), (Op::LSR, Mode::ABX), (Op::SRE, Mode::ABX),

                // 0x60 - 0x6F
                (Op::RTS, Mode::IMP), (Op::ADC, Mode::IZX), (Op::HLT, Mode::IMP), (Op::RRA, Mode::IZX),
                (Op::NOP, Mode::ZP),  (Op::ADC, Mode::ZP),  (Op::ROR, Mode::ZP),  (Op::RRA, Mode::ZP),
                (Op::PLA, Mode::IMP), (Op::ADC, Mode::IMM), (Op::ROR, Mode::IMP), (Op::ARR, Mode::IMM),
                (Op::JMP, Mode::IND), (Op::ADC, Mode::ABS), (Op::ROR, Mode::ABS), (Op::RRA, Mode::ABS),

                // 0x70 - 0x7F
                (Op::BVS, Mode::REL), (Op::ADC, Mode::IZY), (Op::HLT, Mode::IMP), (Op::RRA, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::ADC, Mode::ZPX), (Op::ROR, Mode::ZPX), (Op::RRA, Mode::ZPX),
                (Op::SEI, Mode::IMP), (Op::ADC, Mode::ABY), (Op::NOP, Mode::IMP), (Op::RRA, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::ADC, Mode::ABX), (Op::ROR, Mode::ABX), (Op::RRA, Mode::ABX),

                // 0x80 - 0x8F
                (Op::NOP, Mode::IMM), (Op::STA, Mode::IZX), (Op::NOP, Mode::IMM), (Op::SAX, Mode::IZX),
                (Op::STY, Mode::ZP),  (Op::STA, Mode::ZP),  (Op::STX, Mode::ZP),  (Op::SAX, Mode::ZP),
                (Op::DEY, Mode::IMP), (Op::NOP, Mode::IMM), (Op::TXA, Mode::IMP), (Op::XAA, Mode::IMM),
                (Op::STY, Mode::ABS), (Op::STA, Mode::ABS), (Op::STX, Mode::ABS), (Op::SAX, Mode::ABS),

                // 0x90 - 0x9F
                (Op::BCC, Mode::REL), (Op::STA, Mode::IZY), (Op::HLT, Mode::IMP), (Op::AHX, Mode::IZY),
                (Op::STY, Mode::ZPX), (Op::STA, Mode::ZPX), (Op::STX, Mode::ZPY), (Op::SAX, Mode::ZPY),
                (Op::TYA, Mode::IMP), (Op::STA, Mode::ABY), (Op::TXS, Mode::IMP), (Op::TAS, Mode::ABY),
                (Op::SHY, Mode::ABX), (Op::STA, Mode::ABX), (Op::SHX, Mode::ABY), (Op::AHX, Mode::ABY),

                // 0xA0 - 0xAF
                (Op::LDY, Mode::IMM), (Op::LDA, Mode::IZX), (Op::LDX, Mode::IMM), (Op::LAX, Mode::IZX),
                (Op::LDY, Mode::ZP),  (Op::LDA, Mode::ZP),  (Op::LDX, Mode::ZP),  (Op::LAX, Mode::ZP),
                (Op::TAY, Mode::IMP), (Op::LDA, Mode::IMM), (Op::TAX, Mode::IMP), (Op::LAX, Mode::IMM),
                (Op::LDY, Mode::ABS), (Op::LDA, Mode::ABS), (Op::LDX, Mode::ABS), (Op::LAX, Mode::ABS),

                // 0xB0 - 0xBF
                (Op::BCS, Mode::REL), (Op::LDA, Mode::IZY), (Op::HLT, Mode::IMP), (Op::LAX, Mode::IZY),
                (Op::LDY, Mode::ZPX), (Op::LDA, Mode::ZPX), (Op::LDX, Mode::ZPY), (Op::LAX, Mode::ZPY),
                (Op::CLV, Mode::IMP), (Op::LDA, Mode::ABY), (Op::TSX, Mode::IMP), (Op::LAS, Mode::ABY),
                (Op::LDY, Mode::ABX), (Op::LDA, Mode::ABX), (Op::LDX, Mode::ABX), (Op::LAX, Mode::ABX),

                // 0xC0 - 0xCF
                (Op::CPY, Mode::IMM), (Op::CMP, Mode::IZX), (Op::NOP, Mode::IMM), (Op::DCP, Mode::IZX),
                (Op::CPY, Mode::ZP),  (Op::CMP, Mode::ZP),  (Op::DEC, Mode::ZP),  (Op::DCP, Mode::ZP),
                (Op::INY, Mode::IMP), (Op::CMP, Mode::IMM), (Op::DEX, Mode::IMP), (Op::AXS, Mode::IMM),
                (Op::CPY, Mode::ABS), (Op::CMP, Mode::ABS), (Op::DEC, Mode::ABS), (Op::DCP, Mode::ABS),

                // 0xD0 - 0xDF
                (Op::BNE, Mode::REL), (Op::CMP, Mode::IZY), (Op::HLT, Mode::IMP), (Op::DCP, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::CMP, Mode::ZPX), (Op::DEC, Mode::ZPX), (Op::DCP, Mode::ZPX),
                (Op::CLD, Mode::IMP), (Op::CMP, Mode::ABY), (Op::NOP, Mode::IMP), (Op::DCP, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::CMP, Mode::ABX), (Op::DEC, Mode::ABX), (Op::DCP, Mode::ABX),

                // 0xE0 - 0xEF
                (Op::CPX, Mode::IMM), (Op::SBC, Mode::IZX), (Op::NOP, Mode::IMP), (Op::ISC, Mode::IZX),
                (Op::CPX, Mode::IMP), (Op::SBC, Mode::IZX), (Op::INC, Mode::IMP), (Op::ISC, Mode::IZX),
                (Op::INX, Mode::IMP), (Op::SBC, Mode::IZX), (Op::NOP, Mode::IMP), (Op::SBC, Mode::IZX),
                (Op::CPX, Mode::IMP), (Op::SBC, Mode::IZX), (Op::INC, Mode::IMP), (Op::ISC, Mode::IZX),

                // 0xF0 - 0xFF
                (Op::BEQ, Mode::REL), (Op::SBC, Mode::IZY), (Op::HLT, Mode::IMP), (Op::ISC, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::SBC, Mode::ZPX), (Op::INC, Mode::ZPX), (Op::ISC, Mode::ZPX),
                (Op::SED, Mode::IMP), (Op::SBC, Mode::ABY), (Op::NOP, Mode::IMP), (Op::ISC, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::SBC, Mode::ABX), (Op::INC, Mode::ABX), (Op::ISC, Mode::ABX),
            ],
        }
    }

    pub fn tick(&mut self, cpu: &mut Cpu, mem: &mut Mem) {
        if self.running {
            self.cpu_tick(cpu, mem);
            if self.breakpoints.contains(&cpu.pc) {
                self.running = false;
            }
        } else {
            let mut input = String::new();
            io::stdin().read_line(&mut input);
            let command: Vec<&str> = input.trim().split(' ').collect();

            if command[0] == "ss" {
                self.show_state = !self.show_state;
                println!("Show State {}", self.show_state);
            }
            if command[0] == "sd" {
                self.show_disassembly = !self.show_disassembly;
                println!("Show Disassembly {}", self.show_disassembly);
            }
            if command[0] == "bp" {
                match hex::decode(command[1]) {
                    Ok(addr_vec) => {
                        let hi = (addr_vec[0] as u16) << 8;
                        let lo = addr_vec[1] as u16;
                        let addr = hi | lo;
                        self.breakpoints.insert(addr);
                        println!("Breakpoint added 0x{:04x}", addr);
                    },
                    Err(e) => {}
                }
            }
            if command[0] == "m" {
                let addr = match hex::decode(command[1]) {
                    Ok(addr_vec) => {
                        let hi = (addr_vec[0] as u16) << 8;
                        let lo = addr_vec[1] as u16;
                        let addr = hi | lo;
                        addr
                    },
                    Err(e) => {0_u16}
                };

                println!("{:04x}: {:02x} {:02x} {:02x} {:02x}",
                    addr,
                    mem.get_byte(addr),
                    mem.get_byte(addr + 1),
                    mem.get_byte(addr + 2),
                    mem.get_byte(addr + 3),
                );
            }
            if command[0] == "r" {
                self.running = true;
                println!("Running.");
            }
            if command[0] == "s" {
                self.cpu_tick(cpu, mem);
            }
        }
    }

    fn cpu_tick(&self, cpu: &mut Cpu, mem: &mut Mem) {
        if self.show_state {
            println!("{}", cpu.state_string());
        }

        if self.show_disassembly {
            self.disassemble(mem.get_byte(cpu.pc),
                    mem.get_byte(cpu.pc + 1),
                    mem.get_byte(cpu.pc + 2));
        }

        cpu.tick(mem);
    }

    pub fn disassemble(&self, b1: u8, b2: u8, b3: u8) {
        let opcode = b1;
        let (op, mode) = self.opcodes[opcode as usize];

        let disasm = match mode {
            Mode::ABS => format!("{:?} ${:02x}{:02x}", op, b3, b2),
            Mode::ABX => format!("{:?} ${:02x}{:02x},X", op, b3, b2),
            Mode::ABY => format!("{:?} ${:02x}{:02x},Y", op, b3, b2),
            Mode::IMM => format!("{:?} #${:02x}", op, b2),
            Mode::IMP => format!("{:?}", op),
            Mode::IND => format!("{:?} (${:02x}{:02x})", op, b3, b2),
            Mode::IZX => format!("{:?} (${:02x},X)", op, b2),
            Mode::IZY => format!("{:?} (${:02x}),Y", op, b2),
            Mode::REL => format!("{:?} ${:02x}", op, b2),
            Mode::ZP => format!("{:?} ${:02x}", op, b2),
            Mode::ZPX => format!("{:?} ${:02x},X", op, b2),
            Mode::ZPY => format!("{:?} ${:02x},Y", op, b2),
            _ => String::from("???"),
        };

        println!("{}", disasm);
    }
}

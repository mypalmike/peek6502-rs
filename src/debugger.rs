use std::collections::HashSet;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::bus::Bus;
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
    n_runs: u32,
    breakpoints: HashSet<u16>,
    opcodes: [(Op, Mode); 256],
    pub debug_mode: bool,  // GDB-style CLI debugger mode
    trace_mode: bool,  // Trace mode - print each instruction as it executes
    trace_count: Option<u32>,  // Number of instructions to trace (None = unlimited)
    interrupt_flag: Option<Arc<AtomicBool>>,  // Ctrl-C signal flag
}

impl Debugger {
    pub fn new() -> Debugger {
        Debugger {
            show_state: false,
            show_disassembly: false,
            running: false,
            n_runs: 0,
            breakpoints: HashSet::new(),
            debug_mode: false,
            trace_mode: false,
            trace_count: None,
            interrupt_flag: None,
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
                (Op::CPX, Mode::ZP),  (Op::SBC, Mode::ZP),  (Op::INC, Mode::ZP),  (Op::ISC, Mode::ZP),
                (Op::INX, Mode::IMP), (Op::SBC, Mode::IMM), (Op::NOP, Mode::IMP), (Op::SBC, Mode::IMM),
                (Op::CPX, Mode::ABS), (Op::SBC, Mode::ABS), (Op::INC, Mode::ABS), (Op::ISC, Mode::ABS),

                // 0xF0 - 0xFF
                (Op::BEQ, Mode::REL), (Op::SBC, Mode::IZY), (Op::HLT, Mode::IMP), (Op::ISC, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::SBC, Mode::ZPX), (Op::INC, Mode::ZPX), (Op::ISC, Mode::ZPX),
                (Op::SED, Mode::IMP), (Op::SBC, Mode::ABY), (Op::NOP, Mode::IMP), (Op::ISC, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::SBC, Mode::ABX), (Op::INC, Mode::ABX), (Op::ISC, Mode::ABX),
            ],
        }
    }

    /// Enable GDB-style CLI debugger mode
    pub fn enable_debug_mode(&mut self, initial_pc: u16) {
        self.debug_mode = true;
        self.running = false;
        self.breakpoints.insert(initial_pc);

        // Set up Ctrl-C handler
        let interrupt_flag = Arc::new(AtomicBool::new(false));
        self.interrupt_flag = Some(interrupt_flag.clone());

        ctrlc::set_handler(move || {
            interrupt_flag.store(true, Ordering::SeqCst);
            println!("\n^C (Interrupt - breaking at next instruction)");
        })
        .expect("Error setting Ctrl-C handler");

        println!("Debugger enabled. Breaking at first instruction (${:04X})", initial_pc);
        println!("Commands: c (continue), n (next), s (step), b <addr> (breakpoint), p <addr> [size] (print), h (help)");
        println!("Press Ctrl-C during execution to break at next instruction");
    }

    pub fn tick(&mut self, cpu: &mut Cpu, bus: &mut dyn Bus) {
        // GDB-style debugger mode
        if self.debug_mode {
            return self.debug_tick(cpu, bus);
        }

        // Original debugger mode (for compatibility)
        if self.running {
            self.cpu_tick(cpu, bus);
            if self.breakpoints.contains(&cpu.pc) {
                self.n_runs -= 1;
                if self.n_runs == 0 {
                    self.running = false;
                    println!("Breakpoint at {:04x}", cpu.pc);
                }
            }
        } else {
            let mut input = String::new();
            let _ = io::stdin().read_line(&mut input);
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
                    Err(_e) => {}
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
                    Err(_e) => {0_u16}
                };

                println!("{:04x}: {:02x} {:02x} {:02x} {:02x}",
                    addr,
                    bus.read(addr),
                    bus.read(addr + 1),
                    bus.read(addr + 2),
                    bus.read(addr + 3),
                );
            }
            if command[0] == "r" {
                self.n_runs = 1;
                self.running = true;
                println!("Running.");
            }
            if command[0] == "f" {
                self.n_runs = command[1].parse::<u32>().unwrap();
                self.running = true;
                println!("Forward {} times", self.n_runs);
            }
            if command[0] == "s" {
                self.cpu_tick(cpu, bus);
            }
        }
    }

    fn debug_tick(&mut self, cpu: &mut Cpu, bus: &mut dyn Bus) {
        // Check for Ctrl-C interrupt
        if let Some(ref flag) = self.interrupt_flag {
            if flag.load(Ordering::SeqCst) {
                flag.store(false, Ordering::SeqCst);
                self.running = false;
                if self.trace_mode {
                    self.trace_mode = false;
                    self.trace_count = None;
                    println!("\nTrace stopped.");
                }
            }
        }

        // Check if we hit a breakpoint or need to stop
        if !self.running || self.breakpoints.contains(&cpu.pc) {
            self.running = false;

            // Show current instruction
            let b1 = bus.read(cpu.pc);
            let b2 = bus.read(cpu.pc + 1);
            let b3 = bus.read(cpu.pc + 2);

            println!("=> ${:04X}: ", cpu.pc);
            print!("   ");
            self.disassemble(b1, b2, b3);
            print!("   {}\n", cpu.state_string());

            // Read command
            loop {
                print!("(debug) ");
                use std::io::Write;
                io::stdout().flush().unwrap();

                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                let parts: Vec<&str> = input.trim().split_whitespace().collect();

                if parts.is_empty() {
                    continue;
                }

                match parts[0] {
                    "c" | "continue" => {
                        // Execute one instruction to get past current breakpoint
                        self.execute_instruction(cpu, bus);
                        self.running = true;
                        self.trace_mode = false;
                        break;
                    }
                    "t" | "trace" => {
                        // Continue execution while printing each instruction
                        // Optional: t <count> to trace N instructions
                        let count = if parts.len() >= 2 {
                            match parts[1].parse::<u32>() {
                                Ok(n) => Some(n),
                                Err(_) => {
                                    println!("Invalid count: {}. Usage: t [count]", parts[1]);
                                    continue;
                                }
                            }
                        } else {
                            None
                        };

                        self.execute_instruction(cpu, bus);
                        self.running = true;
                        self.trace_mode = true;
                        self.trace_count = count;

                        if let Some(n) = count {
                            println!("Tracing {} instructions...", n);
                        } else {
                            println!("Tracing enabled. Press Ctrl-C to stop.");
                        }
                        break;
                    }
                    "n" | "next" => {
                        // Step one complete instruction
                        self.execute_instruction(cpu, bus);
                        break;
                    }
                    "s" | "step" => {
                        // Step (same as next for now, could be different for JSR)
                        self.execute_instruction(cpu, bus);
                        break;
                    }
                    "b" | "break" | "breakpoint" => {
                        if parts.len() < 2 {
                            println!("Usage: b <hex_address>");
                            continue;
                        }

                        // Parse hex address
                        let addr_str = parts[1].trim_start_matches("0x").trim_start_matches("$");
                        match u16::from_str_radix(addr_str, 16) {
                            Ok(addr) => {
                                if self.breakpoints.contains(&addr) {
                                    self.breakpoints.remove(&addr);
                                    println!("Removed breakpoint at ${:04X}", addr);
                                } else {
                                    self.breakpoints.insert(addr);
                                    println!("Added breakpoint at ${:04X}", addr);
                                }
                            }
                            Err(_) => println!("Invalid hex address: {}", parts[1]),
                        }
                    }
                    "p" | "print" => {
                        if parts.len() < 2 {
                            println!("Usage: p <hex_address> [1|2]");
                            println!("  1 = print 1 byte (default)");
                            println!("  2 = print 2 bytes (little endian)");
                            continue;
                        }

                        // Parse address
                        let addr_str = parts[1].trim_start_matches("0x").trim_start_matches("$");
                        let addr = match u16::from_str_radix(addr_str, 16) {
                            Ok(a) => a,
                            Err(_) => {
                                println!("Invalid hex address: {}", parts[1]);
                                continue;
                            }
                        };

                        // Parse size (default 1)
                        let size = if parts.len() >= 3 {
                            parts[2].parse::<u8>().unwrap_or(1)
                        } else {
                            1
                        };

                        match size {
                            1 => {
                                let val = bus.read(addr);
                                println!("${:04X}: ${:02X} = {} (decimal)", addr, val, val);
                            }
                            2 => {
                                let lo = bus.read(addr);
                                let hi = bus.read(addr + 1);
                                let val = (hi as u16) << 8 | lo as u16;
                                println!("${:04X}: ${:02X} ${:02X} = ${:04X} = {} (decimal)",
                                         addr, lo, hi, val, val);
                            }
                            _ => {
                                println!("Size must be 1 or 2");
                            }
                        }
                    }
                    "q" | "quit" => {
                        println!("Quitting...");
                        std::process::exit(0);
                    }
                    "i" | "info" => {
                        println!("Breakpoints:");
                        if self.breakpoints.is_empty() {
                            println!("  (none)");
                        } else {
                            let mut bps: Vec<_> = self.breakpoints.iter().collect();
                            bps.sort();
                            for bp in bps {
                                println!("  ${:04X}", bp);
                            }
                        }
                    }
                    "h" | "help" => {
                        println!("Commands:");
                        println!("  c, continue          - Continue execution");
                        println!("  t [count]            - Trace execution (optional: trace N instructions)");
                        println!("  n, next              - Execute next instruction");
                        println!("  s, step              - Step into (same as next)");
                        println!("  b <addr>             - Toggle breakpoint at address");
                        println!("  p <addr> [1|2]       - Print memory (1=byte, 2=word little-endian)");
                        println!("  i, info              - Show breakpoints");
                        println!("  q, quit              - Quit emulator");
                        println!("  h, help              - Show this help");
                    }
                    _ => {
                        println!("Unknown command: {}. Type 'h' for help.", parts[0]);
                    }
                }
            }
        } else {
            // Running normally - execute complete instruction
            if self.trace_mode {
                // Print instruction before executing
                let b1 = bus.read(cpu.pc);
                let b2 = bus.read(cpu.pc + 1);
                let b3 = bus.read(cpu.pc + 2);
                print!("${:04X}: ", cpu.pc);
                self.disassemble(b1, b2, b3);

                // Decrement trace counter if set
                if let Some(count) = self.trace_count {
                    if count <= 1 {
                        // Last instruction - stop tracing
                        self.running = false;
                        self.trace_mode = false;
                        self.trace_count = None;
                        println!("\nTrace complete.");
                    } else {
                        self.trace_count = Some(count - 1);
                    }
                }
            }
            self.execute_instruction(cpu, bus);
        }
    }

    /// Execute one complete instruction (all cycles until instruction boundary)
    fn execute_instruction(&mut self, cpu: &mut Cpu, bus: &mut dyn Bus) {
        // Execute cycles until we're at an instruction boundary
        loop {
            cpu.tick(bus);
            if cpu.cycles_remaining == 0 {
                break;
            }
        }
    }

    fn cpu_tick(&self, cpu: &mut Cpu, bus: &mut dyn Bus) {
        if self.show_state {
            println!("{}", cpu.state_string());
        }

        if self.show_disassembly {
            self.disassemble(bus.read(cpu.pc),
                    bus.read(cpu.pc + 1),
                    bus.read(cpu.pc + 2));
        }

        cpu.tick(bus);
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

/// Minimal 6502 machine for running Klaus Dormann's functional test suite
///
/// This machine type provides:
/// - 64KB all-RAM memory (loads 6502_functional_test.bin)
/// - CPU with cycle-accurate execution
/// - Trap detection (infinite JMP loops indicate test failure)

use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::mem::Mem;

pub struct FunctionalTest {
    cpu: Cpu,
    mem: Mem,
    cycle_count: u64,
}

impl FunctionalTest {
    pub fn new() -> FunctionalTest {
        let mut test = FunctionalTest {
            cpu: Cpu::new(),
            mem: Mem::new(0, true),  // split=0 means all RAM, load_test=true
            cycle_count: 0,
        };

        // Set PC to test start address
        test.cpu.pc = 0x0400;

        test
    }

    /// Run the test until completion or trap
    pub fn run(&mut self) {
        println!("Starting 6502 functional test at PC=${:04X}", self.cpu.pc);
        println!("Trap detection enabled (infinite JMP loops)");
        println!();

        let mut last_pc = self.cpu.pc;
        let mut stuck_count = 0;

        loop {
            // Check for success FIRST (PC = 0x3469 is the success marker in Klaus test)
            // This address contains JMP $3469, so it must be checked before trap detection
            if self.cpu.pc == 0x3469 && self.cpu.cycles_remaining == 0 {
                println!("\n✓ SUCCESS! All tests passed.");
                println!("Completed in {} cycles", self.cycle_count);
                break;
            }

            // Check for trap (JMP to self - infinite loop at any OTHER address)
            if self.is_trap() && self.cpu.pc != 0x3469 {
                self.show_trap_info(last_pc);
                break;
            }

            last_pc = self.cpu.pc;

            // Execute one CPU cycle
            // Use mem::replace to work around borrow checker
            let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());
            cpu.tick(self);
            self.cpu = cpu;
            self.cycle_count += 1;

            // Check if PC is stuck (same instruction executed twice)
            if self.cpu.cycles_remaining == 0 && self.cpu.pc == last_pc {
                stuck_count += 1;
                if stuck_count > 2 {
                    // Definitely trapped
                    self.show_trap_info(last_pc);
                    break;
                }
            } else {
                stuck_count = 0;
            }

            // Progress indicator every 100k cycles
            if self.cycle_count % 100_000 == 0 {
                print!(".");
                use std::io::Write;
                std::io::stdout().flush().ok();
            }
        }
    }

    /// Check if current instruction is a trap (JMP to self)
    fn is_trap(&mut self) -> bool {
        let opcode = self.read(self.cpu.pc);

        // Check for JMP absolute (0x4C)
        if opcode == 0x4C {
            let target_lo = self.read(self.cpu.pc.wrapping_add(1));
            let target_hi = self.read(self.cpu.pc.wrapping_add(2));
            let target = ((target_hi as u16) << 8) | (target_lo as u16);

            // If JMP target equals current PC, it's a trap
            if target == self.cpu.pc {
                return true;
            }
        }

        false
    }

    /// Display diagnostic information when trap is detected
    fn show_trap_info(&mut self, trapped_pc: u16) {
        println!("\n\n╔════════════════════════════════════════════════════════════╗");
        println!("║                   TRAP DETECTED                            ║");
        println!("║              Test Failed - Infinite Loop                   ║");
        println!("╚════════════════════════════════════════════════════════════╝");
        println!();
        println!("Trapped at: ${:04X}", trapped_pc);
        println!("Cycles executed: {}", self.cycle_count);
        println!();

        self.show_cpu_state();
        println!();
        self.show_disassembly(trapped_pc);
    }

    /// Display CPU register state
    fn show_cpu_state(&self) {
        println!("CPU State:");
        println!("  PC: ${:04X}  A: ${:02X}  X: ${:02X}  Y: ${:02X}  SP: ${:02X}",
                 self.cpu.pc, self.cpu.a, self.cpu.x, self.cpu.y, self.cpu.s);

        let flags = format!("{}{}{}{}{}{}{}{}",
            if self.cpu.n { "N" } else { "n" },
            if self.cpu.v { "V" } else { "v" },
            "-",
            if self.cpu.b { "B" } else { "b" },
            if self.cpu.d { "D" } else { "d" },
            if self.cpu.i { "I" } else { "i" },
            if self.cpu.z { "Z" } else { "z" },
            if self.cpu.c { "C" } else { "c" },
        );
        println!("  Flags: {}", flags);
    }

    /// Display disassembly around the trap location
    fn show_disassembly(&mut self, pc: u16) {
        println!("Disassembly:");

        // Show a few instructions before the trap
        let start = pc.saturating_sub(10);
        for addr in (start..=pc.saturating_add(10)).step_by(1) {
            let marker = if addr == pc { ">>>" } else { "   " };
            let opcode = self.read(addr);
            let byte1 = self.read(addr.wrapping_add(1));
            let byte2 = self.read(addr.wrapping_add(2));

            println!("  {} ${:04X}: {:02X} {:02X} {:02X}",
                     marker, addr, opcode, byte1, byte2);

            // Only show a few lines
            if addr >= pc.saturating_add(6) {
                break;
            }
        }
    }
}

impl Bus for FunctionalTest {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem.get_byte(addr)
    }

    fn write(&mut self, addr: u16, val: u8) {
        self.mem.set_byte(addr, val);
    }
}

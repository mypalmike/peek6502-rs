use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::mem::Mem;
use crate::debugger::Debugger;
use crate::antic::Antic;
use crate::gtia::Gtia;
use crate::pokey::Pokey;
use crate::pia::Pia;

pub struct Atari800 {
    // Core components
    cpu: Cpu,
    mem: Mem,

    // Custom chips
    antic: Antic,
    gtia: Gtia,
    pokey: Pokey,
    pia: Pia,

    // Debugger
    debugger: Debugger,

    // Cycle tracking
    master_cycle: u64,
    cpu_halted: bool,
}

impl Atari800 {
    pub fn new() -> Atari800 {
        let mut atari800 = Atari800 {
            cpu: Cpu::new(),
            mem: Mem::new(0, true),
            antic: Antic::new(),
            gtia: Gtia::new(),
            pokey: Pokey::new(),
            pia: Pia::new(),
            debugger: Debugger::new(),
            master_cycle: 0,
            cpu_halted: false,
        };

        // Reset CPU after construction
        // We can't call reset with &mut atari800 due to borrow checker,
        // so we'll just set the PC manually for now
        // TODO: Load PC from reset vector at 0xFFFC
        atari800.cpu.pc = 0x0400; // Functional test start address

        atari800
    }

    pub fn tick(&mut self) {
        // For now, keep debugger-driven execution
        // TODO: Integrate with cycle-accurate execution below

        // We need to temporarily take ownership of cpu and debugger to call tick
        // because we can't borrow self mutably while also passing self as Bus
        let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());
        let mut debugger = std::mem::replace(&mut self.debugger, Debugger::new());

        debugger.tick(&mut cpu, self);

        self.cpu = cpu;
        self.debugger = debugger;

        // Cycle-accurate execution (commented out for now to avoid breaking debugger)
        // self.tick_cycle_accurate();
    }

    /// Cycle-accurate tick - executes one machine cycle
    #[allow(dead_code)]
    fn tick_cycle_accurate(&mut self) {
        // ANTIC runs first and decides if it needs DMA
        let dma_active = self.antic.tick(&mut self.mem);

        if dma_active {
            // ANTIC is using the bus - CPU is halted
            self.cpu_halted = true;
        } else {
            // CPU can execute - executes one cycle
            self.cpu_halted = false;

            // Use mem::replace to temporarily take ownership of CPU
            let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());
            cpu.tick(self);  // CPU now tracks its own multi-cycle state
            self.cpu = cpu;
        }

        // GTIA always runs (generates video)
        self.gtia.tick();

        // POKEY runs (sound, timers, serial I/O)
        self.pokey.tick();

        // PIA runs (joystick input)
        self.pia.tick();

        self.master_cycle += 1;
    }
}

impl Bus for Atari800 {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            // GTIA registers ($D000-$D01F)
            0xD000..=0xD01F => self.gtia.read_register(addr),

            // POKEY registers ($D200-$D2FF)
            0xD200..=0xD2FF => self.pokey.read_register(addr),

            // PIA registers ($D300-$D3FF)
            0xD300..=0xD3FF => self.pia.read_register(addr),

            // ANTIC registers ($D400-$D4FF)
            0xD400..=0xD4FF => self.antic.read_register(addr),

            // Regular memory (RAM/ROM)
            _ => self.mem.get_byte(addr),
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // GTIA registers ($D000-$D01F)
            0xD000..=0xD01F => self.gtia.write_register(addr, val),

            // POKEY registers ($D200-$D2FF)
            0xD200..=0xD2FF => self.pokey.write_register(addr, val),

            // PIA registers ($D300-$D3FF)
            0xD300..=0xD3FF => self.pia.write_register(addr, val),

            // ANTIC registers ($D400-$D4FF)
            0xD400..=0xD4FF => self.antic.write_register(addr, val),

            // Regular memory (RAM/ROM)
            _ => self.mem.set_byte(addr, val),
        }
    }
}

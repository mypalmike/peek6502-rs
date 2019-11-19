use crate::cpu::Cpu;
use crate::mem::Mem;
use crate::debugger::Debugger;

pub struct Atari800 {
    debugger : Debugger,
    mem : Mem,
    cpu : Cpu,
}

impl Atari800 {
    pub fn new() -> Atari800 {
        let mut atari800 = Atari800 {
            debugger : Debugger::new(),
            mem : Mem::new(0, true),
            cpu : Cpu::new(),
        };

        atari800.cpu.reset(&mut atari800.mem);

        atari800
    }

    pub fn tick(&mut self) {
        self.debugger.tick(&mut self.cpu, &mut self.mem);
        // self.cpu.tick(&mut self.mem);
    }
}

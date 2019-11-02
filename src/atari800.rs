use crate::cpu::Cpu;
use crate::mem::Mem;

pub struct Atari800 {
    mem : Mem,
    cpu : Cpu,
}

impl Atari800 {
    pub fn new() -> Atari800 {
        let mut atari800 = Atari800 {
            mem : Mem::new(),
            cpu : Cpu::new(),
        };

        atari800.cpu.reset(&mut atari800.mem);

        atari800
    }

    pub fn tick(&mut self) {
        self.cpu.tick(&mut self.mem);
    }
}

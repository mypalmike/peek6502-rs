use crate::cpu::Cpu;
use crate::cpu_pins::CpuPins;
use crate::mem::Mem;
use crate::antic::Antic;
use crate::antic::antic_tick;
use crate::debugger::Debugger;
use crate::debugger::debugger_tick;
use crate::display::Display;
use crate::mem_controller::MemController;


use std::rc::Rc;
use std::cell::RefCell;

pub struct Atari800 {
    debugger: Rc<RefCell<Debugger>>,
    cpu: Rc<RefCell<Cpu>>,
    cpu_pins: Rc<RefCell<CpuPins>>,
    mem_controller: Rc<RefCell<MemController>>,
    // mem: Rc<RefCell<Mem>>,
    antic: Rc<RefCell<Antic>>,
    display: Rc<RefCell<Display>>,
}

impl Atari800 {
    pub fn new() -> Result<Atari800, String> {
        let cpu_pins = Rc::new(RefCell::new(CpuPins::new()));
        let antic = Rc::new(RefCell::new(Antic::new(cpu_pins.clone())));

        let mut atari800 = Atari800 {
            debugger: Rc::new(RefCell::new(Debugger::new())),
            cpu: Rc::new(RefCell::new(Cpu::new(cpu_pins.clone()))),
            cpu_pins: cpu_pins,
            mem: Rc::new(RefCell::new(Mem::new(antic.clone()))),
            antic: antic,
            display: Rc::new(RefCell::new(Display::new()?)),
        };

        atari800.cpu.clone().borrow_mut().reset(&mut atari800.mem.clone().borrow_mut());

        Ok(atari800)
    }

    pub fn tick(&self) {
        debugger_tick(self.debugger.clone(), self.cpu.clone(), self.mem.clone());
        antic_tick(self.antic.clone(), self.mem.clone(), self.display.clone());

        // self.debugger.clone().tick(self.cpu.clone(), self.mem.clone);
        // self.antic.clone().tick(&mut self.mem);

        // mem_tick(&mut self.mem);//, &mut self.antic);
        // self.mem_controller.tick();
        // self.cpu.tick(&mut self.mem);
    }
}

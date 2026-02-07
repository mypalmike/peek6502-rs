use atari800_rs::bus::Bus;
use atari800_rs::cpu::Cpu;
use atari800_rs::mem::Mem;

// Test bus with controllable interrupt lines
struct InterruptTestBus {
    mem: Mem,
    irq_state: bool,
    nmi_state: bool,
}

impl InterruptTestBus {
    fn new() -> InterruptTestBus {
        InterruptTestBus {
            mem: Mem::new(0, false),  // All RAM for testing
            irq_state: false,
            nmi_state: false,
        }
    }

    fn set_irq(&mut self, asserted: bool) {
        self.irq_state = asserted;
    }

    fn set_nmi(&mut self, asserted: bool) {
        self.nmi_state = asserted;
    }
}

impl Bus for InterruptTestBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem.get_byte(addr)
    }

    fn write(&mut self, addr: u16, val: u8) {
        self.mem.set_byte(addr, val);
    }

    fn nmi_asserted(&self) -> bool {
        self.nmi_state
    }

    fn irq_asserted(&self) -> bool {
        self.irq_state
    }
}

// Helper to set up interrupt vectors
fn setup_vectors(bus: &mut InterruptTestBus, nmi_vec: u16, irq_vec: u16) {
    // NMI vector at $FFFA-$FFFB
    bus.write(0xFFFA, (nmi_vec & 0xFF) as u8);
    bus.write(0xFFFB, (nmi_vec >> 8) as u8);

    // IRQ/BRK vector at $FFFE-$FFFF
    bus.write(0xFFFE, (irq_vec & 0xFF) as u8);
    bus.write(0xFFFF, (irq_vec >> 8) as u8);
}

// Load code at address and set PC
fn load_code(bus: &mut InterruptTestBus, addr: u16, code: &[u8]) {
    for (i, &byte) in code.iter().enumerate() {
        bus.write(addr + i as u16, byte);
    }
}

// Execute one complete instruction (all cycles)
fn execute_instruction(cpu: &mut Cpu, bus: &mut InterruptTestBus) {
    // Execute cycles until we're at an instruction boundary
    loop {
        cpu.tick(bus);
        if cpu.cycles_remaining == 0 {
            break;
        }
    }
}

#[test]
fn test_brk_instruction() {
    let mut cpu = Cpu::new();
    let mut bus = InterruptTestBus::new();

    // Set up IRQ vector to point to handler at $0900
    setup_vectors(&mut bus, 0x0000, 0x0900);

    // Handler: just does RTI
    load_code(&mut bus, 0x0900, &[
        0x40,  // RTI
    ]);

    // Main code: BRK instruction
    load_code(&mut bus, 0x0800, &[
        0x00,  // BRK
    ]);

    cpu.pc = 0x0800;
    cpu.s = 0xFF;  // Stack starts at top

    // Execute BRK
    execute_instruction(&mut cpu, &mut bus);

    // Verify CPU jumped to IRQ handler
    assert_eq!(cpu.pc, 0x0900, "BRK should jump to IRQ vector");

    // Verify B flag was set in pushed status
    let pushed_status = bus.read(0x0100 | ((cpu.s as u16) + 1));
    assert_eq!(pushed_status & 0x10, 0x10, "B flag should be set in pushed status");

    // Verify I flag is set (interrupts disabled)
    assert_eq!(cpu.i, true, "I flag should be set after BRK");

    // Verify return address is PC+2 (after BRK instruction)
    let ret_addr_lo = bus.read(0x0100 | ((cpu.s as u16) + 2));
    let ret_addr_hi = bus.read(0x0100 | ((cpu.s as u16) + 3));
    let ret_addr = (ret_addr_hi as u16) << 8 | ret_addr_lo as u16;
    assert_eq!(ret_addr, 0x0802, "Return address should be PC+2");
}

#[test]
fn test_irq_basic() {
    let mut cpu = Cpu::new();
    let mut bus = InterruptTestBus::new();

    // Set up IRQ vector
    setup_vectors(&mut bus, 0x0000, 0x0900);

    // IRQ handler: set a flag and RTI
    load_code(&mut bus, 0x0900, &[
        0xA9, 0x42,  // LDA #$42
        0x8D, 0x00, 0x02,  // STA $0200
        0x40,  // RTI
    ]);

    // Main code: loop forever
    load_code(&mut bus, 0x0800, &[
        0xEA,  // NOP
        0x4C, 0x00, 0x08,  // JMP $0800 (infinite loop)
    ]);

    cpu.pc = 0x0800;
    cpu.s = 0xFF;
    cpu.i = false;  // Enable interrupts

    // Execute NOP
    execute_instruction(&mut cpu, &mut bus);

    // Assert IRQ line
    bus.set_irq(true);

    // Execute next instruction - should take IRQ before JMP
    execute_instruction(&mut cpu, &mut bus);

    // CPU should be in IRQ handler
    assert_eq!(cpu.pc, 0x0900, "IRQ should jump to handler");
    assert_eq!(cpu.i, true, "I flag should be set during IRQ");

    // Execute handler
    execute_instruction(&mut cpu, &mut bus);  // LDA
    execute_instruction(&mut cpu, &mut bus);  // STA

    // Verify handler ran
    assert_eq!(bus.read(0x0200), 0x42, "Handler should have written $42");

    // Clear IRQ and return
    bus.set_irq(false);
    execute_instruction(&mut cpu, &mut bus);  // RTI

    // Should be back at JMP instruction
    assert_eq!(cpu.pc, 0x0801, "RTI should return to interrupted instruction");
    assert_eq!(cpu.i, false, "I flag should be restored after RTI");
}

#[test]
fn test_irq_masked() {
    let mut cpu = Cpu::new();
    let mut bus = InterruptTestBus::new();

    // Set up IRQ vector
    setup_vectors(&mut bus, 0x0000, 0x0900);

    // Main code
    load_code(&mut bus, 0x0800, &[
        0xEA,  // NOP
        0xEA,  // NOP
        0x00,  // BRK (end marker)
    ]);

    cpu.pc = 0x0800;
    cpu.i = true;  // Disable interrupts

    // Assert IRQ
    bus.set_irq(true);

    // Execute NOPs - IRQ should be ignored
    execute_instruction(&mut cpu, &mut bus);
    assert_eq!(cpu.pc, 0x0801, "IRQ should be masked when I=1");

    execute_instruction(&mut cpu, &mut bus);
    assert_eq!(cpu.pc, 0x0802, "IRQ should still be masked");
}

#[test]
fn test_irq_timing() {
    let mut cpu = Cpu::new();
    let mut bus = InterruptTestBus::new();

    // Set up IRQ vector
    setup_vectors(&mut bus, 0x0000, 0x0900);

    // Main code: multi-cycle instruction
    load_code(&mut bus, 0x0800, &[
        0xAD, 0x00, 0x02,  // LDA $0200 (4 cycles)
        0xEA,  // NOP (to see if we get here)
    ]);
    bus.write(0x0200, 0x42);  // Value to load

    cpu.pc = 0x0800;
    cpu.i = false;

    // Start first cycle of LDA (fetch opcode)
    cpu.tick(&mut bus);

    // Assert IRQ during instruction execution (middle of LDA)
    bus.set_irq(true);

    // Finish the LDA instruction
    while cpu.cycles_remaining > 0 {
        cpu.tick(&mut bus);
    }

    // LDA should have completed, A register should have value
    assert_eq!(cpu.a, 0x42, "LDA should complete despite IRQ during execution");
    assert_eq!(cpu.pc, 0x0803, "PC should be after LDA");

    // Now at instruction boundary, IRQ should be taken
    execute_instruction(&mut cpu, &mut bus);
    assert_eq!(cpu.pc, 0x0900, "IRQ should be taken after instruction completes");
}

#[test]
fn test_nmi_edge_detection() {
    let mut cpu = Cpu::new();
    let mut bus = InterruptTestBus::new();

    // Set up NMI vector
    setup_vectors(&mut bus, 0x0900, 0x0000);

    // NMI handler
    load_code(&mut bus, 0x0900, &[
        0xA9, 0x99,  // LDA #$99
        0x8D, 0x00, 0x02,  // STA $0200
        0x40,  // RTI
    ]);

    // Main code
    load_code(&mut bus, 0x0800, &[
        0xEA,  // NOP
        0xEA,  // NOP
        0x00,  // BRK
    ]);

    cpu.pc = 0x0800;
    cpu.s = 0xFF;

    // NMI line starts low
    bus.set_nmi(false);
    execute_instruction(&mut cpu, &mut bus);  // NOP

    // Rising edge: false -> true
    bus.set_nmi(true);
    execute_instruction(&mut cpu, &mut bus);  // Should take NMI

    assert_eq!(cpu.pc, 0x0900, "NMI should trigger on rising edge");

    // Execute handler
    execute_instruction(&mut cpu, &mut bus);  // LDA
    execute_instruction(&mut cpu, &mut bus);  // STA
    assert_eq!(bus.read(0x0200), 0x99, "NMI handler should have run");

    execute_instruction(&mut cpu, &mut bus);  // RTI
    assert_eq!(cpu.pc, 0x0801, "Should return from NMI");
}

#[test]
fn test_nmi_repeated() {
    let mut cpu = Cpu::new();
    let mut bus = InterruptTestBus::new();

    // Set up NMI vector
    setup_vectors(&mut bus, 0x0900, 0x0000);

    // NMI handler: increment counter at $0200
    load_code(&mut bus, 0x0900, &[
        0xEE, 0x00, 0x02,  // INC $0200
        0x40,  // RTI
    ]);

    // Main code
    load_code(&mut bus, 0x0800, &[
        0xEA,  // NOP
        0xEA,  // NOP
        0xEA,  // NOP
        0xEA,  // NOP
        0x00,  // BRK
    ]);

    cpu.pc = 0x0800;
    cpu.s = 0xFF;
    bus.write(0x0200, 0);  // Initialize counter

    // First NMI: rising edge
    bus.set_nmi(false);
    execute_instruction(&mut cpu, &mut bus);  // NOP at 0x0800
    bus.set_nmi(true);
    execute_instruction(&mut cpu, &mut bus);  // Take NMI

    assert_eq!(cpu.pc, 0x0900);
    execute_instruction(&mut cpu, &mut bus);  // INC
    execute_instruction(&mut cpu, &mut bus);  // RTI
    assert_eq!(bus.read(0x0200), 1, "Counter should be 1");

    // NMI still high - should NOT retrigger
    execute_instruction(&mut cpu, &mut bus);  // NOP at 0x0801
    assert_eq!(cpu.pc, 0x0802, "NMI should not retrigger while high");

    // New rising edge required
    bus.set_nmi(false);
    execute_instruction(&mut cpu, &mut bus);  // NOP at 0x0802
    bus.set_nmi(true);
    execute_instruction(&mut cpu, &mut bus);  // Should take NMI again

    assert_eq!(cpu.pc, 0x0900);
    execute_instruction(&mut cpu, &mut bus);  // INC
    assert_eq!(bus.read(0x0200), 2, "Counter should be 2 after second NMI");
}

#[test]
fn test_nmi_not_masked() {
    let mut cpu = Cpu::new();
    let mut bus = InterruptTestBus::new();

    // Set up NMI vector
    setup_vectors(&mut bus, 0x0900, 0x0000);

    // NMI handler
    load_code(&mut bus, 0x0900, &[
        0xA9, 0xAA,  // LDA #$AA
        0x8D, 0x00, 0x02,  // STA $0200
        0x40,  // RTI
    ]);

    // Main code
    load_code(&mut bus, 0x0800, &[
        0xEA,  // NOP
        0x00,  // BRK
    ]);

    cpu.pc = 0x0800;
    cpu.s = 0xFF;
    cpu.i = true;  // IRQ disabled - should not affect NMI

    bus.set_nmi(false);
    execute_instruction(&mut cpu, &mut bus);  // NOP

    // Trigger NMI
    bus.set_nmi(true);
    execute_instruction(&mut cpu, &mut bus);

    assert_eq!(cpu.pc, 0x0900, "NMI should trigger even with I=1");

    // Execute handler
    execute_instruction(&mut cpu, &mut bus);  // LDA
    execute_instruction(&mut cpu, &mut bus);  // STA
    assert_eq!(bus.read(0x0200), 0xAA, "NMI handler should run despite I flag");

    execute_instruction(&mut cpu, &mut bus);  // RTI
}

#[test]
fn test_rti_instruction() {
    let mut cpu = Cpu::new();
    let mut bus = InterruptTestBus::new();

    // Set up stack with return state
    cpu.s = 0xFC;  // Stack pointer
    cpu.pc = 0x0800;

    // Push status flags (with N=1, V=1, Z=0, C=1)
    let status = 0b11000001;  // N, V, -, -, -, -, C
    bus.write(0x01FD, status);

    // Push return address $1234
    bus.write(0x01FE, 0x34);  // PCL
    bus.write(0x01FF, 0x12);  // PCH

    // RTI instruction
    load_code(&mut bus, 0x0800, &[
        0x40,  // RTI
    ]);

    // Set some different flags
    cpu.n = false;
    cpu.v = false;
    cpu.z = true;
    cpu.c = false;

    execute_instruction(&mut cpu, &mut bus);

    // Verify PC restored
    assert_eq!(cpu.pc, 0x1234, "PC should be restored from stack");

    // Verify flags restored
    assert_eq!(cpu.n, true, "N flag should be restored");
    assert_eq!(cpu.v, true, "V flag should be restored");
    assert_eq!(cpu.z, false, "Z flag should be restored");
    assert_eq!(cpu.c, true, "C flag should be restored");

    // Verify stack pointer
    assert_eq!(cpu.s, 0xFF, "Stack pointer should be adjusted");
}

#[test]
fn test_interrupt_stack_format() {
    let mut cpu = Cpu::new();
    let mut bus = InterruptTestBus::new();

    // Set up IRQ vector
    setup_vectors(&mut bus, 0x0000, 0x0900);

    // Handler: just loops
    load_code(&mut bus, 0x0900, &[
        0x4C, 0x00, 0x09,  // JMP $0900 (infinite loop)
    ]);

    // Main code at specific address
    load_code(&mut bus, 0x1234, &[
        0xEA,  // NOP
    ]);

    cpu.pc = 0x1234;
    cpu.s = 0xFF;
    cpu.i = false;

    // Set specific flag values
    cpu.n = true;   // bit 7
    cpu.v = false;  // bit 6
    cpu.d = true;   // bit 3
    cpu.z = false;  // bit 1
    cpu.c = true;   // bit 0

    // Trigger IRQ
    bus.set_irq(true);
    execute_instruction(&mut cpu, &mut bus);

    // Check stack contents
    // Stack format: [S+3]=PCH, [S+2]=PCL, [S+1]=Status
    let pcl = bus.read(0x0100 | ((cpu.s as u16) + 2));
    let pch = bus.read(0x0100 | ((cpu.s as u16) + 3));
    let status = bus.read(0x0100 | ((cpu.s as u16) + 1));

    // Verify return address (should be $1234, next instruction)
    assert_eq!(pch, 0x12, "PCH should be pushed");
    assert_eq!(pcl, 0x34, "PCL should be pushed");

    // Verify status byte format: NV-BDIZC
    // N=1, V=0, -=1, B=0 (hardware IRQ), D=1, I=0 (before), Z=0, C=1
    assert_eq!(status & 0x80, 0x80, "N flag (bit 7) should be set");
    assert_eq!(status & 0x40, 0x00, "V flag (bit 6) should be clear");
    assert_eq!(status & 0x20, 0x20, "Bit 5 should always be set");
    assert_eq!(status & 0x10, 0x00, "B flag (bit 4) should be clear for IRQ");
    assert_eq!(status & 0x08, 0x08, "D flag (bit 3) should be set");
    assert_eq!(status & 0x01, 0x01, "C flag (bit 0) should be set");
}

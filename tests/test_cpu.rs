use atari800_rs::bus::Bus;
use atari800_rs::cpu::Cpu;
use atari800_rs::mem::Mem;

// Simple test bus that wraps Mem and implements Bus trait
struct TestBus {
    mem: Mem,
}

impl TestBus {
    fn new() -> TestBus {
        TestBus {
            mem: Mem::new(0, false),
        }
    }
}

impl Bus for TestBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem.get_byte(addr)
    }

    fn write(&mut self, addr: u16, val: u8) {
        self.mem.set_byte(addr, val);
    }

    fn nmi_asserted(&self) -> bool {
        false  // No NMI for tests
    }

    fn irq_asserted(&self) -> bool {
        false  // No IRQ for tests
    }
}

fn get_cpu_bus() -> (Cpu, TestBus) {
    let bus = TestBus::new();
    let cpu = Cpu::new();

    (cpu, bus)
}

// Return true if reached break instruction within max_ticks.
fn run(cpu: &mut Cpu, bus: &mut TestBus, code: &[u8], max_ticks: i32) -> bool {
    bus.mem.ram[0x0800..(0x0800 + code.len())].copy_from_slice(code);
    bus.mem.ram[0x0800 + code.len()] = 0x00;
    cpu.pc = 0x0800;
    cpu.cycles_remaining = 0;  // Ensure we start a new instruction

    for _ in 0..max_ticks {
        cpu.tick(bus);  // Now executes one cycle at a time

        // Check if we're ready to start a new instruction and it's BRK
        if cpu.cycles_remaining == 0 && bus.read(cpu.pc) == 0 {
            return true;
        }
    }

    false
}

#[test]
fn test_lda_imm() {
    let (mut cpu, mut bus) = get_cpu_bus();

    let code: [u8; 2] = [0xA9, 0x00];  // LDA IMM
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.a, 0x00);
    assert_eq!(cpu.z, true);
    assert_eq!(cpu.n, false);

    let code: [u8; 2] = [0xA9, 0xFF];  // LDA IMM
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.a, 0xFF);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.n, true);
}

#[test]
fn test_ldx_imm() {
    let (mut cpu, mut bus) = get_cpu_bus();

    let code: [u8; 2] = [0xA2, 0x00];  // LDX IMM
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.x, 0x00);
    assert_eq!(cpu.z, true);
    assert_eq!(cpu.n, false);

    let code: [u8; 2] = [0xA2, 0xFF];  // LDX IMM
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.x, 0xFF);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.n, true);
}

#[test]
fn test_ldy_imm() {
    let (mut cpu, mut bus) = get_cpu_bus();

    let code: [u8; 2] = [0xA0, 0x00];  // LDY IMM
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.y, 0x00);
    assert_eq!(cpu.z, true);
    assert_eq!(cpu.n, false);

    let code: [u8; 2] = [0xA0, 0xFF];  // LDY IMM
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.y, 0xFF);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.n, true);
}

#[test]
fn test_adc_dec() {
    let (mut cpu, mut bus) = get_cpu_bus();

    let code: [u8; 6] = [
        0xF8,           // SED
        0xA9, 0x85,     // LDA #$85
        0x38,           // SEC
        0x69, 0x25      // ADC #$25
    ];
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.a, 0x11);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.c, true);

    let code: [u8; 6] = [
        0xF8,           // SED
        0xA9, 0x85,     // LDA #$85
        0x18,           // CLC
        0x69, 0x25      // ADC #$25
    ];
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.a, 0x10);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.c, true);

    let code: [u8; 6] = [
        0xF8,           // SED
        0xA9, 0x12,     // LDA #$12
        0x18,           // CLC
        0x69, 0x19      // ADC #$19
    ];
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.a, 0x31);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.c, false);

    let code: [u8; 6] = [
        0xF8,           // SED
        0xA9, 0x99,     // LDA #$99
        0x38,           // SEC
        0x69, 0x00      // ADC #$00
    ];
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert!(halted);
    assert_eq!(cpu.a, 0x00);
    assert_eq!(cpu.z, true);
    assert_eq!(cpu.c, true);
}

#[test]
fn test_sbc_dec() {
    let (mut cpu, mut bus) = get_cpu_bus();

    let code: [u8; 6] = [
        0xF8,           // SED
        0xA9, 0x25,     // LDA #$25
        0x18,           // CLC
        0xEB, 0x85      // SBC #$85
    ];
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.a, 0x39);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.c, false);

    let code: [u8; 6] = [
        0xF8,           // SED
        0xA9, 0x25,     // LDA #$25
        0x38,           // SEC
        0xEB, 0x85      // SBC #$85
    ];
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.a, 0x40);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.c, false);

    let code: [u8; 6] = [
        0xF8,           // SED
        0xA9, 0x85,     // LDA #$85
        0x38,           // SEC
        0xEB, 0x22      // SBC #$22
    ];
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert_eq!(cpu.a, 0x63);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.c, true);

    let code: [u8; 6] = [
        0xF8,           // SED
        0xA9, 0x75,     // LDA #$75
        0x38,           // SEC
        0xEB, 0x75      // SBC #$75
    ];
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert!(halted);
    assert_eq!(cpu.a, 0x00);
    assert_eq!(cpu.z, true);
    assert_eq!(cpu.c, true);

    let code: [u8; 6] = [
        0xF8,           // SED
        0xA9, 0x00,     // LDA #$00
        0x18,           // CLC
        0xEB, 0x99      // SBC #$99
    ];
    let halted = run(&mut cpu, &mut bus, &code, 100);
    assert!(halted);
    assert_eq!(cpu.a, 0x00);
    assert_eq!(cpu.z, true);
    assert_eq!(cpu.c, false);
}

// Helper function to count cycles for instruction execution
fn count_cycles(cpu: &mut Cpu, bus: &mut TestBus) -> u32 {
    let mut cycles = 0;
    while cpu.cycles_remaining > 0 || cycles == 0 {
        cpu.tick(bus);
        cycles += 1;
    }
    cycles
}

// ===== PAGE BOUNDARY TIMING TESTS =====

#[test]
fn test_lda_abx_no_page_cross() {
    let (mut cpu, mut bus) = get_cpu_bus();

    // Base 0x0200 + X=0x10 = 0x0210 (same page)
    bus.mem.ram[0x0210] = 0x42;
    cpu.x = 0x10;

    let code: [u8; 3] = [0xBD, 0x00, 0x02];  // LDA $0200,X
    bus.mem.ram[0x0800..0x0803].copy_from_slice(&code);
    cpu.pc = 0x0800;
    cpu.cycles_remaining = 0;

    let cycles = count_cycles(&mut cpu, &mut bus);

    assert_eq!(cpu.a, 0x42);
    assert_eq!(cycles, 4);  // Base timing, no penalty
}

#[test]
fn test_lda_abx_with_page_cross() {
    let (mut cpu, mut bus) = get_cpu_bus();

    // Base 0x02FF + X=0x01 = 0x0300 (crosses page)
    bus.mem.ram[0x0300] = 0x42;
    cpu.x = 0x01;

    let code: [u8; 3] = [0xBD, 0xFF, 0x02];  // LDA $02FF,X
    bus.mem.ram[0x0800..0x0803].copy_from_slice(&code);
    cpu.pc = 0x0800;
    cpu.cycles_remaining = 0;

    let cycles = count_cycles(&mut cpu, &mut bus);

    assert_eq!(cpu.a, 0x42);
    assert_eq!(cycles, 5);  // Base 4 + 1 penalty
}

#[test]
fn test_bne_not_taken() {
    let (mut cpu, mut bus) = get_cpu_bus();

    cpu.z = true;  // BNE won't branch (Z=1)
    let code: [u8; 2] = [0xD0, 0x10];  // BNE +16
    bus.mem.ram[0x0800..0x0802].copy_from_slice(&code);
    cpu.pc = 0x0800;
    cpu.cycles_remaining = 0;

    let cycles = count_cycles(&mut cpu, &mut bus);

    assert_eq!(cpu.pc, 0x0802);  // Falls through
    assert_eq!(cycles, 2);  // Base timing only
}

#[test]
fn test_bne_taken_no_page_cross() {
    let (mut cpu, mut bus) = get_cpu_bus();

    cpu.z = false;  // BNE will branch (Z=0)
    let code: [u8; 2] = [0xD0, 0x10];  // BNE +16
    bus.mem.ram[0x0800..0x0802].copy_from_slice(&code);
    cpu.pc = 0x0800;
    cpu.cycles_remaining = 0;

    let cycles = count_cycles(&mut cpu, &mut bus);

    assert_eq!(cpu.pc, 0x0812);  // 0x0802 + 0x10
    assert_eq!(cycles, 3);  // Base 2 + 1 for taken
}

#[test]
fn test_bne_taken_with_page_cross() {
    let (mut cpu, mut bus) = get_cpu_bus();

    cpu.z = false;  // BNE will branch (Z=0)
    let code: [u8; 2] = [0xD0, 0x7F];  // BNE +127
    bus.mem.ram[0x08F0..0x08F2].copy_from_slice(&code);
    cpu.pc = 0x08F0;  // Near page boundary
    cpu.cycles_remaining = 0;

    let cycles = count_cycles(&mut cpu, &mut bus);

    assert_eq!(cpu.pc, 0x0971);  // 0x08F2 + 0x7F crosses page
    assert_eq!(cycles, 4);  // Base 2 + 1 taken + 1 page cross
}

#[test]
fn test_bne_backward_with_page_cross() {
    let (mut cpu, mut bus) = get_cpu_bus();

    cpu.z = false;  // BNE will branch (Z=0)
    let code: [u8; 2] = [0xD0, 0x80];  // BNE -128 (0x80 as i8 = -128)
    bus.mem.ram[0x0902..0x0904].copy_from_slice(&code);
    cpu.pc = 0x0902;  // Jumping back crosses page
    cpu.cycles_remaining = 0;

    let cycles = count_cycles(&mut cpu, &mut bus);

    assert_eq!(cpu.pc, 0x0884);  // 0x0904 + (-128) crosses page
    assert_eq!(cycles, 4);  // Base 2 + 1 taken + 1 page cross
}

#[test]
fn test_sta_abx_always_same_cycles() {
    let (mut cpu, mut bus) = get_cpu_bus();

    cpu.a = 0x42;

    // Case 1: No page cross
    cpu.x = 0x01;
    let code: [u8; 3] = [0x9D, 0x00, 0x02];  // STA $0200,X
    bus.mem.ram[0x0800..0x0803].copy_from_slice(&code);
    cpu.pc = 0x0800;
    cpu.cycles_remaining = 0;

    let cycles1 = count_cycles(&mut cpu, &mut bus);
    assert_eq!(cycles1, 5);  // Always 5
    assert_eq!(bus.mem.ram[0x0201], 0x42);

    // Case 2: With page cross
    cpu.x = 0x01;
    let code2: [u8; 3] = [0x9D, 0xFF, 0x02];  // STA $02FF,X
    bus.mem.ram[0x0800..0x0803].copy_from_slice(&code2);
    cpu.pc = 0x0800;
    cpu.cycles_remaining = 0;

    let cycles2 = count_cycles(&mut cpu, &mut bus);
    assert_eq!(cycles2, 5);  // Still always 5
    assert_eq!(bus.mem.ram[0x0300], 0x42);
}

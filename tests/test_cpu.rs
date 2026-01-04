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

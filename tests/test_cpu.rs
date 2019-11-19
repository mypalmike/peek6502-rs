use atari800_rs::cpu::Cpu;
use atari800_rs::mem::Mem;


fn get_cpu_mem() -> (Cpu, Mem) {
    let mut mem = Mem::new(0, false);
    let mut cpu = Cpu::new();

    (cpu, mem)
}

// Return true if reached break instruction within max_ticks.
fn run(cpu: &mut Cpu, mem: &mut Mem, code: &[u8], max_ticks: i32) -> bool {
    mem.ram[0x0800..(0x0800 + code.len())].copy_from_slice(code);
    mem.ram[0x0800 + code.len()] = 0x00;
    cpu.pc = 0x0800;

    for _ in 0..max_ticks {
        cpu.tick(mem);
        if mem.get_byte(cpu.pc) == 0 {
            return true;
        }
    }

    false
}

#[test]
fn test_lda_imm() {
    let (mut cpu, mut mem) = get_cpu_mem();

    let code: [u8; 2] = [0xA9, 0x00];  // LDA IMM
    let halted = run(&mut cpu, &mut mem, &code, 100);
    assert_eq!(cpu.a, 0x00);
    assert_eq!(cpu.z, true);
    assert_eq!(cpu.n, false);

    let code: [u8; 2] = [0xA9, 0xFF];  // LDA IMM
    let halted = run(&mut cpu, &mut mem, &code, 100);
    assert_eq!(cpu.a, 0xFF);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.n, true);
}

#[test]
fn test_ldx_imm() {
    let (mut cpu, mut mem) = get_cpu_mem();

    let code: [u8; 2] = [0xA2, 0x00];  // LDX IMM
    let halted = run(&mut cpu, &mut mem, &code, 100);
    assert_eq!(cpu.x, 0x00);
    assert_eq!(cpu.z, true);
    assert_eq!(cpu.n, false);

    let code: [u8; 2] = [0xA2, 0xFF];  // LDX IMM
    let halted = run(&mut cpu, &mut mem, &code, 100);
    assert_eq!(cpu.x, 0xFF);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.n, true);
}

#[test]
fn test_ldy_imm() {
    let (mut cpu, mut mem) = get_cpu_mem();

    let code: [u8; 2] = [0xA0, 0x00];  // LDY IMM
    let halted = run(&mut cpu, &mut mem, &code, 100);
    assert_eq!(cpu.y, 0x00);
    assert_eq!(cpu.z, true);
    assert_eq!(cpu.n, false);

    let code: [u8; 2] = [0xA0, 0xFF];  // LDY IMM
    let halted = run(&mut cpu, &mut mem, &code, 100);
    assert_eq!(cpu.y, 0xFF);
    assert_eq!(cpu.z, false);
    assert_eq!(cpu.n, true);
}

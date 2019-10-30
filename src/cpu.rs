use crate::mem::Mem;

const STACK_BASE: u16 = 0x0100_u16;

pub struct Cpu {
    // Cpu registers and flags
    pc : u16,
    a : u8,
    x : u8,
    y : u8,
    p : u8,
    s : u8,
    n : bool,
    v : bool,
    d : bool,
    z : bool,
    c : bool,

    // Instruction dispatch table
    dispatch : [fn(&mut Cpu, &mut Mem); 256],
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut new_cpu = Cpu {
            pc : 0x8000, // 0x0000,
            a : 0x00,
            x : 0x00,
            y : 0x00,
            p : 0x00,
            s : 0xff, // 0xfd, ??
            n : false,
            v : false,
            d : false,
            z : false,
            c : false,
            dispatch : [Cpu::unimpl; 256],
        };

        new_cpu.dispatch[0x00 as usize] = Cpu::op_brk;
        new_cpu.dispatch[0x01 as usize] = Cpu::op_ora_izx;
        new_cpu.dispatch[0x02 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0x03 as usize] = Cpu::op_slo_izx;
        new_cpu.dispatch[0x04 as usize] = Cpu::op_nop_zp;
        new_cpu.dispatch[0x05 as usize] = Cpu::op_ora_zp;
        new_cpu.dispatch[0x06 as usize] = Cpu::op_asl_zp;
        new_cpu.dispatch[0x07 as usize] = Cpu::op_slo_zp;
        new_cpu.dispatch[0x08 as usize] = Cpu::op_php;
        new_cpu.dispatch[0x09 as usize] = Cpu::op_ora_imm;
        new_cpu.dispatch[0x0a as usize] = Cpu::op_asl;
        new_cpu.dispatch[0x0b as usize] = Cpu::op_anc_imm;
        new_cpu.dispatch[0x0c as usize] = Cpu::op_nop_abs;
        new_cpu.dispatch[0x0d as usize] = Cpu::op_ora_abs;
        new_cpu.dispatch[0x0e as usize] = Cpu::op_asl_abs;
        new_cpu.dispatch[0x0f as usize] = Cpu::op_slo_abs;
        new_cpu.dispatch[0x10 as usize] = Cpu::op_bpl_rel;
        new_cpu.dispatch[0x11 as usize] = Cpu::op_ora_izy;
        new_cpu.dispatch[0x12 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0x13 as usize] = Cpu::op_slo_izy;
        new_cpu.dispatch[0x14 as usize] = Cpu::op_nop_zpx;
        new_cpu.dispatch[0x15 as usize] = Cpu::op_ora_zpx;
        new_cpu.dispatch[0x16 as usize] = Cpu::op_asl_zpx;
        new_cpu.dispatch[0x17 as usize] = Cpu::op_slo_zpx;
        new_cpu.dispatch[0x18 as usize] = Cpu::op_clc;
        new_cpu.dispatch[0x19 as usize] = Cpu::op_ora_aby;
        new_cpu.dispatch[0x1a as usize] = Cpu::op_nop;
        new_cpu.dispatch[0x1b as usize] = Cpu::op_slo_aby;
        new_cpu.dispatch[0x1c as usize] = Cpu::op_nop_abx;

        new_cpu.dispatch[0x4c as usize] = Cpu::op_jmp_abs;
        new_cpu.dispatch[0x69 as usize] = Cpu::op_adc_imm;
        new_cpu.dispatch[0x86 as usize] = Cpu::op_stx_zp;
        new_cpu.dispatch[0xa5 as usize] = Cpu::op_lda_zp;
        new_cpu.dispatch[0xa9 as usize] = Cpu::op_lda_imm;
        new_cpu.dispatch[0xaa as usize] = Cpu::op_tax;

        new_cpu
    }

    pub fn tick(&mut self, mem : &mut Mem) {
        let pc = self.pc;
        let opcode = self.fetch_byte(mem);
        println!("opcode {:02x} at {:04x}", opcode, pc);

        self.dispatch[opcode as usize](self, mem);
    }

    pub fn unimpl(&mut self, mem : &mut Mem) {
        panic!("Unimplemented instruction");
    }

    // fn addr_for() -> u16 {
    //     let addr1 = mem.get_byte(self.pc) as u16;
    //     self.pc += 1;
    //     let addr2 = (mem.get_byte(self.pc) as u16) << 8;
    //     self.pc += 1;
    //     let addr = addr1 | addr2;
    // }

    // Fetch from program counter
    fn fetch_byte(&mut self, mem : &mut Mem) -> u8 {
        let addr = mem.get_byte(self.pc);
        self.pc += 1;
        addr
    }

    fn fetch_word(&mut self, mem : &mut Mem) -> u16 {
        let addr = mem.get_word(self.pc);
        self.pc += 2;
        addr
    }

    // Addressing modes. addr_X computes the address for an operation.
    fn fetch_addr_mode_abs(&mut self, mem : &mut Mem) -> u16 {
        self.fetch_word(mem)
    }

    fn fetch_val_mode_abs(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.fetch_addr_mode_abs(mem);
        mem.get_byte(addr)
    }

    fn fetch_addr_mode_abx(&mut self, mem : &mut Mem) -> u16 {
        self.fetch_word(mem) + self.y as u16
    }

    fn fetch_val_mode_abx(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.fetch_addr_mode_abx(mem);
        mem.get_byte(addr)
    }

    fn fetch_addr_mode_aby(&mut self, mem : &mut Mem) -> u16 {
        self.fetch_word(mem) + self.y as u16
    }

    fn fetch_val_mode_aby(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.fetch_addr_mode_aby(mem);
        mem.get_byte(addr)
    }

    fn fetch_addr_mode_zp(&mut self, mem : &mut Mem) -> u16 {
        self.fetch_byte(mem) as u16
    }

    fn fetch_val_mode_zp(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.fetch_addr_mode_zp(mem);
        mem.get_byte(addr)
    }

    fn fetch_addr_mode_zpx(&mut self, mem : &mut Mem) -> u16 {
        let offset = self.fetch_byte(mem);
        self.x.wrapping_add(offset) as u16
    }

    fn fetch_val_mode_zpx(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.fetch_addr_mode_zpx(mem);
        mem.get_byte(addr)
    }

    fn fetch_addr_mode_izx(&mut self, mem : &mut Mem) -> u16 {
        let addr = self.fetch_byte(mem) as u16 + self.x as u16;
        mem.get_word(addr)
    }

    fn fetch_val_mode_izx(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.fetch_addr_mode_izx(mem);
        mem.get_byte(addr)
    }

    fn fetch_addr_mode_izy(&mut self, mem : &mut Mem) -> u16 {
        let addr_i = self.fetch_byte(mem) as u16 + self.y as u16;
        mem.get_word(addr_i)
    }

    fn fetch_val_mode_izy(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.fetch_addr_mode_izy(mem);
        mem.get_byte(addr)
    }

    fn fetch_addr_mode_rel(&mut self, mem : &mut Mem) -> u16 {
        let pc = (self.pc - 1) as i16;
        let offset = self.fetch_byte(mem) as i8 as i16;
        (pc + offset) as u16
    }

    fn fetch_addr_mode_ind(&mut self, mem : &mut Mem) -> u16 {
        let addr_i = self.fetch_word(mem);
        mem.get_word(addr_i)
    }

    fn addr_stack(&mut self) -> u16 {
        STACK_BASE + self.s as u16
    }

    // 0x00, time 7
    fn op_brk(&mut self, mem : &mut Mem) {
        // time 7
        panic!("op_brk is not implemented");
    }

    // 0x01, time 6
    fn op_ora_izx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izx(mem);
        self.ora(mem, val);
    }

    // 0x02, unofficial
    fn op_hlt(&mut self, mem : &mut Mem) {
        panic!("cpu halt");
    }

    // 0x03, time 8, unofficial
    fn op_slo_izx(&mut self, mem : &mut Mem) {
        panic!("op_slo_izx is not implemented");
    }

    // 0x04, time 3, unofficial
    fn op_nop_zp(&mut self, mem : &mut Mem) {
        self.pc += 1;
    }

    // 0x05, time 3
    fn op_ora_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.ora(mem, val);
    }

    // 0x06, time 5
    fn op_asl_zp(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zp(mem);
        self.asl_mem(mem, addr);
    }

    // 0x07, time 5, unofficial
    fn op_slo_zp(&mut self, mem : &mut Mem) {
        panic!("op_slo_zp is not implemented");
    }

    // 0x08, time 3
    fn op_php(&mut self, mem : &mut Mem) {
        let status = self.status();
        self.stack_push_byte(mem, status);
    }

    // 0x09, time 2
    fn op_ora_imm(&mut self, mem : &mut Mem) {
        let val = self.fetch_byte(mem);
        self.ora(mem, val);
    }

    // 0x0a, time 2
    fn op_asl(&mut self, mem : &mut Mem) {
        let val = self.a;
        self.a = self.asl_val(mem, val);
    }

    // 0x0b, time 2, unofficial
    fn op_anc_imm(&mut self, mem : &mut Mem) {
        panic!("op_anc_imm is not implemented");
    }

    // 0x0c, time 4, unofficial
    fn op_nop_abs(&mut self, mem : &mut Mem) {
        self.pc += 3;
    }

    // 0x0d, time 4
    fn op_ora_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.ora(mem, val);
    }

    // 0x0e, time 6
    fn op_asl_abs(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abs(mem);
        self.asl_mem(mem, addr);
    }

    // 0x0f, time 6, unofficial
    fn op_slo_abs(&mut self, mem : &mut Mem) {
        panic!("op_nop_abs is not implemented");
    }

    // 0x10, time 2+
    fn op_bpl_rel(&mut self, mem : &mut Mem) {
        // TODO : review.
        if self.n {
            self.pc += 1;
        } else {
            let offset = mem.get_byte(self.pc) as i8 as i16;
            self.pc = (self.pc as i16 + offset) as u16;
        }
    }

    // 0x11, time 5+
    fn op_ora_izy(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izy(mem);
        self.ora(mem, val);
    }

    // 0x12 is hlt

    // 0x13, time 8, unofficial
    fn op_slo_izy(&mut self, mem : &mut Mem) {
        panic!("op_slo_izy is not implemented");
    }

    // 0x14, time 4, unofficial
    fn op_nop_zpx(&mut self, mem : &mut Mem) {
        self.pc += 1;
    }

    // 0x15, time 4
    fn op_ora_zpx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zpx(mem);
        self.ora(mem, val);
    }

    // 0x16, time 6
    fn op_asl_zpx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zpx(mem);
        self.asl_mem(mem, addr);
    }

    // 0x17, time 6
    fn op_slo_zpx(&mut self, mem : &mut Mem) {
        panic!("op_slo_zpx is not implemented");
    }

    // 0x18, time 2
    fn op_clc(&mut self, mem : &mut Mem) {
        self.c = false;
    }

    // 0x19, time 4
    fn op_ora_aby(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_aby(mem);
        self.ora(mem, val);
    }

    // 0x1a, time 2, unofficial
    fn op_nop(&mut self, mem : &mut Mem) {
    }

    // 0x1b, time 7, unofficial
    fn op_slo_aby(&mut self, mem : &mut Mem) {
        panic!("op_slo_aby is not implemented");
    }

    // 0x1c, time 4+, unofficial
    fn op_nop_abx(&mut self, mem : &mut Mem) {
        self.pc += 2;
    }

    // 0x1d, time 4
    fn op_ora_abx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abx(mem);
        self.ora(mem, val);
    }

    // 0x1e, time 7
    fn op_asl_abx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abx(mem);
        self.asl_mem(mem, addr);
    }

    // 0x1f, time 7, unofficial
    fn op_slo_abx(&mut self, mem : &mut Mem) {
        panic!("op_slo_abx is not implemented");
    }

    // 0x20, time 6
    fn op_jsr_abs(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abs(mem);
        self.stack_push_word(mem, self.pc - 1);
        self.pc = addr;
    }

    // 0x21, time 6
    fn op_and_izx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izx(mem);
        self.and(mem, val);
    }

    // 0x22 hlt

    // 0x23, time 7, unofficial
    fn op_rla_izx(&mut self, mem : &mut Mem) {
        panic!("op_rla_izx is not implemented");
    }

    // 0x24, time 3
    fn op_bit_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.bit(mem, val);
    }

    // 0x25, time 3
    fn op_and_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.and(mem, val);
    }

    // 0x26, time 5
    fn op_rol_zp(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zp(mem);
        self.rol_mem(mem, addr);
    }

    // 0x27, time 5, unofficial
    fn op_rla_zp(&mut self, mem : &mut Mem) {
         panic!("op_rla_zp is not implemented");
    }

    // 0x28, time 4
    fn op_plp(&mut self, mem : &mut Mem) {
        self.p = self.stack_pop_byte(mem);
    }

    // 0x29, time 2
    fn op_and_imm(&mut self, mem : &mut Mem) {
        let val = self.fetch_byte(mem);
        self.and(mem, val);
    }

    // 0x2a, time 2
    fn op_rol(&mut self, mem : &mut Mem) {
        let val = self.a;
        let new_val = self.rol_val(mem, val);
        self.a = new_val;
    }

    // 0x2b op_anc_imm (see above)

    // 0x2c, time 4
    fn op_bit_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.bit(mem, val);
    }

    // 0x2d, time 4
    fn op_and_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.and(mem, val);
    }

    // 0x2e, time 6
    fn op_rol_abs(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abs(mem);
        self.rol_mem(mem, addr);
    }

    // 0x2f, time 6, unofficial
    fn op_rla_abs(&mut self, mem : &mut Mem) {
         panic!("op_rla_zp is not implemented");
    }

    // 0x30, time 2+
    fn op_bmi_rel(&mut self, mem : &mut Mem) {
        // TODO : Review
        if self.n {
            let offset = mem.get_byte(self.pc) as i8 as i16;
            self.pc = (self.pc as i16 + offset) as u16;
        } else {
            self.pc += 1;
        }
    }

    // 0x31, time 5+
    fn op_and_izy(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izy(mem);
        self.and(mem, val);
    }

    // 0x32 hlt

    // 0x33, time 8
    fn op_rla_izy(&mut self, mem : &mut Mem) {
        panic!("op_rla_izy is not implemented");
    }

    // 0x34 nop_zpx

    // 0x35, time 4
    fn op_and_zpx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zpx(mem);
        self.and(mem, val);
    }

    // 0x36, time 6
    fn op_rol_zpx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zpx(mem);
        self.rol_mem(mem, addr);
    }

    // 0x37, time 6, unofficial
    fn op_rla_zpx(&mut self, mem : &mut Mem) {
        panic!("op_rla_zpx is not implemented");
    }

    // 0x38, time 2
    fn op_sec(&mut self, mem : &mut Mem) {
        self.c = true;
    }

    // 0x39, time 4
    fn op_and_aby(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_aby(mem);
        self.and(mem, val);
    }

    // 0x3a nop

    // 0x3b, time 7, unofficial
    fn op_rla_aby(&mut self, mem : &mut Mem) {
        panic!("op_rla_aby is not implemented");
    }

    // 0x3c nop_abx

    // 0x3d, time 4+
    fn op_and_abx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abx(mem);
        self.and(mem, val);
    }

    // 0x3e, time 7
    fn op_rol_abx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abx(mem);
        self.rol_mem(mem, addr);
    }

    // 0x3f, time 7, unofficial
    fn op_rla_abx(&mut self, mem : &mut Mem) {
        panic!("op_rla_aby is not implemented");
    }

    // 0x40, time 6
    fn op_rti(&mut self, mem : &mut Mem) {
        // TODO
        panic!("op_rti is not implemented");
    }

    // 0x41, time 6
    fn op_eor_izx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izx(mem);
        self.eor(mem, val);
    }

    // 0x42 hlt

    // 0x43, time 8, unofficial
    fn op_sre_izx(&mut self, mem : &mut Mem) {
        panic!("op_sre_izx is not implemented");
    }

    // 0x44 op_nop_zp

    // 0x45, time 3
    fn op_eor_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.eor(mem, val);
    }

    // 0x46, time 5
    fn op_lsr_zp(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zp(mem);
        self.lsr_mem(mem, addr);
    }

    // 0x47, time 5
    fn op_sre_zp(&mut self, mem : &mut Mem) {
        panic!("op_sre_zp is not implemented");
    }

    // 0x48, time 3
    fn op_pha(&mut self, mem : &mut Mem) {
        self.stack_push_byte(mem, self.a);
    }

    // 0x49, time 2
    fn op_eor_imm(&mut self, mem : &mut Mem) {
        let val = self.fetch_byte(mem);
        self.eor(mem, val);
    }

    // 0x4a, time 2
    fn op_lsr(&mut self, mem : &mut Mem) {
        let val = self.a;
        let new_val = self.lsr_val(mem, val);
        self.a = new_val;
    }

    // 0x4b alr_imm

    // 0x4c, time 3
    fn op_jmp_abs(&mut self, mem : &mut Mem) {
        panic!("op_jmp_abs is not implemented");

        // let addr = self.fetch_addr_mode_abs(mem);
        // self.jmp(addr);
        // println!("jmp_abs 0x{:04x}", addr);
    }

    // 0x4d, time 4
    fn op_eor_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.eor(mem, val);
    }

    // 0x4e, time 6
    fn op_lsr_abs(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abs(mem);
        self.lsr_mem(mem, addr);
    }

    // 0x4f, time 6, unofficial
    fn op_sre_abs(&mut self, mem : &mut Mem) {
        panic!("op_jmp_abs is not implemented");
    }

    // 0x50, time 2+
    fn op_bvc_rel(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_rel(mem);
        self.bvc(mem, addr);
    }

    // 0x51, time 5
    fn op_eor_izy(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izy(mem);
        self.eor(mem, val);
    }

    // 0x52 hlt

    // 0x53 sre_izy

    // 0x54 nop_zpx

    // 0x55, time 4
    fn op_eor_zpx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zpx(mem);
        self.eor(mem, val);
    }

    // 0x56, time 6
    fn op_lsr_zpx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zpx(mem);
        self.lsr_mem(mem, addr);
    }

    // 0x57, time 6, unofficial
    fn op_sre_zpx(&mut self, mem : &mut Mem) {
        panic!("op_sre_zpx is not implemented");
    }

    // 0x58, time 2
    fn op_cli(&mut self, mem : &mut Mem) {
        // TODO
        panic!("op_cli is not implemented");
    }

    // 0x59, time 4+
    fn op_eor_aby(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_aby(mem);
        self.eor(mem, val);
    }

    // 0x5a nop

    // 0x5b, time 7, unofficial
    fn op_sre_aby(&mut self, mem : &mut Mem) {
        panic!("op_sre_aby is not implemented");
    }

    // 0x5c nop_abx

    // 0x5d, time 4
    fn op_eor_abx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abx(mem);
        self.eor(mem, val);
    }

    // 0x5e, time 7
    fn op_lsr_abx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abx(mem);
        self.lsr_mem(mem, addr);
    }

    // 0x5f, time 7, unofficial
    fn op_sre_abx(&mut self, mem : &mut Mem) {
        panic!("op_sre_abx is not implemented");
    }

    // 0x60, time 6
    fn op_rts(&mut self, mem : &mut Mem) {
        let addr = self.stack_pop_word(mem) + 1;
        self.pc = addr;
    }

    // 0x61, time 6
    fn op_adc_izx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izx(mem);
        self.adc(mem, val);
    }

    // 0x62 hlt

    // 0x63, time 8, unofficial
    fn op_rra_izx(&mut self, mem : &mut Mem) {
        panic!("op_rra_izx is not implemented");
    }

    // 0x64 nop_zp

    // 0x65, time 3
    fn op_adc_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.adc(mem, val);
    }

    // 0x66, time 5
    fn op_ror_zp(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zp(mem);
        self.ror_mem(mem, addr);
    }

    // 0x67, time 5
    fn op_rra_zp(&mut self, mem : &mut Mem) {
        panic!("op_rra_zp is not implemented");
    }

    // 0x68, time 4
    fn op_pla(&mut self, mem : &mut Mem) {
        let val = self.stack_pop_byte(mem);
        self.a = val;
    }

    // 0x69, time 2
    fn op_adc_imm(&mut self, mem : &mut Mem) {
        // TODO : Deal with setting, checking flags
        let val = self.fetch_byte(mem);
        self.adc(mem, val);
        println!("adc_imm {:02x} -> {:02x}", val, self.a);
    }

    // 0x6a, time 2
    fn op_ror(&mut self, mem : &mut Mem) {
        let val = self.a;
        self.a = self.ror_val(mem, val);
    }

    // 0x6b, time 2, unofficial
    fn op_arr_imm(&mut self, mem : &mut Mem) {
        panic!("op_arr_imm is not implemented");
    }

    // 0x6c, time 5
    fn op_jmp_ind(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_ind(mem);
        self.jmp(mem, addr);
    }

    // 0x6d, time 4
    fn op_adc_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.adc(mem, val);
    }

    // 0x6e, time 6
    fn op_ror_abs(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abs(mem);
        self.ror_mem(mem, addr);
    }

    // 0x6f, time 6
    fn op_rra_abs(&mut self, mem : &mut Mem) {
        panic!("op_rra_abs is not implemented");
    }

    // 0x70, time 2
    fn op_bvs_rel(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_rel(mem);
        self.bvs(mem, addr);
    }

    // 0x71, time 5+
    fn op_adc_izy(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izy(mem);
        self.adc(mem, val);
    }

    // 0x72 hlt

    // 0x73, time 8, unofficial
    fn op_rra_izy(&mut self, mem : &mut Mem) {
        panic!("op_rra_izy is not implemented");
    }

    // 0x74 nop_zpx

    // 0x75, time 4
    fn op_adc_zpx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zpx(mem);
        self.adc(mem, val);
    }

    // 0x76, time 6
    fn op_ror_zpx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zpx(mem);
        self.ror_mem(mem, addr);
    }

    // 0x77, time 6, unofficial
    fn op_rra_zpx(&mut self, mem : &mut Mem) {
        panic!("op_rra_izy is not implemented");
    }

    // 0x78, time 2
    fn op_sei(&mut self, mem : &mut Mem) {
        // TODO
        panic!("op_sei is not implemented");
    }

    // 0x79, time 4+
    fn op_adc_aby(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_aby(mem);
        self.adc(mem, val);
    }

    // 0x7a nop

    // 0x7b, time 7
    fn op_rda_aby(&mut self, mem : &mut Mem) {
        panic!("op_rda_aby is not implemented");
    }

    // 0x7c nop_abx

    // 0x7d, time 4
    fn op_adc_abx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abx(mem);
        self.adc(mem, val);
    }

    // 0x7e, time 7
    fn op_ror_abx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abx(mem);
        self.ror_mem(mem, addr);
    }

    // 0x7f, time 7
    fn op_rra_abx(&mut self, mem : &mut Mem) {
        panic!("op_rra_abx is not implemented");
    }

    // 0x80 nop_imm

    // 0x81, time 6
    fn op_sta_izx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abx(mem);
        self.sta(mem, addr);
    }

    // 0x82 nop_imm

    // 0x83
    fn op_sax_izx(&mut self, mem : &mut Mem) {
        panic!("op_rra_abx is not implemented");
    }

    // 0x84, time 3
    fn op_sty_zp(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zp(mem);
        self.sty(mem, addr);
    }

    // 0x85, time 3
    fn op_sta_zp(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zp(mem);
        self.sta(mem, addr);
    }

    // 0x86, time 3
    fn op_stx_zp(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zp(mem);
        self.stx(mem, addr);
        println!("stx_zp {:04x} {:02x}", addr, self.x);
    }

    // 0x87, time 3
    fn op_sax_zp(&mut self, mem : &mut Mem) {
        panic!("op_sax_zp is not implemented");
    }

    // 0x88, time 2
    fn op_dey(&mut self, mem : &mut Mem) {
        self.y -= 1;
    }

    // 0x89 nop_imm

    // 0x8a, time 2
    fn op_txa(&mut self, mem : &mut Mem) {
        self.a = self.x;
    }

    // 0x8b, time 2, unofficial
    fn op_xaa_imm(&mut self, mem : &mut Mem) {
        panic!("op_xaa_imm is not implemented");
    }

    // 0x8c, time 4
    fn op_sty_abs(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abs(mem);
        self.sty(mem, addr);
    }

    // 0x8d, time 4
    fn op_sta_abs(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abs(mem);
        self.sta(mem, addr);
    }

    // 0x8e, time 4
    fn op_stx_abs(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abs(mem);
        self.stx(mem, addr);
    }

    // 0x8f, time 4, unofficial
    fn op_sax_abs(&mut self, mem : &mut Mem) {
        panic!("op_sax_abs is not implemented");
    }

    // 0x90, time 2+
    fn op_bcc_rel(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_rel(mem);
        self.bcc(mem, addr);
    }

    // 0x91, time 6
    fn op_sta_izy(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_izy(mem);
        self.sta(mem, addr);
    }










    // 0xa5, time 3
    fn op_lda_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.lda(mem, val);
        println!("lda_zp {:02x}", self.a);
    }

    // 0xa9, time 2
    fn op_lda_imm(&mut self, mem : &mut Mem) {
        let val = self.fetch_byte(mem);
        self.lda(mem, val);
        println!("lda_imm {:02x}", self.a);
    }

    // 0xaa, time 2
    fn op_tax(&mut self, mem : &mut Mem) {
        self.x = self.a;
        println!("tax {:02x}", self.x);
    }

    // Implementations of core functionality once the address has been
    // computed
    fn adc(&mut self, mem : &mut Mem, val : u8) {
        // TODO : Verify this
        // TODO : Deal with BCD
        self.a += val;
        if self.c {
            self.a += 1;
        }
        self.compute_nz();
    }

    fn and(&mut self, mem : &mut Mem, val : u8) {
        self.a = self.a & val;
        self.compute_nz();
    }

    fn asl_mem(&mut self, mem : &mut Mem, addr: u16) {
        let val = mem.get_byte(addr);
        let new_val = self.asl_val(mem, val);
        mem.set_byte(addr, new_val);
    }

    fn asl_val(&mut self, mem : &mut Mem, val: u8) -> u8 {
        let (new_val, overflow) = val.overflowing_shl(1);
        self.c = overflow;
        self.compute_nz();
        new_val
    }

    fn bcs(&mut self, mem : &mut Mem, addr: u16) {
        if self.c {
            self.pc = addr;
        }
    }

    fn bcc(&mut self, mem : &mut Mem, addr: u16) {
        if !self.c {
            self.pc = addr;
        }
    }

    fn bvc(&mut self, mem : &mut Mem, addr: u16) {
        if !self.v {
            self.pc = addr;
        }
    }

    fn bvs(&mut self, mem : &mut Mem, addr: u16) {
        if self.v {
            self.pc = addr;
        }
    }

    //	Set flags only. n and v are set to val bits 7 and 6. z is AND of a and val
    fn bit(&mut self, mem : &mut Mem, val: u8) {
        self.n = val & 0x80 != 0;
        self.v = val & 0x40 != 0;
        self.z = val & self.a == 0;
    }

    fn eor(&mut self, mem : &mut Mem, val: u8) {
        self.a = self.a ^ val;
        self.compute_nz();
    }

    fn jmp(&mut self, mem : &mut Mem, addr: u16) {
        self.pc = addr;
    }

    fn lda(&mut self, mem : &mut Mem, val: u8) {
        self.a = val;
        self.compute_nz();
    }

    fn lsr_mem(&mut self, mem : &mut Mem, addr: u16) {
        let val = mem.get_byte(addr);
        let new_val = self.lsr_val(mem, val);
        mem.set_byte(addr, new_val);
    }

    fn lsr_val(&mut self, mem : &mut Mem, val: u8) -> u8 {
        self.c = val & 0x01 != 0;
        val >> 1;
        self.compute_nz_val(val);
        val
    }

    fn ora(&mut self, mem : &mut Mem, val : u8) {
        self.a = self.a | val;
        self.compute_nz();
    }

    fn rol_mem(&mut self, mem : &mut Mem, addr: u16) {
        let val = mem.get_byte(addr);
        let new_val = self.rol_val(mem, val);
        mem.set_byte(addr, new_val);
    }

    fn rol_val(&mut self, mem : &mut Mem, val: u8) -> u8 {
        let (val2, overflow) = val.overflowing_shl(1);
        let c = self.c;
        self.c = overflow;
        let new_val = val2 | if c {0x01} else {0x00};
        self.compute_nz_val(new_val);
        new_val
    }

    fn ror_mem(&mut self, mem : &mut Mem, addr: u16) {
        let val = mem.get_byte(addr);
        let new_val = self.ror_val(mem, val);
        mem.set_byte(addr, new_val);
    }

    fn ror_val(&mut self, mem : &mut Mem, val: u8) -> u8 {
        let new_c = val & 0x01 == 0x01;
        let val2 = val >> 1;
        let c = self.c;
        let new_val = val2 | if c{0x80} else {0x00};
        self.c = new_c;
        self.compute_nz_val(new_val);
        new_val
    }

    fn sta(&mut self, mem : &mut Mem, addr : u16) {
        mem.set_byte(addr, self.a);
    }

    fn stx(&mut self, mem : &mut Mem, addr : u16) {
        mem.set_byte(addr, self.x);
    }

    fn sty(&mut self, mem : &mut Mem, addr : u16) {
        mem.set_byte(addr, self.y);
    }

    fn compute_nz(&mut self) {
        self.compute_nz_val(self.a);
    }

    fn compute_nz_val(&mut self, val: u8) {
        self.n = val >= 0x80;
        self.z = val == 0;
    }

    // Stack functions
    fn stack_push_byte(&mut self, mem : &mut Mem, val : u8) {
        let addr = self.addr_stack();
        mem.set_byte(addr, val);
        self.s -= 1;
    }

    fn stack_pop_byte(&mut self, mem : &mut Mem) -> u8 {
        self.s += 1;
        let addr = self.addr_stack();
        mem.get_byte(addr)
    }

    fn stack_push_word(&mut self, mem : &mut Mem, val : u16) {
        let addr = self.addr_stack();
        self.s -= 1;
        mem.set_word(addr, val);
        self.s -= 1;
    }

    fn stack_pop_word(&mut self, mem : &mut Mem) -> u16 {
        self.s += 1;
        let addr = self.addr_stack();
        let val = mem.get_word(addr);
        self.s += 1;
        val
    }

    // fn jmp_abs(&mut self, mem : &mut Mem) {
    //     self.pc += 1;
    //     let addr1 = mem.get_byte(self.pc) as u16;
    //     self.pc += 1;
    //     let addr2 = (mem.get_byte(self.pc) as u16) << 8;
    //     self.pc += 1;
    //     let addr = addr1 | addr2;
    //     self.pc = addr;
    //     println!("jmp_abs 0x{:04x}", addr);
    // }

    fn status(&self) -> u8 {
        let mut st = 0x20;
        if self.n {
            st = st | 0x80;
        }
        if self.v {
            st = st | 0x40;
        }
        // TODO : brk bit
        if self.d {
            st = st | 0x08;
        }
        // TODO : interrupt bit
        if self.z {
            st = st | 0x02;
        }
        if self.c {
            st = st | 0x01;
        }

        st
    }
}

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
        


        new_cpu.dispatch[0x4c as usize] = Cpu::op_jmp_abs;
        new_cpu.dispatch[0x69 as usize] = Cpu::op_adc_imm;
        new_cpu.dispatch[0x86 as usize] = Cpu::op_stx_zp;
        new_cpu.dispatch[0xa5 as usize] = Cpu::op_lda_zp;
        new_cpu.dispatch[0xa9 as usize] = Cpu::op_lda_imm;
        new_cpu.dispatch[0xaa as usize] = Cpu::op_tax;

        new_cpu
    }

    pub fn tick(&mut self, mem : &mut Mem) {
        let opcode = mem.get_byte(self.pc);
        println!("opcode {:02x} at {:04x}", opcode, self.pc);

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

    // Addressing modes. addr_X computes the address for an operation.
    fn addr_mode_abs(&mut self, mem : &mut Mem) -> u16 {
        mem.get_word(self.pc)
    }

    fn val_mode_abs(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.addr_mode_abs(mem);
        mem.get_byte(addr)
    }

    fn addr_mode_aby(&mut self, mem : &mut Mem) -> u16 {
        mem.get_word(self.pc) + self.y as u16
    }

    fn val_mode_aby(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.addr_mode_abs(mem);
        mem.get_byte(addr)
    }

    fn addr_mode_zp(&mut self, mem : &mut Mem) -> u16 {
        mem.get_byte(self.pc) as u16
    }

    fn val_mode_zp(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.addr_mode_zp(mem);
        mem.get_byte(addr)
    }

    fn addr_mode_zpx(&mut self, mem : &mut Mem) -> u16 {
        let offset = mem.get_byte(self.pc);
        self.x.wrapping_add(offset) as u16
    }

    fn val_mode_zpx(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.addr_mode_zpx(mem);
        mem.get_byte(addr)
    }

    fn addr_mode_izx(&mut self, mem : &mut Mem) -> u16 {
        let addr_i = mem.get_byte(self.pc) as u16 + self.x as u16;
        mem.get_word(addr_i)
    }

    fn val_mode_izx(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.addr_mode_izx(mem);
        mem.get_byte(addr)
    }

    fn addr_mode_izy(&mut self, mem : &mut Mem) -> u16 {
        let addr_i = mem.get_byte(self.pc) as u16 + self.y as u16;
        mem.get_word(addr_i)
    }

    fn val_mode_izy(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.addr_mode_izy(mem);
        mem.get_byte(addr)
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
        self.pc += 1;
        let val = self.val_mode_izx(mem);
        self.pc += 1;
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
        self.pc += 2;
    }

    // 0x05, time 3
    fn op_ora_zp(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let val = self.val_mode_zp(mem);
        self.pc += 1;
        self.ora(mem, val);
    }

    // 0x06, time 5
    fn op_asl_zp(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let addr = self.addr_mode_zp(mem);
        self.pc += 1;
        self.asl_mem(mem, addr);
    }

    // 0x07, time 5, unofficial
    fn op_slo_zp(&mut self, mem : &mut Mem) {
        panic!("op_slo_zp is not implemented");
    }

    // 0x08, time 3
    fn op_php(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let status = self.status();
        self.stack_push(mem, status);
    }

    // 0x09, time 2
    fn op_ora_imm(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let val = mem.get_byte(self.pc);
        self.pc += 1;
        self.ora(mem, val);
    }

    // 0x0a, time 2
    fn op_asl(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let val = self.a;
        let new_val = self.asl_val(mem, val);
        self.a = new_val;
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
        self.pc += 1;
        let val = self.val_mode_abs(mem);
        self.pc += 2;
        self.ora(mem, val);
    }

    // 0x0e, time 6
    fn op_asl_abs(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let addr = self.addr_mode_abs(mem);
        self.pc += 2;
        self.asl_mem(mem, addr);
    }

    // 0x0f, time 6, unofficial
    fn op_slo_abs(&mut self, mem : &mut Mem) {
        panic!("op_nop_abs is not implemented");
    }

    // 0x10, time 2+
    fn op_bpl_rel(&mut self, mem : &mut Mem) {
        self.pc += 1;
        if self.n {
            self.pc += 1;
        } else {
            let offset = mem.get_byte(self.pc) as i8 as i16;
            self.pc = (self.pc as i16 + offset) as u16;
        }
    }
    
    // 0x11, time 5+
    fn op_ora_izy(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let val = self.val_mode_izy(mem);
        self.pc += 1;
        self.ora(mem, val);        
    }

    // 0x12 is hlt

    // 0x13, time 8, unofficial
    fn op_slo_izy(&mut self, mem : &mut Mem) {
        panic!("op_slo_izy is not implemented");
    }

    // 0x14, time 4, unofficial
    fn op_nop_zpx(&mut self, mem : &mut Mem) {
        self.pc += 2;
    }

    // 0x15, time 4
    fn op_ora_zpx(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let val = self.val_mode_zpx(mem);
        self.pc += 1;
        self.ora(mem, val);
    }

    // 0x16, time 6
    fn op_asl_zpx(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let addr = self.addr_mode_zpx(mem);
        self.pc += 1;
        self.asl_mem(mem, addr);
    }

    // 0x17, time 6
    fn op_slo_zpx(&mut self, mem : &mut Mem) {
        panic!("op_slo_zpx is not implemented");
    }

    // 0x18, time 2
    fn op_clc(&mut self, mem : &mut Mem) {
        self.pc += 1;
        self.c = false;
    }

    // 0x19, time 4
    fn op_ora_aby(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let val = self.val_mode_aby(mem);
        self.pc += 2;
        self.ora(mem, val);
    }

    // 0x1a, time 2, unofficial
    fn op_nop(&mut self, mem : &mut Mem) {
        self.pc += 1;
    }

    // 0x1b, time 7, unofficial
    fn op_slo_aby(&mut self, mem : &mut Mem) {
        panic!("op_slo_aby is not implemented");
    }

    // 0x1c, time 4+, unofficial
    fn op_nop_abx(&mut self, mem : &mut Mem) {
        self.pc += 3;
    }






    // 0x69, time 2
    fn op_adc_imm(&mut self, mem : &mut Mem) {
        // TODO : Deal with setting, checking flags
        self.pc += 1;
        let val = mem.get_byte(self.pc);
        self.pc += 1;
        self.a += val;
        if self.c {
            self.a += 1;
        }
        println!("adc_imm {:02x} -> {:02x}", val, self.a);
    }

    // 0xa5, time 3
    fn op_lda_zp(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let val = self.val_mode_zp(mem);
        self.pc += 1;
        self.lda(mem, val);
        println!("lda_zp {:02x}", self.a);
    }

    // 0xa9, time 2
    fn op_lda_imm(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let val = mem.get_byte(self.pc);
        self.pc += 1;
        self.lda(mem, val);
        println!("lda_imm {:02x}", self.a);
    }

    // 0xaa, time 2
    fn op_tax(&mut self, mem : &mut Mem) {
        self.pc += 1;
        self.x = self.a;
        println!("tax {:02x}", self.x);
    }

    // 0x86, time 3
    fn op_stx_zp(&mut self, mem : &mut Mem) {
        self.pc += 1;
        let addr = self.addr_mode_zp(mem);
        self.pc += 1;
        self.stx(mem, addr);
        println!("stx_zp {:04x} {:02x}", addr, self.x);
    }

    // 0x4c, time 3
    fn op_jmp_abs(&mut self, mem : &mut Mem) {
        panic!("op_jmp_abs is not implemented");

        // self.pc += 1;
        // let addr = self.addr_mode_abs(mem);
        // self.pc = addr;
        // println!("jmp_abs 0x{:04x}", addr);
    }

    // Implementations of core functionality once the address has been
    // computed
    fn asl_mem(&mut self, mem : &mut Mem, addr: u16) {
        let val = mem.get_byte(addr);
        let new_val = self.asl_val(mem, val);
        mem.set_byte(addr, new_val);
    }

    fn asl_val(&mut self, mem : &mut Mem, val: u8) -> u8 {
        let (new_val, overflow) = val.overflowing_shl(1);
        self.c = overflow;
        new_val
    }

    fn lda(&mut self, mem : &mut Mem, val: u8) {
        self.a = val;
    }

    fn ora(&mut self, mem : &mut Mem, val : u8) {
        self.a = self.a | val;
    }    

    fn stx(&mut self, mem : &mut Mem, addr : u16) {
        mem.set_byte(addr, self.x);
    }

    // Stack functions
    fn stack_push(&mut self, mem : &mut Mem, val : u8) {
        let addr = self.addr_stack();
        mem.set_byte(addr, val);
        self.s -= 1;
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

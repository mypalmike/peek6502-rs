use crate::debugger::Debugger;
use crate::mem::Mem;

const STACK_BASE: u16 = 0x0100_u16;

pub struct Cpu {
    // Cpu registers and flags
    pc : u16,
    a : u8,
    x : u8,
    y : u8,
    // p : u8,
    s : u8,
    n : bool,
    v : bool,
    d : bool,
    z : bool,
    c : bool,

    // Instruction dispatch table
    dispatch : [fn(&mut Cpu, &mut Mem); 256],
    debugger : Debugger,
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut new_cpu = Cpu {
            pc : 0x0000,
            a : 0x00,
            x : 0x00,
            y : 0x00,
            // p : 0x00,
            s : 0xff, // 0xfd, ??
            n : false,
            v : false,
            d : false,
            z : false,
            c : false,
            dispatch : [Cpu::unimpl; 256],
            debugger : Debugger::new(),
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
        new_cpu.dispatch[0x1d as usize] = Cpu::op_ora_abx;
        new_cpu.dispatch[0x1e as usize] = Cpu::op_asl_abx;
        new_cpu.dispatch[0x1f as usize] = Cpu::op_slo_abx;

        new_cpu.dispatch[0x20 as usize] = Cpu::op_jsr_abs;
        new_cpu.dispatch[0x21 as usize] = Cpu::op_and_izx;
        new_cpu.dispatch[0x22 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0x23 as usize] = Cpu::op_rla_izx;
        new_cpu.dispatch[0x24 as usize] = Cpu::op_bit_zp;
        new_cpu.dispatch[0x25 as usize] = Cpu::op_and_zp;
        new_cpu.dispatch[0x26 as usize] = Cpu::op_rol_zp;
        new_cpu.dispatch[0x27 as usize] = Cpu::op_rla_zp;
        new_cpu.dispatch[0x28 as usize] = Cpu::op_plp;
        new_cpu.dispatch[0x29 as usize] = Cpu::op_and_imm;
        new_cpu.dispatch[0x2a as usize] = Cpu::op_rol;
        new_cpu.dispatch[0x2b as usize] = Cpu::op_anc_imm;
        new_cpu.dispatch[0x2c as usize] = Cpu::op_bit_abs;
        new_cpu.dispatch[0x2d as usize] = Cpu::op_and_abs;
        new_cpu.dispatch[0x2e as usize] = Cpu::op_rol_abs;
        new_cpu.dispatch[0x2f as usize] = Cpu::op_rla_abs;

        new_cpu.dispatch[0x30 as usize] = Cpu::op_bmi_rel;
        new_cpu.dispatch[0x31 as usize] = Cpu::op_and_izy;
        new_cpu.dispatch[0x32 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0x33 as usize] = Cpu::op_rla_izy;
        new_cpu.dispatch[0x34 as usize] = Cpu::op_nop_zpx;
        new_cpu.dispatch[0x35 as usize] = Cpu::op_and_zpx;
        new_cpu.dispatch[0x36 as usize] = Cpu::op_rol_zpx;
        new_cpu.dispatch[0x37 as usize] = Cpu::op_rla_zpx;
        new_cpu.dispatch[0x38 as usize] = Cpu::op_sec;
        new_cpu.dispatch[0x39 as usize] = Cpu::op_and_aby;
        new_cpu.dispatch[0x3a as usize] = Cpu::op_nop;
        new_cpu.dispatch[0x3b as usize] = Cpu::op_rla_aby;
        new_cpu.dispatch[0x3c as usize] = Cpu::op_nop_abx;
        new_cpu.dispatch[0x3d as usize] = Cpu::op_and_abx;
        new_cpu.dispatch[0x3e as usize] = Cpu::op_rol_abx;
        new_cpu.dispatch[0x3f as usize] = Cpu::op_rla_abx;

        new_cpu.dispatch[0x40 as usize] = Cpu::op_rti;
        new_cpu.dispatch[0x41 as usize] = Cpu::op_eor_izx;
        new_cpu.dispatch[0x42 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0x43 as usize] = Cpu::op_sre_izx;
        new_cpu.dispatch[0x44 as usize] = Cpu::op_nop_zp;
        new_cpu.dispatch[0x45 as usize] = Cpu::op_eor_zp;
        new_cpu.dispatch[0x46 as usize] = Cpu::op_lsr_zp;
        new_cpu.dispatch[0x47 as usize] = Cpu::op_sre_zp;
        new_cpu.dispatch[0x48 as usize] = Cpu::op_pha;
        new_cpu.dispatch[0x49 as usize] = Cpu::op_eor_imm;
        new_cpu.dispatch[0x4a as usize] = Cpu::op_lsr;
        new_cpu.dispatch[0x4b as usize] = Cpu::op_alr_imm;
        new_cpu.dispatch[0x4c as usize] = Cpu::op_jmp_abs;
        new_cpu.dispatch[0x4d as usize] = Cpu::op_eor_abs;
        new_cpu.dispatch[0x4e as usize] = Cpu::op_lsr_abs;
        new_cpu.dispatch[0x4f as usize] = Cpu::op_sre_abs;

        new_cpu.dispatch[0x50 as usize] = Cpu::op_bvc_rel;
        new_cpu.dispatch[0x51 as usize] = Cpu::op_eor_izy;
        new_cpu.dispatch[0x52 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0x53 as usize] = Cpu::op_sre_izy;
        new_cpu.dispatch[0x54 as usize] = Cpu::op_nop_zpx;
        new_cpu.dispatch[0x55 as usize] = Cpu::op_eor_zpx;
        new_cpu.dispatch[0x56 as usize] = Cpu::op_lsr_zpx;
        new_cpu.dispatch[0x57 as usize] = Cpu::op_sre_zpx;
        new_cpu.dispatch[0x58 as usize] = Cpu::op_cli;
        new_cpu.dispatch[0x59 as usize] = Cpu::op_eor_aby;
        new_cpu.dispatch[0x5a as usize] = Cpu::op_nop;
        new_cpu.dispatch[0x5b as usize] = Cpu::op_sre_aby;
        new_cpu.dispatch[0x5c as usize] = Cpu::op_nop_abx;
        new_cpu.dispatch[0x5d as usize] = Cpu::op_eor_abx;
        new_cpu.dispatch[0x5e as usize] = Cpu::op_lsr_abx;
        new_cpu.dispatch[0x5f as usize] = Cpu::op_sre_abx;

        new_cpu.dispatch[0x60 as usize] = Cpu::op_rts;
        new_cpu.dispatch[0x61 as usize] = Cpu::op_adc_izx;
        new_cpu.dispatch[0x62 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0x63 as usize] = Cpu::op_rra_izx;
        new_cpu.dispatch[0x64 as usize] = Cpu::op_nop_zp;
        new_cpu.dispatch[0x65 as usize] = Cpu::op_adc_zp;
        new_cpu.dispatch[0x66 as usize] = Cpu::op_ror_zp;
        new_cpu.dispatch[0x67 as usize] = Cpu::op_rra_zp;
        new_cpu.dispatch[0x68 as usize] = Cpu::op_pla;
        new_cpu.dispatch[0x69 as usize] = Cpu::op_adc_imm;
        new_cpu.dispatch[0x6a as usize] = Cpu::op_ror;
        new_cpu.dispatch[0x6b as usize] = Cpu::op_arr_imm;
        new_cpu.dispatch[0x6c as usize] = Cpu::op_jmp_ind;
        new_cpu.dispatch[0x6d as usize] = Cpu::op_adc_abs;
        new_cpu.dispatch[0x6e as usize] = Cpu::op_ror_abs;
        new_cpu.dispatch[0x6f as usize] = Cpu::op_rra_abs;

        new_cpu.dispatch[0x70 as usize] = Cpu::op_bvs_rel;
        new_cpu.dispatch[0x71 as usize] = Cpu::op_adc_izy;
        new_cpu.dispatch[0x72 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0x73 as usize] = Cpu::op_rra_izy;
        new_cpu.dispatch[0x74 as usize] = Cpu::op_nop_zpx;
        new_cpu.dispatch[0x75 as usize] = Cpu::op_adc_zpx;
        new_cpu.dispatch[0x76 as usize] = Cpu::op_ror_zpx;
        new_cpu.dispatch[0x77 as usize] = Cpu::op_rra_zpx;
        new_cpu.dispatch[0x78 as usize] = Cpu::op_sei;
        new_cpu.dispatch[0x79 as usize] = Cpu::op_adc_aby;
        new_cpu.dispatch[0x7a as usize] = Cpu::op_nop;
        new_cpu.dispatch[0x7b as usize] = Cpu::op_rda_aby;
        new_cpu.dispatch[0x7c as usize] = Cpu::op_nop_abx;
        new_cpu.dispatch[0x7d as usize] = Cpu::op_adc_abx;
        new_cpu.dispatch[0x7e as usize] = Cpu::op_ror_abx;
        new_cpu.dispatch[0x7f as usize] = Cpu::op_rra_abx;

        new_cpu.dispatch[0x80 as usize] = Cpu::op_nop_imm;
        new_cpu.dispatch[0x81 as usize] = Cpu::op_sta_izx;
        new_cpu.dispatch[0x82 as usize] = Cpu::op_nop_imm;
        new_cpu.dispatch[0x83 as usize] = Cpu::op_sax_izx;
        new_cpu.dispatch[0x84 as usize] = Cpu::op_sty_zp;
        new_cpu.dispatch[0x85 as usize] = Cpu::op_sta_zp;
        new_cpu.dispatch[0x86 as usize] = Cpu::op_stx_zp;
        new_cpu.dispatch[0x87 as usize] = Cpu::op_sax_zp;
        new_cpu.dispatch[0x88 as usize] = Cpu::op_dey;
        new_cpu.dispatch[0x89 as usize] = Cpu::op_nop_imm;
        new_cpu.dispatch[0x8a as usize] = Cpu::op_txa;
        new_cpu.dispatch[0x8b as usize] = Cpu::op_xaa_imm;
        new_cpu.dispatch[0x8c as usize] = Cpu::op_sty_abs;
        new_cpu.dispatch[0x8d as usize] = Cpu::op_sta_abs;
        new_cpu.dispatch[0x8e as usize] = Cpu::op_stx_abs;
        new_cpu.dispatch[0x8f as usize] = Cpu::op_sax_abs;

        new_cpu.dispatch[0x90 as usize] = Cpu::op_bcc_rel;
        new_cpu.dispatch[0x91 as usize] = Cpu::op_sta_izy;
        new_cpu.dispatch[0x92 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0x93 as usize] = Cpu::op_ahx_izy;
        new_cpu.dispatch[0x94 as usize] = Cpu::op_sty_zpx;
        new_cpu.dispatch[0x95 as usize] = Cpu::op_sta_zpx;
        new_cpu.dispatch[0x96 as usize] = Cpu::op_stx_zpy;
        new_cpu.dispatch[0x97 as usize] = Cpu::op_sax_zpy;
        new_cpu.dispatch[0x98 as usize] = Cpu::op_tya;
        new_cpu.dispatch[0x99 as usize] = Cpu::op_sta_aby;
        new_cpu.dispatch[0x9a as usize] = Cpu::op_txs;
        new_cpu.dispatch[0x9b as usize] = Cpu::op_tas_aby;
        new_cpu.dispatch[0x9c as usize] = Cpu::op_shy_abx;
        new_cpu.dispatch[0x9d as usize] = Cpu::op_sta_abx;
        new_cpu.dispatch[0x9e as usize] = Cpu::op_shx_aby;
        new_cpu.dispatch[0x9f as usize] = Cpu::op_ahx_aby;

        new_cpu.dispatch[0xa0 as usize] = Cpu::op_ldy_imm;
        new_cpu.dispatch[0xa1 as usize] = Cpu::op_lda_izx;
        new_cpu.dispatch[0xa2 as usize] = Cpu::op_ldx_imm;
        new_cpu.dispatch[0xa3 as usize] = Cpu::op_lax_izx;
        new_cpu.dispatch[0xa4 as usize] = Cpu::op_ldy_zp;
        new_cpu.dispatch[0xa5 as usize] = Cpu::op_lda_zp;
        new_cpu.dispatch[0xa6 as usize] = Cpu::op_ldx_zp;
        new_cpu.dispatch[0xa7 as usize] = Cpu::op_lax_zp;
        new_cpu.dispatch[0xa8 as usize] = Cpu::op_tay;
        new_cpu.dispatch[0xa9 as usize] = Cpu::op_lda_imm;
        new_cpu.dispatch[0xaa as usize] = Cpu::op_tax;
        new_cpu.dispatch[0xab as usize] = Cpu::op_lax_imm;
        new_cpu.dispatch[0xac as usize] = Cpu::op_ldy_abs;
        new_cpu.dispatch[0xad as usize] = Cpu::op_lda_abs;
        new_cpu.dispatch[0xae as usize] = Cpu::op_ldx_abs;
        new_cpu.dispatch[0xaf as usize] = Cpu::op_lax_abs;

        new_cpu.dispatch[0xb0 as usize] = Cpu::op_bcs_rel;
        new_cpu.dispatch[0xb1 as usize] = Cpu::op_lda_izy;
        new_cpu.dispatch[0xb2 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0xb3 as usize] = Cpu::op_lax_izy;
        new_cpu.dispatch[0xb4 as usize] = Cpu::op_ldy_zpx;
        new_cpu.dispatch[0xb5 as usize] = Cpu::op_lda_zpx;
        new_cpu.dispatch[0xb6 as usize] = Cpu::op_ldx_zpy;
        new_cpu.dispatch[0xb7 as usize] = Cpu::op_lax_zpy;
        new_cpu.dispatch[0xb8 as usize] = Cpu::op_clv;
        new_cpu.dispatch[0xb9 as usize] = Cpu::op_lda_aby;
        new_cpu.dispatch[0xba as usize] = Cpu::op_tsx;
        new_cpu.dispatch[0xbb as usize] = Cpu::op_las_aby;
        new_cpu.dispatch[0xbc as usize] = Cpu::op_ldy_abx;
        new_cpu.dispatch[0xbd as usize] = Cpu::op_lda_abx;
        new_cpu.dispatch[0xbe as usize] = Cpu::op_ldx_aby;
        new_cpu.dispatch[0xbf as usize] = Cpu::op_lax_aby;

        new_cpu.dispatch[0xc0 as usize] = Cpu::op_cpy_imm;
        new_cpu.dispatch[0xc1 as usize] = Cpu::op_cmp_izx;
        new_cpu.dispatch[0xc2 as usize] = Cpu::op_nop_imm;
        new_cpu.dispatch[0xc3 as usize] = Cpu::op_dcp_izx;
        new_cpu.dispatch[0xc4 as usize] = Cpu::op_cpy_zp;
        new_cpu.dispatch[0xc5 as usize] = Cpu::op_cmp_zp;
        new_cpu.dispatch[0xc6 as usize] = Cpu::op_dec_zp;
        new_cpu.dispatch[0xc7 as usize] = Cpu::op_dcp_zp;
        new_cpu.dispatch[0xc8 as usize] = Cpu::op_iny;
        new_cpu.dispatch[0xc9 as usize] = Cpu::op_cmp_imm;
        new_cpu.dispatch[0xca as usize] = Cpu::op_dex;
        new_cpu.dispatch[0xcb as usize] = Cpu::op_axs_imm;
        new_cpu.dispatch[0xcc as usize] = Cpu::op_cpy_abs;
        new_cpu.dispatch[0xcd as usize] = Cpu::op_cmp_abs;
        new_cpu.dispatch[0xce as usize] = Cpu::op_dec_abs;
        new_cpu.dispatch[0xcf as usize] = Cpu::op_dcp_abs;

        new_cpu.dispatch[0xd0 as usize] = Cpu::op_bne_rel;
        new_cpu.dispatch[0xd1 as usize] = Cpu::op_cmp_izy;
        new_cpu.dispatch[0xd2 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0xd3 as usize] = Cpu::op_dcp_izy;
        new_cpu.dispatch[0xd4 as usize] = Cpu::op_nop_zpx;
        new_cpu.dispatch[0xd5 as usize] = Cpu::op_cmp_zpx;
        new_cpu.dispatch[0xd6 as usize] = Cpu::op_dec_zpx;
        new_cpu.dispatch[0xd7 as usize] = Cpu::op_dcp_zpx;
        new_cpu.dispatch[0xd8 as usize] = Cpu::op_cld;
        new_cpu.dispatch[0xd9 as usize] = Cpu::op_cmp_aby;
        new_cpu.dispatch[0xda as usize] = Cpu::op_nop;
        new_cpu.dispatch[0xdb as usize] = Cpu::op_dcp_aby;
        new_cpu.dispatch[0xdc as usize] = Cpu::op_nop_abx;
        new_cpu.dispatch[0xdd as usize] = Cpu::op_cmp_abx;
        new_cpu.dispatch[0xde as usize] = Cpu::op_dec_abx;
        new_cpu.dispatch[0xdf as usize] = Cpu::op_dcp_abx;

        new_cpu.dispatch[0xe0 as usize] = Cpu::op_cpx_imm;
        new_cpu.dispatch[0xe1 as usize] = Cpu::op_sbc_izx;
        new_cpu.dispatch[0xe2 as usize] = Cpu::op_nop_imm;
        new_cpu.dispatch[0xe3 as usize] = Cpu::op_isc_izx;
        new_cpu.dispatch[0xe4 as usize] = Cpu::op_cpx_zp;
        new_cpu.dispatch[0xe5 as usize] = Cpu::op_sbc_zp;
        new_cpu.dispatch[0xe6 as usize] = Cpu::op_inc;
        new_cpu.dispatch[0xe7 as usize] = Cpu::op_isc_zp;
        new_cpu.dispatch[0xe8 as usize] = Cpu::op_inx;
        new_cpu.dispatch[0xe9 as usize] = Cpu::op_sbc_imm;
        new_cpu.dispatch[0xea as usize] = Cpu::op_nop;
        new_cpu.dispatch[0xeb as usize] = Cpu::op_sbc_imm;
        new_cpu.dispatch[0xec as usize] = Cpu::op_cpx_abs;
        new_cpu.dispatch[0xed as usize] = Cpu::op_sbc_abs;
        new_cpu.dispatch[0xee as usize] = Cpu::op_inc_abs;
        new_cpu.dispatch[0xef as usize] = Cpu::op_isc_abs;

        new_cpu.dispatch[0xf0 as usize] = Cpu::op_beq_rel;
        new_cpu.dispatch[0xf1 as usize] = Cpu::op_sbc_izy;
        new_cpu.dispatch[0xf2 as usize] = Cpu::op_hlt;
        new_cpu.dispatch[0xf3 as usize] = Cpu::op_isc_izy;
        new_cpu.dispatch[0xf4 as usize] = Cpu::op_nop_zpx;
        new_cpu.dispatch[0xf5 as usize] = Cpu::op_sbc_zpx;
        new_cpu.dispatch[0xf6 as usize] = Cpu::op_inc_zpx;
        new_cpu.dispatch[0xf7 as usize] = Cpu::op_isc_zpx;
        new_cpu.dispatch[0xf8 as usize] = Cpu::op_sed;
        new_cpu.dispatch[0xf9 as usize] = Cpu::op_sbc_aby;
        new_cpu.dispatch[0xfa as usize] = Cpu::op_nop;
        new_cpu.dispatch[0xfb as usize] = Cpu::op_isc_aby;
        new_cpu.dispatch[0xfc as usize] = Cpu::op_nop_abx;
        new_cpu.dispatch[0xfd as usize] = Cpu::op_sbc_abx;
        new_cpu.dispatch[0xfe as usize] = Cpu::op_inc_abx;
        new_cpu.dispatch[0xff as usize] = Cpu::op_isc_abx;

        new_cpu
    }

    pub fn reset(&mut self, mem : &mut Mem) {
        // 6502 starts with pc pointed at value found in memory at 0xfffc
        // self.pc = mem.get_word(0xfffc_u16);


        // Test code.
        // From https://github.com/Klaus2m5/6502_65C02_functional_tests/blob/master/6502_functional_test.a65
        self.pc = 0x0400_u16
        // self.pc = 0x0594_u16;
    }

    pub fn tick(&mut self, mem : &mut Mem) {
        let pc = self.pc;
        println!("pc:{:04x} a:{:02x} x:{:02x} y:{:02x} s:{:02x} n:{} v:{} d:{} z:{} c:{}",
                self.pc, self.a, self.x, self.y, self.s, self.n as i8, self.v as i8,
                self.d as i8, self.z as i8, self.c as i8);

        self.debugger.disassemble(mem.get_byte(self.pc),
                mem.get_byte(self.pc + 1),
                mem.get_byte(self.pc + 2));

        let opcode = self.fetch_byte(mem);

        // println!("opcode {:02x} at {:04x}", opcode, pc);

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

    fn fetch_addr_mode_zpy(&mut self, mem : &mut Mem) -> u16 {
        let offset = self.fetch_byte(mem);
        self.y.wrapping_add(offset) as u16
    }

    fn fetch_val_mode_zpy(&mut self, mem : &mut Mem) -> u8 {
        let addr = self.fetch_addr_mode_zpy(mem);
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
        // Branch instructions are relative to PC of the next instruction.
        let offset = self.fetch_byte(mem) as i8 as i16;
        let pc = self.pc as i16;
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
        let status = self.get_status();
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
        let val = self.stack_pop_byte(mem);
        self.set_status(val);
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
        let addr = self.fetch_addr_mode_rel(mem);
        self.bmi(mem, addr);

        // // TODO : Review
        // if self.n {
        //     let offset = mem.get_byte(self.pc) as i8 as i16;
        //     self.pc = (self.pc as i16 + offset) as u16;
        // } else {
        //     self.pc += 1;
        // }
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

    // 0x4b, time 2
    fn op_alr_imm(&mut self, mem : &mut Mem) {
        panic!("op_alr_imm is not implemented");
    }

    // 0x4c, time 3
    fn op_jmp_abs(&mut self, mem : &mut Mem) {
        // panic!("op_jmp_abs is not implemented");

        let addr = self.fetch_addr_mode_abs(mem);
        self.jmp(mem, addr);
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

    // 0x53, time 8, unofficial
    fn op_sre_izy(&mut self, mem : &mut Mem) {
        panic!("op_sre_izy is not implemented");
    }

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
    fn op_nop_imm(&mut self, mem : &mut Mem) {
        self.pc += 1;
    }

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
        self.y = self.y.wrapping_add(0xff);
        self.compute_nz_val(self.y);
    }

    // 0x89 nop_imm

    // 0x8a, time 2
    fn op_txa(&mut self, mem : &mut Mem) {
        self.a = self.x;
        self.compute_nz_val(self.a);
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

    // 0x92 hlt

    // 0x93, time 6
    fn op_ahx_izy(&mut self, mem : &mut Mem) {
        panic!("op_ahx_izy is not implemented");
    }

    // 0x94, time 4
    fn op_sty_zpx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zpx(mem);
        self.sty(mem, addr);
    }

    // 0x95, time 4
    fn op_sta_zpx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zpx(mem);
        self.sta(mem, addr);
    }

    // 0x96, time 4
    fn op_stx_zpy(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zpy(mem);
        self.stx(mem, addr);
    }

    // 0x97, time 4, unofficial
    fn op_sax_zpy(&mut self, mem : &mut Mem) {
        panic!("op_ahx_izy is not implemented");
    }

    // 0x98, time 2
    fn op_tya(&mut self, mem : &mut Mem) {
        self.a = self.y;
        self.compute_nz_val(self.a);
    }

    // 0x99, time 5
    fn op_sta_aby(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_aby(mem);
        self.sta(mem, addr);
    }

    // 0x9a, time 2
    fn op_txs(&mut self, mem : &mut Mem) {
        self.s = self.x;
        self.compute_nz_val(self.s);
    }

    // 0x9b, time 5
    fn op_tas_aby(&mut self, mem : &mut Mem) {
        panic!("op_tas_aby is not implemented");
    }

    // 0x9c, time 5
    fn op_shy_abx(&mut self, mem : &mut Mem) {
        panic!("op_shy_abx is not implemented");
    }

    // 0x9d, time 5
    fn op_sta_abx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abx(mem);
        self.sta(mem, addr);
    }

    // 0x9e, time 5
    fn op_shx_aby(&mut self, mem : &mut Mem) {
        panic!("op_shx_aby is not implemented");
    }

    // 0x9f, time 5
    fn op_ahx_aby(&mut self, mem : &mut Mem) {
        panic!("op_ahx_aby is not implemented");
    }

    // 0xa0, time 2
    fn op_ldy_imm(&mut self, mem : &mut Mem) {
        let val = self.fetch_byte(mem);
        self.ldy(mem, val);
    }

    // 0xa1, time 6
    fn op_lda_izx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izx(mem);
        self.lda(mem, val);
    }

    // 0xa2, time 2
    fn op_ldx_imm(&mut self, mem : &mut Mem) {
        let val = self.fetch_byte(mem);
        self.ldx(mem, val);
    }

    // 0xa3, time 6, unofficial
    fn op_lax_izx(&mut self, mem : &mut Mem) {
        panic!("op_lax_izx is not implemented");
    }

    // 0xa4, time 3
    fn op_ldy_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.ldy(mem, val);
    }

    // 0xa5, time 3
    fn op_lda_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.lda(mem, val);
        println!("lda_zp {:02x}", self.a);
    }

    // 0xa6, time 3
    fn op_ldx_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.ldx(mem, val);
    }

    // 0xa7, time 3
    fn op_lax_zp(&mut self, mem : &mut Mem) {
        panic!("op_lax_zp is not implemented");
    }

    // 0xa8, time 2
    fn op_tay(&mut self, mem : &mut Mem) {
        self.y = self.a;
        self.compute_nz_val(self.y);
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
        self.compute_nz_val(self.x);
        println!("tax {:02x}", self.x);
    }

    // 0xab, time 2, unofficial
    fn op_lax_imm(&mut self, mem : &mut Mem) {
        panic!("op_lax_imm is not implemented");
    }

    // 0xac, time 4
    fn op_ldy_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.ldy(mem, val);
    }

    // 0xad, time 4
    fn op_lda_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.lda(mem, val);
    }

    // 0xae, time 4
    fn op_ldx_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.ldx(mem, val);
    }

    // 0xaf, time 4
    fn op_lax_abs(&mut self, mem : &mut Mem) {
        panic!("op_lax_imm is not implemented");
    }

    // 0xb0, time 2
    fn op_bcs_rel(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_rel(mem);
        self.bcs(mem, addr);
    }

    // 0xb1, time 5
    fn op_lda_izy(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izy(mem);
        self.lda(mem, val);
    }

    // 0xb2 hlt

    // 0xb3, time 5+, unofficial
    fn op_lax_izy(&mut self, mem : &mut Mem) {
        panic!("op_lax_imm is not implemented");
    }

    // 0xb4, time 4
    fn op_ldy_zpx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zpx(mem);
        self.ldy(mem, val);
    }

    // 0xb5, time 4
    fn op_lda_zpx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zpx(mem);
        self.lda(mem, val);
    }

    // 0xb6, time 4
    fn op_ldx_zpy(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zpy(mem);
        self.ldx(mem, val);
    }

    // 0xb7, time 4
    fn op_lax_zpy(&mut self, mem : &mut Mem) {
        panic!("op_lax_imm is not implemented");
    }

    // 0xb8, time 2
    fn op_clv(&mut self, mem : &mut Mem) {
        self.v = false;
    }

    // 0xb9, time 4+
    fn op_lda_aby(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_aby(mem);
        self.lda(mem, val);
    }

    // 0xba, time 2
    fn op_tsx(&mut self, mem : &mut Mem) {
        self.x = self.s;
        self.compute_nz_val(self.x);
    }

    // 0xbb, time 4+, unofficial
    fn op_las_aby(&mut self, mem : &mut Mem) {
        panic!("op_las_aby is not implemented");
    }

    // 0xbc, time 4+
    fn op_ldy_abx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abx(mem);
        self.ldy(mem, val);
    }

    // 0xbd, time 4+
    fn op_lda_abx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abx(mem);
        self.lda(mem, val);
    }

    // 0xbe, time 4+
    fn op_ldx_aby(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_aby(mem);
        self.ldx(mem, val);
    }

    // 0xbf, time 4+, unofficial
    fn op_lax_aby(&mut self, mem : &mut Mem) {
        panic!("op_lax_aby is not implemented");
    }

    // 0xc0, time 2
    fn op_cpy_imm(&mut self, mem : &mut Mem) {
        let val = self.fetch_byte(mem);
        self.cmp(mem, self.y, val);
    }

    // 0xc1, time 6
    fn op_cmp_izx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izx(mem);
        self.cmp(mem, self.a, val);
    }

    // 0xc2 nop_imm

    // 0xc3, time 8, unofficial
    fn op_dcp_izx(&mut self, mem : &mut Mem) {
        panic!("op_lax_aby is not implemented");
    }

    // 0xc4, time 3
    fn op_cpy_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.cmp(mem, self.y, val);
    }

    // 0xc5, time 3
    fn op_cmp_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.cmp(mem, self.a, val);
    }

    // 0xc6, time 5
    fn op_dec_zp(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zp(mem);
        self.dec(mem, addr);
    }

    // 0xc7, time 5, unofficial
    fn op_dcp_zp(&mut self, mem : &mut Mem) {
        panic!("op_dcp_zp is not implemented");
    }

    // 0xc8, time 2
    fn op_iny(&mut self, mem : &mut Mem) {
        self.y = self.y.wrapping_add(1);
        self.compute_nz_val(self.y);
    }

    // 0xc9, time 2
    fn op_cmp_imm(&mut self, mem : &mut Mem) {
        let val = self.fetch_byte(mem);
        self.cmp(mem, self.a, val);
    }

    // 0xca, time 2
    fn op_dex(&mut self, mem : &mut Mem) {
        self.x = self.x.wrapping_add(0xff);
        self.compute_nz_val(self.x);
    }

    // 0xcb, time 2, unofficial
    fn op_axs_imm(&mut self, mem : &mut Mem) {
        panic!("op_dcp_zp is not implemented");
    }

    // 0xcc, time 4
    fn op_cpy_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.cmp(mem, self.y, val);
    }

    // 0xcd, time 4
    fn op_cmp_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.cmp(mem, self.a, val);
    }

    // 0xce, time 6
    fn op_dec_abs(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abs(mem);
        self.dec(mem, addr);
    }

    // 0xcf, time 6
    fn op_dcp_abs(&mut self, mem : &mut Mem) {
        panic!("op_dcp_abs is not implemented");
    }

    // 0xd0, time 2+
    fn op_bne_rel(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_rel(mem);
        self.bne(mem, addr);
    }

    // 0xd1, time 5+
    fn op_cmp_izy(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izy(mem);
        self.cmp(mem, self.a, val);
    }

    // 0xd2 hlt

    // 0xd3, time 8
    fn op_dcp_izy(&mut self, mem : &mut Mem) {
        panic!("op_dcp_izy is not implemented");
    }

    // 0xd4 nop_zpx

    // 0xd5, time 4
    fn op_cmp_zpx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zpx(mem);
        self.cmp(mem, self.a, val);
    }

    // 0xd6, time 6
    fn op_dec_zpx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zpx(mem);
        self.dec(mem, addr);
    }

    // 0xd7, time 6
    fn op_dcp_zpx(&mut self, mem : &mut Mem) {
        panic!("op_dcp_izy is not implemented");
    }

    // 0xd8, time 2
    fn op_cld(&mut self, mem : &mut Mem) {
        self.d = false;
    }

    // 0xd9, time 4+
    fn op_cmp_aby(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_aby(mem);
        self.cmp(mem, self.a, val);
    }

    // 0xda nop

    // 0xdb, time 7
    fn op_dcp_aby(&mut self, mem : &mut Mem) {
        panic!("op_dcp_aby is not implemented");
    }

    // 0xdc nop_abx

    // 0xdd, time 4
    fn op_cmp_abx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abx(mem);
        self.cmp(mem, self.a, val);
    }

    // 0xde, time 7
    fn op_dec_abx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abx(mem);
        self.dec(mem, addr);
    }

    // 0xdf, time 7
    fn op_dcp_abx(&mut self, mem : &mut Mem) {
        panic!("op_dcp_abx is not implemented");
    }

    // 0xe0, time 2
    fn op_cpx_imm(&mut self, mem : &mut Mem) {
        let val = self.fetch_byte(mem);
        self.cmp(mem, self.x, val);
    }

    // 0xe1, time 6
    fn op_sbc_izx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izx(mem);
        self.sbc(mem, val);
    }

    // 0xe2 nop_imm

    // 0xe3, time 8
    fn op_isc_izx(&mut self, mem : &mut Mem) {
        panic!("op_isc_izx is not implemented");
    }

    // 0xe4, time 3
    fn op_cpx_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.cmp(mem, self.x, val);
    }

    // 0xe5, time 3
    fn op_sbc_zp(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zp(mem);
        self.sbc(mem, val);
    }

    // 0xe6, time 5
    fn op_inc(&mut self, mem : &mut Mem) {
        self.a = self.a.wrapping_add(1);
        self.compute_nz_val(self.a);
    }

    // 0xe7, time 5
    fn op_isc_zp(&mut self, mem : &mut Mem) {
        panic!("op_isc_zp is not implemented");
    }

    // 0xe8, time 2
    fn op_inx(&mut self, mem : &mut Mem) {
        self.x = self.x.wrapping_add(1);
        self.compute_nz_val(self.x);
    }

    // 0xe9, time 2
    fn op_sbc_imm(&mut self, mem : &mut Mem) {
        let val = self.fetch_byte(mem);
        self.sbc(mem, val);
    }

    // 0xea nop, official

    // 0xeb sbc_imm, unofficial

    // 0xec, time 4
    fn op_cpx_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.cmp(mem, self.x, val);
    }

    // 0xed, time 4
    fn op_sbc_abs(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abs(mem);
        self.sbc(mem, val);
    }

    // 0xee, time 6
    fn op_inc_abs(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abs(mem);
        self.inc(mem, addr);
    }

    // 0xef, time 6
    fn op_isc_abs(&mut self, mem : &mut Mem) {
        panic!("op_isc_abs is not implemented");
    }

    // 0xf0, time 2+
    fn op_beq_rel(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_rel(mem);
        self.beq(mem, addr);
    }

    // 0xf1, time 5+
    fn op_sbc_izy(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_izy(mem);
        self.sbc(mem, val);
    }

    // 0xf2 hlt

    // 0xf3, time 8
    fn op_isc_izy(&mut self, mem : &mut Mem) {
        panic!("op_isc_izy is not implemented");
    }

    // 0xf4 nop_zpx

    // 0xf5, time 4
    fn op_sbc_zpx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_zpx(mem);
        self.sbc(mem, val);
    }

    // 0xf6, time 6
    fn op_inc_zpx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_zpx(mem);
        self.inc(mem, addr);
    }

    // 0xf7, time 6
    fn op_isc_zpx(&mut self, mem : &mut Mem) {
        panic!("op_isc_izy is not implemented");
    }

    // 0xf8, time 2
    fn op_sed(&mut self, mem : &mut Mem) {
        self.d = true;
    }

    // 0xf9, time 4+
    fn op_sbc_aby(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_aby(mem);
        self.sbc(mem, val);
    }

    // 0xfa nop

    // 0xfb, time 7
    fn op_isc_aby(&mut self, mem : &mut Mem) {
        panic!("op_isc_aby is not implemented");
    }

    // 0xfc nop_abx

    // 0xfd, time 4+
    fn op_sbc_abx(&mut self, mem : &mut Mem) {
        let val = self.fetch_val_mode_abx(mem);
        self.sbc(mem, val);
    }

    // 0xfe, time 7
    fn op_inc_abx(&mut self, mem : &mut Mem) {
        let addr = self.fetch_addr_mode_abx(mem);
        self.inc(mem, addr);
    }

    // 0xff, time 7
    fn op_isc_abx(&mut self, mem : &mut Mem) {
        panic!("op_isc_abx is not implemented");
    }

    // Implementations of core functionality once the address has been
    // computed
    fn adc(&mut self, mem : &mut Mem, val : u8) {
        // TODO : Verify this
        // TODO : Deal with BCD mode

        // Add numbers twice: once in signed, the other unsigned. This gets us
        // the v and c flags.
        let (mut u_sum, mut u_overflow) = self.a.overflowing_add(val);
        let (mut s_sum, mut s_overflow) = (self.a as i8).overflowing_add(val as i8);

        if self.c {
            let(u_sumc, u_overflowc) = u_sum.overflowing_add(1);
            let(s_sumc, s_overflowc) = s_sum.overflowing_add(1);

            u_sum = u_sumc;
            u_overflow = u_overflow || u_overflowc;
            s_overflow = s_overflow || s_overflowc;
        }

        self.c = u_overflow;
        self.v = s_overflow;
        self.a = u_sum;

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

    fn beq(&mut self, mem : &mut Mem, addr: u16) {
        // TODO : verify this is what beq means
        if self.z {
            self.pc = addr;
        }
    }

    fn bmi(&mut self, mem : &mut Mem, addr: u16) {
        if self.n {
            self.pc = addr;
        }
    }

    fn bne(&mut self, mem : &mut Mem, addr: u16) {
        // TODO : verify this is what bne means
        if !self.z {
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

    fn cmp(&mut self, mem : &mut Mem, val1: u8, val2: u8) {
        let (delta, overflow) = val1.overflowing_sub(val2);
        // This is unintuitive, but CMP is like SBC with an implied carry bit already set.
        self.c = !overflow;
        self.compute_nz_val(delta)
    }

    fn dec(&mut self, mem : &mut Mem, addr: u16) {
        let val = mem.get_byte(addr);
        let new_val = val.wrapping_add(0xff);
        mem.set_byte(addr, val);
        self.compute_nz_val(new_val);
    }

    fn eor(&mut self, mem : &mut Mem, val: u8) {
        self.a = self.a ^ val;
        self.compute_nz();
    }

    fn inc(&mut self, mem : &mut Mem, addr: u16) {
        let val = mem.get_byte(addr);
        let new_val = val.wrapping_add(1);
        mem.set_byte(addr, val);
        self.compute_nz_val(new_val);
    }

    fn jmp(&mut self, mem : &mut Mem, addr: u16) {
        self.pc = addr;
    }

    fn lda(&mut self, mem : &mut Mem, val: u8) {
        self.a = val;
        self.compute_nz();
    }

    fn ldx(&mut self, mem : &mut Mem, val: u8) {
        self.x = val;
        self.compute_nz_val(self.x);
    }

    fn ldy(&mut self, mem : &mut Mem, val: u8) {
        self.y = val;
        self.compute_nz_val(self.y);
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

    fn sbc(&mut self, mem : &mut Mem, val : u8) {
        // Note : Based on adc, keep in sync.
        // TODO : Verify this
        // TODO : Deal with BCD mode


        // Add numbers twice: once in signed, the other unsigned. This gets us
        // the v and c flags.
        let (mut u_sum, mut u_overflow) = self.a.overflowing_sub(val);
        let (mut s_sum, mut s_overflow) = (self.a as i8).overflowing_sub(val as i8);

        if self.c {
            let(u_sumc, u_overflowc) = u_sum.overflowing_sub(1); // TODO ? add or sub here?
            let(s_sumc, s_overflowc) = s_sum.overflowing_sub(1);

            u_sum = u_sumc;
            u_overflow = u_overflow || u_overflowc;
            s_overflow = s_overflow || s_overflowc;
        }

        self.c = u_overflow;
        self.v = s_overflow;
        self.a = u_sum;

        self.compute_nz();
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

    fn get_status(&self) -> u8 {
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

    fn set_status(&mut self, status: u8) {
        let mut st = 0x20;

        self.n = status & 0x80 == 0x80;
        self.v = status & 0x40 == 0x40;
        self.d = status & 0x08 == 0x08;
        self.z = status & 0x04 == 0x04;
        self.c = status & 0x01 == 0x01;
    }
}

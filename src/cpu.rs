use crate::bus::Bus;

const STACK_BASE: u16 = 0x0100_u16;
const VECTOR_NMI: u16 = 0xfffa_u16;
const VECTOR_RESET: u16 = 0xfffc_u16;
const VECTOR_IRQBRK: u16 = 0xfffe_u16;
const STATUS_NEG : u8 = 0x80_u8;
const STATUS_OVR : u8 = 0x40_u8;
const STATUS_RES : u8 = 0x20_u8;
const STATUS_BRK : u8 = 0x10_u8;
const STATUS_DCM : u8 = 0x08_u8;
const STATUS_INT : u8 = 0x04_u8;
const STATUS_ZER : u8 = 0x02_u8;
const STATUS_CAR : u8 = 0x01_u8;

pub struct Cpu {
    // Cpu registers and flags
    pub pc : u16,
    pub a : u8,
    pub x : u8,
    pub y : u8,
    pub s : u8,
    pub n : bool,
    pub v : bool,
    pub d : bool,
    pub z : bool,
    pub c : bool,
    pub b : bool,
    pub i : bool,

    // Instruction dispatch table
    dispatch : [fn(&mut Cpu, &mut dyn Bus); 256],

    // Cycle timing infrastructure
    cycle_table: [u8; 256],        // Base cycle counts for each opcode
    pub cycles_remaining: u8,       // Cycles left in current instruction
    current_opcode: u8,             // Currently executing opcode
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut new_cpu = Cpu {
            pc : 0x0000,
            a : 0x00,
            x : 0x00,
            y : 0x00,
            s : 0xff,
            n : false,
            v : false,
            d : false,
            z : false,
            c : false,
            b : false,
            i : false,
            dispatch : [Cpu::unimpl; 256],
            cycle_table: [0; 256],  // Will be initialized below
            cycles_remaining: 0,
            current_opcode: 0,
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
        new_cpu.dispatch[0xe6 as usize] = Cpu::op_inc_zp;
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

        // Initialize cycle timing table
        // Timing extracted from instruction comments
        // 0 cycles indicates HLT or unimplemented opcodes
        new_cpu.cycle_table = [
            7, 6, 0, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6,  // 0x00-0x0F
            2, 5, 0, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,  // 0x10-0x1F
            6, 6, 0, 7, 3, 3, 5, 5, 4, 2, 2, 0, 4, 4, 6, 6,  // 0x20-0x2F
            2, 5, 0, 8, 0, 4, 6, 6, 2, 4, 0, 7, 0, 4, 7, 7,  // 0x30-0x3F
            6, 6, 0, 8, 0, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6,  // 0x40-0x4F
            2, 5, 0, 8, 0, 4, 6, 6, 2, 4, 0, 7, 0, 4, 7, 7,  // 0x50-0x5F
            6, 6, 0, 8, 0, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6,  // 0x60-0x6F
            2, 5, 0, 8, 0, 4, 6, 6, 2, 4, 0, 7, 0, 4, 7, 7,  // 0x70-0x7F
            0, 6, 0, 0, 3, 3, 3, 3, 2, 0, 2, 2, 4, 4, 4, 4,  // 0x80-0x8F
            2, 6, 0, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5,  // 0x90-0x9F
            2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4,  // 0xA0-0xAF
            2, 5, 0, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4,  // 0xB0-0xBF
            2, 6, 0, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6,  // 0xC0-0xCF
            2, 5, 0, 8, 0, 4, 6, 6, 2, 4, 0, 7, 0, 4, 7, 7,  // 0xD0-0xDF
            2, 6, 0, 8, 3, 3, 5, 5, 2, 2, 0, 0, 4, 4, 6, 6,  // 0xE0-0xEF
            2, 5, 0, 8, 0, 4, 6, 6, 2, 4, 0, 7, 0, 4, 7, 7,  // 0xF0-0xFF
        ];

        new_cpu
    }

    pub fn reset(&mut self, bus: &mut dyn Bus) {
        // 6502 starts with pc pointed at value found in memory at 0xfffc
        // self.pc = bus.read_word(0xfffc_u16);


        // Test code.
        // From https://github.com/Klaus2m5/6502_65C02_functional_tests/blob/master/6502_functional_test.a65
        self.pc = 0x0400_u16
        // self.pc = 0x0594_u16;
    }

    pub fn tick(&mut self, bus: &mut dyn Bus) -> u8 {
        if self.cycles_remaining == 0 {
            // Start new instruction
            self.current_opcode = self.fetch_byte(bus);

            // Get base cycle count from table
            self.cycles_remaining = self.cycle_table[self.current_opcode as usize];

            // Execute the instruction (may add extra cycles for page crossings)
            self.dispatch[self.current_opcode as usize](self, bus);

            // Consume one cycle for instruction fetch/execution start
            if self.cycles_remaining > 0 {
                self.cycles_remaining -= 1;
            }
            return 1;
        } else {
            // Continue multi-cycle instruction
            self.cycles_remaining -= 1;
            return 1;
        }
    }

    pub fn state_string(&self) -> String {
        format!("pc:{:04x} a:{:02x} x:{:02x} y:{:02x} s:{:02x} / n:{} v:{} b:{} d:{} i:{} z:{} c:{}",
                self.pc, self.a, self.x, self.y, self.s, self.n as i8, self.v as i8, self.b as i8,
                self.d as i8, self.i as i8, self.z as i8, self.c as i8)
    }

    /// Check if adding offset to base address crosses a page boundary
    fn crosses_page_boundary(&self, base: u16, offset: u8) -> bool {
        let result = base.wrapping_add(offset as u16);
        (base & 0xFF00) != (result & 0xFF00)
    }

    /// Check if branch target crosses page boundary from current PC
    fn branch_crosses_page(&self, offset: i8) -> bool {
        let target = self.pc.wrapping_add(offset as i16 as u16);
        (self.pc & 0xFF00) != (target & 0xFF00)
    }

    pub fn unimpl(&mut self, bus: &mut dyn Bus) {
        panic!("Unimplemented instruction");
    }

    // Fetch from program counter
    fn fetch_byte(&mut self, bus: &mut dyn Bus) -> u8 {
        let addr = bus.read(self.pc);
        self.pc += 1;
        addr
    }

    fn fetch_word(&mut self, bus: &mut dyn Bus) -> u16 {
        let addr = bus.read_word(self.pc);
        self.pc += 2;
        addr
    }

    // Addressing modes. addr_X computes the address for an operation.
    fn fetch_addr_mode_abs(&mut self, bus: &mut dyn Bus) -> u16 {
        self.fetch_word(bus)
    }

    fn fetch_val_mode_abs(&mut self, bus: &mut dyn Bus) -> u8 {
        let addr = self.fetch_addr_mode_abs(bus);
        bus.read(addr)
    }

    fn fetch_addr_mode_abx(&mut self, bus: &mut dyn Bus) -> u16 {
        self.fetch_word(bus) + self.x as u16
    }

    fn fetch_val_mode_abx(&mut self, bus: &mut dyn Bus) -> u8 {
        let addr = self.fetch_addr_mode_abx(bus);
        bus.read(addr)
    }

    fn fetch_addr_mode_aby(&mut self, bus: &mut dyn Bus) -> u16 {
        self.fetch_word(bus) + self.y as u16
    }

    fn fetch_val_mode_aby(&mut self, bus: &mut dyn Bus) -> u8 {
        let addr = self.fetch_addr_mode_aby(bus);
        bus.read(addr)
    }

    fn fetch_addr_mode_zp(&mut self, bus: &mut dyn Bus) -> u16 {
        self.fetch_byte(bus) as u16
    }

    fn fetch_val_mode_zp(&mut self, bus: &mut dyn Bus) -> u8 {
        let addr = self.fetch_addr_mode_zp(bus);
        bus.read(addr)
    }

    fn fetch_addr_mode_zpx(&mut self, bus: &mut dyn Bus) -> u16 {
        let offset = self.fetch_byte(bus);
        self.x.wrapping_add(offset) as u16
    }

    fn fetch_val_mode_zpx(&mut self, bus: &mut dyn Bus) -> u8 {
        let addr = self.fetch_addr_mode_zpx(bus);
        bus.read(addr)
    }

    fn fetch_addr_mode_zpy(&mut self, bus: &mut dyn Bus) -> u16 {
        let offset = self.fetch_byte(bus);
        self.y.wrapping_add(offset) as u16
    }

    fn fetch_val_mode_zpy(&mut self, bus: &mut dyn Bus) -> u8 {
        let addr = self.fetch_addr_mode_zpy(bus);
        bus.read(addr)
    }

    fn fetch_addr_mode_izx(&mut self, bus: &mut dyn Bus) -> u16 {
        let addr = self.fetch_byte(bus).wrapping_add(self.x) as u16;
        bus.read_word(addr)
    }

    fn fetch_val_mode_izx(&mut self, bus: &mut dyn Bus) -> u8 {
        let addr = self.fetch_addr_mode_izx(bus);
        bus.read(addr)
    }

    fn fetch_addr_mode_izy(&mut self, bus: &mut dyn Bus) -> u16 {
        let addr = self.fetch_byte(bus) as u16;
        bus.read_word(addr).wrapping_add(self.y as u16)
    }

    fn fetch_val_mode_izy(&mut self, bus: &mut dyn Bus) -> u8 {
        let addr = self.fetch_addr_mode_izy(bus);
        bus.read(addr)
    }

    fn fetch_addr_mode_rel(&mut self, bus: &mut dyn Bus) -> u16 {
        // Branch instructions are relative to PC of the next instruction.
        let offset = self.fetch_byte(bus) as i8 as i16;
        let pc = self.pc as i16;
        (pc + offset) as u16
    }

    fn fetch_addr_mode_ind(&mut self, bus: &mut dyn Bus) -> u16 {
        let addr = self.fetch_word(bus);
        bus.read_word(addr)
    }

    fn addr_stack(&mut self) -> u16 {
        STACK_BASE + self.s as u16
    }

    // 0x00, time 7
    fn op_brk(&mut self, bus: &mut dyn Bus) {
        let addr = bus.read_word(VECTOR_IRQBRK);
        let status = self.get_status(true);
        self.i = true;
        self.b = true;
        self.stack_push_word(bus, self.pc + 1);
        self.stack_push_byte(bus, status);
        self.pc = addr;
        // panic!("op_brk is not implemented");
    }

    // 0x01, time 6
    fn op_ora_izx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izx(bus);
        self.ora(bus, val);
    }

    // 0x02, unofficial
    fn op_hlt(&mut self, bus: &mut dyn Bus) {
        panic!("cpu halt");
    }

    // 0x03, time 8, unofficial
    fn op_slo_izx(&mut self, bus: &mut dyn Bus) {
        panic!("op_slo_izx is not implemented");
    }

    // 0x04, time 3, unofficial
    fn op_nop_zp(&mut self, bus: &mut dyn Bus) {
        self.pc += 1;
    }

    // 0x05, time 3
    fn op_ora_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.ora(bus, val);
    }

    // 0x06, time 5
    fn op_asl_zp(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zp(bus);
        self.asl_mem(bus, addr);
    }

    // 0x07, time 5, unofficial
    fn op_slo_zp(&mut self, bus: &mut dyn Bus) {
        panic!("op_slo_zp is not implemented");
    }

    // 0x08, time 3
    fn op_php(&mut self, bus: &mut dyn Bus) {
        let status = self.get_status(true);
        self.stack_push_byte(bus, status);
    }

    // 0x09, time 2
    fn op_ora_imm(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_byte(bus);
        self.ora(bus, val);
    }

    // 0x0a, time 2
    fn op_asl(&mut self, bus: &mut dyn Bus) {
        let val = self.a;
        self.a = self.asl_val(bus, val);
    }

    // 0x0b, time 2, unofficial
    fn op_anc_imm(&mut self, bus: &mut dyn Bus) {
        panic!("op_anc_imm is not implemented");
    }

    // 0x0c, time 4, unofficial
    fn op_nop_abs(&mut self, bus: &mut dyn Bus) {
        self.pc += 3;
    }

    // 0x0d, time 4
    fn op_ora_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.ora(bus, val);
    }

    // 0x0e, time 6
    fn op_asl_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.asl_mem(bus, addr);
    }

    // 0x0f, time 6, unofficial
    fn op_slo_abs(&mut self, bus: &mut dyn Bus) {
        panic!("op_nop_abs is not implemented");
    }

    // 0x10, time 2+
    fn op_bpl_rel(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_rel(bus);
        self.bpl(bus, addr);
    }

    // 0x11, time 5+
    fn op_ora_izy(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izy(bus);
        self.ora(bus, val);
    }

    // 0x12 is hlt

    // 0x13, time 8, unofficial
    fn op_slo_izy(&mut self, bus: &mut dyn Bus) {
        panic!("op_slo_izy is not implemented");
    }

    // 0x14, time 4, unofficial
    fn op_nop_zpx(&mut self, bus: &mut dyn Bus) {
        self.pc += 1;
    }

    // 0x15, time 4
    fn op_ora_zpx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zpx(bus);
        self.ora(bus, val);
    }

    // 0x16, time 6
    fn op_asl_zpx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zpx(bus);
        self.asl_mem(bus, addr);
    }

    // 0x17, time 6
    fn op_slo_zpx(&mut self, bus: &mut dyn Bus) {
        panic!("op_slo_zpx is not implemented");
    }

    // 0x18, time 2
    fn op_clc(&mut self, bus: &mut dyn Bus) {
        self.c = false;
    }

    // 0x19, time 4
    fn op_ora_aby(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_aby(bus);
        self.ora(bus, val);
    }

    // 0x1a, time 2, unofficial
    fn op_nop(&mut self, bus: &mut dyn Bus) {
    }

    // 0x1b, time 7, unofficial
    fn op_slo_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_slo_aby is not implemented");
    }

    // 0x1c, time 4+, unofficial
    fn op_nop_abx(&mut self, bus: &mut dyn Bus) {
        self.pc += 2;
    }

    // 0x1d, time 4
    fn op_ora_abx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abx(bus);
        self.ora(bus, val);
    }

    // 0x1e, time 7
    fn op_asl_abx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abx(bus);
        self.asl_mem(bus, addr);
    }

    // 0x1f, time 7, unofficial
    fn op_slo_abx(&mut self, bus: &mut dyn Bus) {
        panic!("op_slo_abx is not implemented");
    }

    // 0x20, time 6
    fn op_jsr_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.stack_push_word(bus, self.pc - 1);
        self.pc = addr;
    }

    // 0x21, time 6
    fn op_and_izx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izx(bus);
        self.and(bus, val);
    }

    // 0x22 hlt

    // 0x23, time 7, unofficial
    fn op_rla_izx(&mut self, bus: &mut dyn Bus) {
        panic!("op_rla_izx is not implemented");
    }

    // 0x24, time 3
    fn op_bit_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.bit(bus, val);
    }

    // 0x25, time 3
    fn op_and_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.and(bus, val);
    }

    // 0x26, time 5
    fn op_rol_zp(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zp(bus);
        self.rol_mem(bus, addr);
    }

    // 0x27, time 5, unofficial
    fn op_rla_zp(&mut self, bus: &mut dyn Bus) {
         panic!("op_rla_zp is not implemented");
    }

    // 0x28, time 4
    fn op_plp(&mut self, bus: &mut dyn Bus) {
        let val = self.stack_pop_byte(bus);
        self.set_status(val, false);
    }

    // 0x29, time 2
    fn op_and_imm(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_byte(bus);
        self.and(bus, val);
    }

    // 0x2a, time 2
    fn op_rol(&mut self, bus: &mut dyn Bus) {
        let val = self.a;
        let new_val = self.rol_val(bus, val);
        self.a = new_val;
    }

    // 0x2b op_anc_imm (see above)

    // 0x2c, time 4
    fn op_bit_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.bit(bus, val);
    }

    // 0x2d, time 4
    fn op_and_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.and(bus, val);
    }

    // 0x2e, time 6
    fn op_rol_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.rol_mem(bus, addr);
    }

    // 0x2f, time 6, unofficial
    fn op_rla_abs(&mut self, bus: &mut dyn Bus) {
         panic!("op_rla_zp is not implemented");
    }

    // 0x30, time 2+
    fn op_bmi_rel(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_rel(bus);
        self.bmi(bus, addr);
    }

    // 0x31, time 5+
    fn op_and_izy(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izy(bus);
        self.and(bus, val);
    }

    // 0x32 hlt

    // 0x33, time 8
    fn op_rla_izy(&mut self, bus: &mut dyn Bus) {
        panic!("op_rla_izy is not implemented");
    }

    // 0x34 nop_zpx

    // 0x35, time 4
    fn op_and_zpx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zpx(bus);
        self.and(bus, val);
    }

    // 0x36, time 6
    fn op_rol_zpx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zpx(bus);
        self.rol_mem(bus, addr);
    }

    // 0x37, time 6, unofficial
    fn op_rla_zpx(&mut self, bus: &mut dyn Bus) {
        panic!("op_rla_zpx is not implemented");
    }

    // 0x38, time 2
    fn op_sec(&mut self, bus: &mut dyn Bus) {
        self.c = true;
    }

    // 0x39, time 4
    fn op_and_aby(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_aby(bus);
        self.and(bus, val);
    }

    // 0x3a nop

    // 0x3b, time 7, unofficial
    fn op_rla_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_rla_aby is not implemented");
    }

    // 0x3c nop_abx

    // 0x3d, time 4+
    fn op_and_abx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abx(bus);
        self.and(bus, val);
    }

    // 0x3e, time 7
    fn op_rol_abx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abx(bus);
        self.rol_mem(bus, addr);
    }

    // 0x3f, time 7, unofficial
    fn op_rla_abx(&mut self, bus: &mut dyn Bus) {
        panic!("op_rla_aby is not implemented");
    }

    // 0x40, time 6
    fn op_rti(&mut self, bus: &mut dyn Bus) {
        let val = self.stack_pop_byte(bus);
        let addr = self.stack_pop_word(bus);
        self.set_status(val, false);
        self.pc = addr;
    }

    // 0x41, time 6
    fn op_eor_izx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izx(bus);
        self.eor(bus, val);
    }

    // 0x42 hlt

    // 0x43, time 8, unofficial
    fn op_sre_izx(&mut self, bus: &mut dyn Bus) {
        panic!("op_sre_izx is not implemented");
    }

    // 0x44 op_nop_zp

    // 0x45, time 3
    fn op_eor_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.eor(bus, val);
    }

    // 0x46, time 5
    fn op_lsr_zp(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zp(bus);
        self.lsr_mem(bus, addr);
    }

    // 0x47, time 5
    fn op_sre_zp(&mut self, bus: &mut dyn Bus) {
        panic!("op_sre_zp is not implemented");
    }

    // 0x48, time 3
    fn op_pha(&mut self, bus: &mut dyn Bus) {
        self.stack_push_byte(bus, self.a);
    }

    // 0x49, time 2
    fn op_eor_imm(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_byte(bus);
        self.eor(bus, val);
    }

    // 0x4a, time 2
    fn op_lsr(&mut self, bus: &mut dyn Bus) {
        let val = self.a;
        let new_val = self.lsr_val(bus, val);
        self.a = new_val;
    }

    // 0x4b, time 2
    fn op_alr_imm(&mut self, bus: &mut dyn Bus) {
        panic!("op_alr_imm is not implemented");
    }

    // 0x4c, time 3
    fn op_jmp_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.jmp(bus, addr);
    }

    // 0x4d, time 4
    fn op_eor_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.eor(bus, val);
    }

    // 0x4e, time 6
    fn op_lsr_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.lsr_mem(bus, addr);
    }

    // 0x4f, time 6, unofficial
    fn op_sre_abs(&mut self, bus: &mut dyn Bus) {
        panic!("op_jmp_abs is not implemented");
    }

    // 0x50, time 2+
    fn op_bvc_rel(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_rel(bus);
        self.bvc(bus, addr);
    }

    // 0x51, time 5
    fn op_eor_izy(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izy(bus);
        self.eor(bus, val);
    }

    // 0x52 hlt

    // 0x53, time 8, unofficial
    fn op_sre_izy(&mut self, bus: &mut dyn Bus) {
        panic!("op_sre_izy is not implemented");
    }

    // 0x54 nop_zpx

    // 0x55, time 4
    fn op_eor_zpx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zpx(bus);
        self.eor(bus, val);
    }

    // 0x56, time 6
    fn op_lsr_zpx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zpx(bus);
        self.lsr_mem(bus, addr);
    }

    // 0x57, time 6, unofficial
    fn op_sre_zpx(&mut self, bus: &mut dyn Bus) {
        panic!("op_sre_zpx is not implemented");
    }

    // 0x58, time 2
    fn op_cli(&mut self, bus: &mut dyn Bus) {
        self.i = false;
    }

    // 0x59, time 4+
    fn op_eor_aby(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_aby(bus);
        self.eor(bus, val);
    }

    // 0x5a nop

    // 0x5b, time 7, unofficial
    fn op_sre_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_sre_aby is not implemented");
    }

    // 0x5c nop_abx

    // 0x5d, time 4
    fn op_eor_abx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abx(bus);
        self.eor(bus, val);
    }

    // 0x5e, time 7
    fn op_lsr_abx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abx(bus);
        self.lsr_mem(bus, addr);
    }

    // 0x5f, time 7, unofficial
    fn op_sre_abx(&mut self, bus: &mut dyn Bus) {
        panic!("op_sre_abx is not implemented");
    }

    // 0x60, time 6
    fn op_rts(&mut self, bus: &mut dyn Bus) {
        let addr = self.stack_pop_word(bus) + 1;
        self.pc = addr;
    }

    // 0x61, time 6
    fn op_adc_izx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izx(bus);
        self.adc(bus, val);
    }

    // 0x62 hlt

    // 0x63, time 8, unofficial
    fn op_rra_izx(&mut self, bus: &mut dyn Bus) {
        panic!("op_rra_izx is not implemented");
    }

    // 0x64 nop_zp

    // 0x65, time 3
    fn op_adc_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.adc(bus, val);
    }

    // 0x66, time 5
    fn op_ror_zp(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zp(bus);
        self.ror_mem(bus, addr);
    }

    // 0x67, time 5
    fn op_rra_zp(&mut self, bus: &mut dyn Bus) {
        panic!("op_rra_zp is not implemented");
    }

    // 0x68, time 4
    fn op_pla(&mut self, bus: &mut dyn Bus) {
        let val = self.stack_pop_byte(bus);
        self.a = val;
        self.compute_nz();

    }

    // 0x69, time 2
    fn op_adc_imm(&mut self, bus: &mut dyn Bus) {
        // TODO : Deal with setting, checking flags
        let val = self.fetch_byte(bus);
        self.adc(bus, val);
    }

    // 0x6a, time 2
    fn op_ror(&mut self, bus: &mut dyn Bus) {
        let val = self.a;
        self.a = self.ror_val(bus, val);
    }

    // 0x6b, time 2, unofficial
    fn op_arr_imm(&mut self, bus: &mut dyn Bus) {
        panic!("op_arr_imm is not implemented");
    }

    // 0x6c, time 5
    fn op_jmp_ind(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_ind(bus);
        self.jmp(bus, addr);
    }

    // 0x6d, time 4
    fn op_adc_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.adc(bus, val);
    }

    // 0x6e, time 6
    fn op_ror_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.ror_mem(bus, addr);
    }

    // 0x6f, time 6
    fn op_rra_abs(&mut self, bus: &mut dyn Bus) {
        panic!("op_rra_abs is not implemented");
    }

    // 0x70, time 2
    fn op_bvs_rel(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_rel(bus);
        self.bvs(bus, addr);
    }

    // 0x71, time 5+
    fn op_adc_izy(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izy(bus);
        self.adc(bus, val);
    }

    // 0x72 hlt

    // 0x73, time 8, unofficial
    fn op_rra_izy(&mut self, bus: &mut dyn Bus) {
        panic!("op_rra_izy is not implemented");
    }

    // 0x74 nop_zpx

    // 0x75, time 4
    fn op_adc_zpx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zpx(bus);
        self.adc(bus, val);
    }

    // 0x76, time 6
    fn op_ror_zpx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zpx(bus);
        self.ror_mem(bus, addr);
    }

    // 0x77, time 6, unofficial
    fn op_rra_zpx(&mut self, bus: &mut dyn Bus) {
        panic!("op_rra_izy is not implemented");
    }

    // 0x78, time 2
    fn op_sei(&mut self, bus: &mut dyn Bus) {
        self.i = true;
    }

    // 0x79, time 4+
    fn op_adc_aby(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_aby(bus);
        self.adc(bus, val);
    }

    // 0x7a nop

    // 0x7b, time 7
    fn op_rda_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_rda_aby is not implemented");
    }

    // 0x7c nop_abx

    // 0x7d, time 4
    fn op_adc_abx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abx(bus);
        self.adc(bus, val);
    }

    // 0x7e, time 7
    fn op_ror_abx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abx(bus);
        self.ror_mem(bus, addr);
    }

    // 0x7f, time 7
    fn op_rra_abx(&mut self, bus: &mut dyn Bus) {
        panic!("op_rra_abx is not implemented");
    }

    // 0x80 nop_imm
    fn op_nop_imm(&mut self, bus: &mut dyn Bus) {
        self.pc += 1;
    }

    // 0x81, time 6
    fn op_sta_izx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_izx(bus);
        self.sta(bus, addr);
    }

    // 0x82 nop_imm

    // 0x83
    fn op_sax_izx(&mut self, bus: &mut dyn Bus) {
        panic!("op_rra_abx is not implemented");
    }

    // 0x84, time 3
    fn op_sty_zp(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zp(bus);
        self.sty(bus, addr);
    }

    // 0x85, time 3
    fn op_sta_zp(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zp(bus);
        self.sta(bus, addr);
    }

    // 0x86, time 3
    fn op_stx_zp(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zp(bus);
        self.stx(bus, addr);
    }

    // 0x87, time 3
    fn op_sax_zp(&mut self, bus: &mut dyn Bus) {
        panic!("op_sax_zp is not implemented");
    }

    // 0x88, time 2
    fn op_dey(&mut self, bus: &mut dyn Bus) {
        self.y = self.y.wrapping_sub(1);
        self.compute_nz_val(self.y);
    }

    // 0x89 nop_imm

    // 0x8a, time 2
    fn op_txa(&mut self, bus: &mut dyn Bus) {
        self.a = self.x;
        self.compute_nz_val(self.a);
    }

    // 0x8b, time 2, unofficial
    fn op_xaa_imm(&mut self, bus: &mut dyn Bus) {
        panic!("op_xaa_imm is not implemented");
    }

    // 0x8c, time 4
    fn op_sty_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.sty(bus, addr);
    }

    // 0x8d, time 4
    fn op_sta_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.sta(bus, addr);
    }

    // 0x8e, time 4
    fn op_stx_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.stx(bus, addr);
    }

    // 0x8f, time 4, unofficial
    fn op_sax_abs(&mut self, bus: &mut dyn Bus) {
        panic!("op_sax_abs is not implemented");
    }

    // 0x90, time 2+
    fn op_bcc_rel(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_rel(bus);
        self.bcc(bus, addr);
    }

    // 0x91, time 6
    fn op_sta_izy(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_izy(bus);
        self.sta(bus, addr);
    }

    // 0x92 hlt

    // 0x93, time 6
    fn op_ahx_izy(&mut self, bus: &mut dyn Bus) {
        panic!("op_ahx_izy is not implemented");
    }

    // 0x94, time 4
    fn op_sty_zpx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zpx(bus);
        self.sty(bus, addr);
    }

    // 0x95, time 4
    fn op_sta_zpx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zpx(bus);
        self.sta(bus, addr);
    }

    // 0x96, time 4
    fn op_stx_zpy(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zpy(bus);
        self.stx(bus, addr);
    }

    // 0x97, time 4, unofficial
    fn op_sax_zpy(&mut self, bus: &mut dyn Bus) {
        panic!("op_ahx_izy is not implemented");
    }

    // 0x98, time 2
    fn op_tya(&mut self, bus: &mut dyn Bus) {
        self.a = self.y;
        self.compute_nz_val(self.a);
    }

    // 0x99, time 5
    fn op_sta_aby(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_aby(bus);
        self.sta(bus, addr);
    }

    // 0x9a, time 2
    fn op_txs(&mut self, bus: &mut dyn Bus) {
        self.s = self.x;
    }

    // 0x9b, time 5
    fn op_tas_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_tas_aby is not implemented");
    }

    // 0x9c, time 5
    fn op_shy_abx(&mut self, bus: &mut dyn Bus) {
        panic!("op_shy_abx is not implemented");
    }

    // 0x9d, time 5
    fn op_sta_abx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abx(bus);
        self.sta(bus, addr);
    }

    // 0x9e, time 5
    fn op_shx_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_shx_aby is not implemented");
    }

    // 0x9f, time 5
    fn op_ahx_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_ahx_aby is not implemented");
    }

    // 0xa0, time 2
    fn op_ldy_imm(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_byte(bus);
        self.ldy(bus, val);
    }

    // 0xa1, time 6
    fn op_lda_izx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izx(bus);
        self.lda(bus, val);
    }

    // 0xa2, time 2
    fn op_ldx_imm(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_byte(bus);
        self.ldx(bus, val);
    }

    // 0xa3, time 6, unofficial
    fn op_lax_izx(&mut self, bus: &mut dyn Bus) {
        panic!("op_lax_izx is not implemented");
    }

    // 0xa4, time 3
    fn op_ldy_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.ldy(bus, val);
    }

    // 0xa5, time 3
    fn op_lda_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.lda(bus, val);
    }

    // 0xa6, time 3
    fn op_ldx_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.ldx(bus, val);
    }

    // 0xa7, time 3
    fn op_lax_zp(&mut self, bus: &mut dyn Bus) {
        panic!("op_lax_zp is not implemented");
    }

    // 0xa8, time 2
    fn op_tay(&mut self, bus: &mut dyn Bus) {
        self.y = self.a;
        self.compute_nz_val(self.y);
    }

    // 0xa9, time 2
    fn op_lda_imm(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_byte(bus);
        self.lda(bus, val);
    }

    // 0xaa, time 2
    fn op_tax(&mut self, bus: &mut dyn Bus) {
        self.x = self.a;
        self.compute_nz_val(self.x);
    }

    // 0xab, time 2, unofficial
    fn op_lax_imm(&mut self, bus: &mut dyn Bus) {
        panic!("op_lax_imm is not implemented");
    }

    // 0xac, time 4
    fn op_ldy_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.ldy(bus, val);
    }

    // 0xad, time 4
    fn op_lda_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.lda(bus, val);
    }

    // 0xae, time 4
    fn op_ldx_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.ldx(bus, val);
    }

    // 0xaf, time 4
    fn op_lax_abs(&mut self, bus: &mut dyn Bus) {
        panic!("op_lax_imm is not implemented");
    }

    // 0xb0, time 2
    fn op_bcs_rel(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_rel(bus);
        self.bcs(bus, addr);
    }

    // 0xb1, time 5
    fn op_lda_izy(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izy(bus);
        self.lda(bus, val);
    }

    // 0xb2 hlt

    // 0xb3, time 5+, unofficial
    fn op_lax_izy(&mut self, bus: &mut dyn Bus) {
        panic!("op_lax_imm is not implemented");
    }

    // 0xb4, time 4
    fn op_ldy_zpx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zpx(bus);
        self.ldy(bus, val);
    }

    // 0xb5, time 4
    fn op_lda_zpx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zpx(bus);
        self.lda(bus, val);
    }

    // 0xb6, time 4
    fn op_ldx_zpy(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zpy(bus);
        self.ldx(bus, val);
    }

    // 0xb7, time 4
    fn op_lax_zpy(&mut self, bus: &mut dyn Bus) {
        panic!("op_lax_imm is not implemented");
    }

    // 0xb8, time 2
    fn op_clv(&mut self, bus: &mut dyn Bus) {
        self.v = false;
    }

    // 0xb9, time 4+
    fn op_lda_aby(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_aby(bus);
        self.lda(bus, val);
    }

    // 0xba, time 2
    fn op_tsx(&mut self, bus: &mut dyn Bus) {
        self.x = self.s;
        self.compute_nz_val(self.x);
    }

    // 0xbb, time 4+, unofficial
    fn op_las_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_las_aby is not implemented");
    }

    // 0xbc, time 4+
    fn op_ldy_abx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abx(bus);
        self.ldy(bus, val);
    }

    // 0xbd, time 4+
    fn op_lda_abx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abx(bus);
        self.lda(bus, val);
    }

    // 0xbe, time 4+
    fn op_ldx_aby(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_aby(bus);
        self.ldx(bus, val);
    }

    // 0xbf, time 4+, unofficial
    fn op_lax_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_lax_aby is not implemented");
    }

    // 0xc0, time 2
    fn op_cpy_imm(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_byte(bus);
        self.cmp(bus, self.y, val);
    }

    // 0xc1, time 6
    fn op_cmp_izx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izx(bus);
        self.cmp(bus, self.a, val);
    }

    // 0xc2 nop_imm

    // 0xc3, time 8, unofficial
    fn op_dcp_izx(&mut self, bus: &mut dyn Bus) {
        panic!("op_lax_aby is not implemented");
    }

    // 0xc4, time 3
    fn op_cpy_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.cmp(bus, self.y, val);
    }

    // 0xc5, time 3
    fn op_cmp_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.cmp(bus, self.a, val);
    }

    // 0xc6, time 5
    fn op_dec_zp(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zp(bus);
        self.dec(bus, addr);
    }

    // 0xc7, time 5, unofficial
    fn op_dcp_zp(&mut self, bus: &mut dyn Bus) {
        panic!("op_dcp_zp is not implemented");
    }

    // 0xc8, time 2
    fn op_iny(&mut self, bus: &mut dyn Bus) {
        self.y = self.y.wrapping_add(1);
        self.compute_nz_val(self.y);
    }

    // 0xc9, time 2
    fn op_cmp_imm(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_byte(bus);
        self.cmp(bus, self.a, val);
    }

    // 0xca, time 2
    fn op_dex(&mut self, bus: &mut dyn Bus) {
        self.x = self.x.wrapping_sub(1);
        self.compute_nz_val(self.x);
    }

    // 0xcb, time 2, unofficial
    fn op_axs_imm(&mut self, bus: &mut dyn Bus) {
        panic!("op_dcp_zp is not implemented");
    }

    // 0xcc, time 4
    fn op_cpy_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.cmp(bus, self.y, val);
    }

    // 0xcd, time 4
    fn op_cmp_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.cmp(bus, self.a, val);
    }

    // 0xce, time 6
    fn op_dec_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.dec(bus, addr);
    }

    // 0xcf, time 6
    fn op_dcp_abs(&mut self, bus: &mut dyn Bus) {
        panic!("op_dcp_abs is not implemented");
    }

    // 0xd0, time 2+
    fn op_bne_rel(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_rel(bus);
        self.bne(bus, addr);
    }

    // 0xd1, time 5+
    fn op_cmp_izy(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izy(bus);
        self.cmp(bus, self.a, val);
    }

    // 0xd2 hlt

    // 0xd3, time 8
    fn op_dcp_izy(&mut self, bus: &mut dyn Bus) {
        panic!("op_dcp_izy is not implemented");
    }

    // 0xd4 nop_zpx

    // 0xd5, time 4
    fn op_cmp_zpx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zpx(bus);
        self.cmp(bus, self.a, val);
    }

    // 0xd6, time 6
    fn op_dec_zpx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zpx(bus);
        self.dec(bus, addr);
    }

    // 0xd7, time 6
    fn op_dcp_zpx(&mut self, bus: &mut dyn Bus) {
        panic!("op_dcp_izy is not implemented");
    }

    // 0xd8, time 2
    fn op_cld(&mut self, bus: &mut dyn Bus) {
        self.d = false;
    }

    // 0xd9, time 4+
    fn op_cmp_aby(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_aby(bus);
        self.cmp(bus, self.a, val);
    }

    // 0xda nop

    // 0xdb, time 7
    fn op_dcp_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_dcp_aby is not implemented");
    }

    // 0xdc nop_abx

    // 0xdd, time 4
    fn op_cmp_abx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abx(bus);
        self.cmp(bus, self.a, val);
    }

    // 0xde, time 7
    fn op_dec_abx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abx(bus);
        self.dec(bus, addr);
    }

    // 0xdf, time 7
    fn op_dcp_abx(&mut self, bus: &mut dyn Bus) {
        panic!("op_dcp_abx is not implemented");
    }

    // 0xe0, time 2
    fn op_cpx_imm(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_byte(bus);
        self.cmp(bus, self.x, val);
    }

    // 0xe1, time 6
    fn op_sbc_izx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izx(bus);
        self.sbc(bus, val);
    }

    // 0xe2 nop_imm

    // 0xe3, time 8
    fn op_isc_izx(&mut self, bus: &mut dyn Bus) {
        panic!("op_isc_izx is not implemented");
    }

    // 0xe4, time 3
    fn op_cpx_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.cmp(bus, self.x, val);
    }

    // 0xe5, time 3
    fn op_sbc_zp(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zp(bus);
        self.sbc(bus, val);
    }

    // 0xe6, time 5
    fn op_inc_zp(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zp(bus);
        self.inc(bus, addr);
    }

    // 0xe7, time 5
    fn op_isc_zp(&mut self, bus: &mut dyn Bus) {
        panic!("op_isc_zp is not implemented");
    }

    // 0xe8, time 2
    fn op_inx(&mut self, bus: &mut dyn Bus) {
        self.x = self.x.wrapping_add(1);
        self.compute_nz_val(self.x);
    }

    // 0xe9, time 2
    fn op_sbc_imm(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_byte(bus);
        self.sbc(bus, val);
    }

    // 0xea nop, official

    // 0xeb sbc_imm, unofficial

    // 0xec, time 4
    fn op_cpx_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.cmp(bus, self.x, val);
    }

    // 0xed, time 4
    fn op_sbc_abs(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abs(bus);
        self.sbc(bus, val);
    }

    // 0xee, time 6
    fn op_inc_abs(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abs(bus);
        self.inc(bus, addr);
    }

    // 0xef, time 6
    fn op_isc_abs(&mut self, bus: &mut dyn Bus) {
        panic!("op_isc_abs is not implemented");
    }

    // 0xf0, time 2+
    fn op_beq_rel(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_rel(bus);
        self.beq(bus, addr);
    }

    // 0xf1, time 5+
    fn op_sbc_izy(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_izy(bus);
        self.sbc(bus, val);
    }

    // 0xf2 hlt

    // 0xf3, time 8
    fn op_isc_izy(&mut self, bus: &mut dyn Bus) {
        panic!("op_isc_izy is not implemented");
    }

    // 0xf4 nop_zpx

    // 0xf5, time 4
    fn op_sbc_zpx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_zpx(bus);
        self.sbc(bus, val);
    }

    // 0xf6, time 6
    fn op_inc_zpx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_zpx(bus);
        self.inc(bus, addr);
    }

    // 0xf7, time 6
    fn op_isc_zpx(&mut self, bus: &mut dyn Bus) {
        panic!("op_isc_izy is not implemented");
    }

    // 0xf8, time 2
    fn op_sed(&mut self, bus: &mut dyn Bus) {
        self.d = true;
    }

    // 0xf9, time 4+
    fn op_sbc_aby(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_aby(bus);
        self.sbc(bus, val);
    }

    // 0xfa nop

    // 0xfb, time 7
    fn op_isc_aby(&mut self, bus: &mut dyn Bus) {
        panic!("op_isc_aby is not implemented");
    }

    // 0xfc nop_abx

    // 0xfd, time 4+
    fn op_sbc_abx(&mut self, bus: &mut dyn Bus) {
        let val = self.fetch_val_mode_abx(bus);
        self.sbc(bus, val);
    }

    // 0xfe, time 7
    fn op_inc_abx(&mut self, bus: &mut dyn Bus) {
        let addr = self.fetch_addr_mode_abx(bus);
        self.inc(bus, addr);
    }

    // 0xff, time 7
    fn op_isc_abx(&mut self, bus: &mut dyn Bus) {
        panic!("op_isc_abx is not implemented");
    }

    // Implementations of core functionality once the address has been
    // computed
    fn adc(&mut self, bus: &mut dyn Bus, val : u8) {
        if self.d {
            self.adc_dec(bus, val);
        } else {
            self.adc_bin(bus, val);
        }
    }

    fn adc_bin(&mut self, bus: &mut dyn Bus, val : u8) {
        // Add numbers twice: once in signed, the other unsigned. This gets us
        // the v and c flags.
        let (mut u_sum, mut u_overflow) = self.a.overflowing_add(val);
        let (mut s_sum, mut s_overflow) = (self.a as i8).overflowing_add(val as i8);

        if self.c {
            let(u_sumc, u_overflowc) = u_sum.overflowing_add(1);
            let(s_sumc, s_overflowc) = s_sum.overflowing_add(1);

            u_sum = u_sumc;
            u_overflow = u_overflow || u_overflowc;
            s_overflow = s_overflow != s_overflowc; // Carry bit can re-toggle overflow.
        }

        self.c = u_overflow;
        self.v = s_overflow;
        self.a = u_sum;

        // Z is valid, N is invalid but reportedly computed the same as binary.
        self.compute_nz();
    }

    fn adc_dec(&mut self, bus: &mut dyn Bus, val : u8) {
        let mut lo = (self.a & 0x0f) + (val & 0x0f);
        let mut hi = ((self.a & 0xf0) >> 4) + ((val & 0xf0) >> 4);

        if self.c {
            lo += 1;
        }

        if lo > 9 {
            hi += 1;
            lo -= 10;
        }

        if hi > 9 {
            self.c = true;
            hi -= 10;
        } else {
            self.c = false;
        }

        self.v = self.adc_dec_compute_v(val);
        self.a = ((hi & 0x0f) << 4) | (lo & 0x0f);

        self.compute_nz();
    }

    fn adc_dec_compute_v(&mut self, val: u8) -> bool {
        // v is "undefined" but is reported to act as if in binary mode.
        let (mut s_sum, mut s_overflow) = (self.a as i8).overflowing_add(val as i8);
        if self.c {
            let(s_sumc, s_overflowc) = s_sum.overflowing_add(1);
            s_overflow = s_overflow != s_overflowc; // Carry bit can re-toggle overflow.
        }

        s_overflow
    }

    fn and(&mut self, bus: &mut dyn Bus, val : u8) {
        self.a = self.a & val;
        self.compute_nz();
    }

    fn asl_mem(&mut self, bus: &mut dyn Bus, addr: u16) {
        let val = bus.read(addr);
        let new_val = self.asl_val(bus, val);
        bus.write(addr, new_val);
    }

    fn asl_val(&mut self, bus: &mut dyn Bus, val: u8) -> u8 {
        let carry = val >= 0x80;
        let new_val = val.wrapping_shl(1);
        self.c = carry;
        self.compute_nz_val(new_val);
        new_val
    }

    fn bcs(&mut self, bus: &mut dyn Bus, addr: u16) {
        if self.c {
            self.check_addr(addr);
            self.pc = addr;
        }
    }

    fn bcc(&mut self, bus: &mut dyn Bus, addr: u16) {
        if !self.c {
            self.check_addr(addr);
            self.pc = addr;
        }
    }

    fn beq(&mut self, bus: &mut dyn Bus, addr: u16) {
        if self.z {
            self.check_addr(addr);
            self.pc = addr;
        }
    }

    fn bmi(&mut self, bus: &mut dyn Bus, addr: u16) {
        if self.n {
            self.check_addr(addr);
            self.pc = addr;
        }
    }

    fn bne(&mut self, bus: &mut dyn Bus, addr: u16) {
        if !self.z {
            self.check_addr(addr);
            self.pc = addr;
        }
    }

    fn bpl(&mut self, bus: &mut dyn Bus, addr: u16) {
        if !self.n {
            self.check_addr(addr);
            self.pc = addr;
        }
    }

    fn bvc(&mut self, bus: &mut dyn Bus, addr: u16) {
        if !self.v {
            self.check_addr(addr);
            self.pc = addr;
        }
    }

    fn bvs(&mut self, bus: &mut dyn Bus, addr: u16) {
        if self.v {
            self.check_addr(addr);
            self.pc = addr;
        }
    }

    fn check_addr(&self, addr: u16) {
        // TODO : Remove this. This is used for running tests and detecting
        // failure.
        if (self.pc as i16).wrapping_add(-(addr as i16)) == 2 {
            println!("{}", self.state_string());
            panic!("Encountered trap.");
        }
    }

    //	Set flags only. n and v are set to val bits 7 and 6. z is AND of a and val
    fn bit(&mut self, bus: &mut dyn Bus, val: u8) {
        self.n = val & 0x80 != 0;
        self.v = val & 0x40 != 0;
        self.z = val & self.a == 0;
    }

    fn cmp(&mut self, bus: &mut dyn Bus, val1: u8, val2: u8) {
        let (delta, overflow) = val1.overflowing_sub(val2);
        // This is unintuitive, but CMP is like SBC with an implied carry bit already set.
        self.c = !overflow;
        self.compute_nz_val(delta)
    }

    fn dec(&mut self, bus: &mut dyn Bus, addr: u16) {
        let val = bus.read(addr);
        let new_val = val.wrapping_sub(1);
        bus.write(addr, new_val);
        self.compute_nz_val(new_val);
    }

    fn eor(&mut self, bus: &mut dyn Bus, val: u8) {
        self.a = self.a ^ val;
        self.compute_nz();
    }

    fn inc(&mut self, bus: &mut dyn Bus, addr: u16) {
        let val = bus.read(addr);
        let new_val = val.wrapping_add(1);
        bus.write(addr, new_val);
        self.compute_nz_val(new_val);
    }

    fn jmp(&mut self, bus: &mut dyn Bus, addr: u16) {
        self.pc = addr;
    }

    fn lda(&mut self, bus: &mut dyn Bus, val: u8) {
        self.a = val;
        self.compute_nz();
    }

    fn ldx(&mut self, bus: &mut dyn Bus, val: u8) {
        self.x = val;
        self.compute_nz_val(self.x);
    }

    fn ldy(&mut self, bus: &mut dyn Bus, val: u8) {
        self.y = val;
        self.compute_nz_val(self.y);
    }

    fn lsr_mem(&mut self, bus: &mut dyn Bus, addr: u16) {
        let val = bus.read(addr);
        let new_val = self.lsr_val(bus, val);
        bus.write(addr, new_val);
    }

    fn lsr_val(&mut self, bus: &mut dyn Bus, val: u8) -> u8 {
        self.c = val & 0x01_u8 == 0x01u8;
        let new_val = val >> 1;
        self.compute_nz_val(new_val);
        new_val
    }

    fn ora(&mut self, bus: &mut dyn Bus, val : u8) {
        self.a = self.a | val;
        self.compute_nz();
    }

    fn rol_mem(&mut self, bus: &mut dyn Bus, addr: u16) {
        let val = bus.read(addr);
        let new_val = self.rol_val(bus, val);
        bus.write(addr, new_val);
    }

    fn rol_val(&mut self, bus: &mut dyn Bus, val: u8) -> u8 {
        let carry = val >= 0x80;
        let val2 = val.wrapping_shl(1);
        let c = self.c;
        self.c = carry;
        let new_val = val2 | if c {0x01} else {0x00};
        self.compute_nz_val(new_val);
        new_val
    }

    fn ror_mem(&mut self, bus: &mut dyn Bus, addr: u16) {
        let val = bus.read(addr);
        let new_val = self.ror_val(bus, val);
        bus.write(addr, new_val);
    }

    fn ror_val(&mut self, bus: &mut dyn Bus, val: u8) -> u8 {
        let new_c = val & 0x01 == 0x01;
        let val2 = val >> 1;
        let c = self.c;
        let new_val = val2 | if c {0x80} else {0x00};
        self.c = new_c;
        self.compute_nz_val(new_val);
        new_val
    }

    fn sbc(&mut self, bus: &mut dyn Bus, val : u8) {
        // Note : Based on adc, keep in sync.
        // TODO : Deal with BCD mode
        if self.d {
            self.sbc_dec(bus, val);
        } else {
            self.sbc_bin(bus, val);
        }
    }

    fn sbc_bin(&mut self, bus: &mut dyn Bus, val : u8) {
        // Add numbers twice: once in signed, the other unsigned. This gets us
        // the v and c flags.
        let (mut u_sum, mut u_overflow) = self.a.overflowing_sub(val);
        let (mut s_sum, mut s_overflow) = (self.a as i8).overflowing_sub(val as i8);

        if !self.c {
            let(u_sumc, u_overflowc) = u_sum.overflowing_sub(1);
            let(s_sumc, s_overflowc) = s_sum.overflowing_sub(1);

            u_sum = u_sumc;
            u_overflow = u_overflow || u_overflowc;
            s_overflow = s_overflow != s_overflowc; // Carry bit can re-toggle overflow.
        }

        self.c = !u_overflow;
        self.v = s_overflow;
        self.a = u_sum;

        self.compute_nz();
    }

    fn sbc_dec(&mut self, bus: &mut dyn Bus, val : u8) {
        let mut lo = (self.a & 0x0f).wrapping_sub(val & 0x0f);
        let mut hi = ((self.a & 0xf0) >> 4).wrapping_sub((val & 0xf0) >> 4);

        if !self.c {
            lo = lo.wrapping_sub(1);
        }

        if lo & 0x80 == 0x80 {
            hi = hi.wrapping_sub(1);
            lo = lo.wrapping_add(10);
        }

        if hi & 0x80 == 0x80 {
            self.c = false;
            hi = hi.wrapping_add(10);
        } else {
            self.c = true;
        }

        self.v = self.sbc_dec_compute_v(val);
        self.a = ((hi & 0x0f) << 4) | (lo & 0x0f);

        self.compute_nz();
    }

    fn sbc_dec_compute_v(&self, val: u8) -> bool {
        let (mut s_sum, mut s_overflow) = (self.a as i8).overflowing_sub(val as i8);
        if !self.c {
            let(s_sumc, s_overflowc) = s_sum.overflowing_sub(1);
            s_overflow = s_overflow != s_overflowc; // Carry bit can re-toggle overflow.
        }

        s_overflow
    }

    fn sta(&mut self, bus: &mut dyn Bus, addr : u16) {
        bus.write(addr, self.a);
    }

    fn stx(&mut self, bus: &mut dyn Bus, addr : u16) {
        bus.write(addr, self.x);
    }

    fn sty(&mut self, bus: &mut dyn Bus, addr : u16) {
        bus.write(addr, self.y);
    }

    fn compute_nz(&mut self) {
        self.compute_nz_val(self.a);
    }

    fn compute_nz_val(&mut self, val: u8) {
        self.n = val >= 0x80;
        self.z = val == 0;
    }

    // Stack functions
    fn stack_push_byte(&mut self, bus: &mut dyn Bus, val : u8) {
        let addr = self.addr_stack();
        bus.write(addr, val);
        self.s = self.s.wrapping_sub(1);
    }

    fn stack_pop_byte(&mut self, bus: &mut dyn Bus) -> u8 {
        self.s = self.s.wrapping_add(1);
        let addr = self.addr_stack();
        bus.read(addr)
    }

    fn stack_push_word(&mut self, bus: &mut dyn Bus, val : u16) {
        let addr = self.addr_stack() - 1;
        bus.write_word(addr, val);
        self.s = self.s.wrapping_sub(2);
    }

    fn stack_pop_word(&mut self, bus: &mut dyn Bus) -> u16 {
        self.s = self.s.wrapping_add(1);
        let addr = self.addr_stack();
        let val = bus.read_word(addr);
        self.s = self.s.wrapping_add(1);
        val
    }

    fn get_status(&self, brk: bool) -> u8 {
        let mut st = STATUS_RES;
        if self.n {
            st = st | STATUS_NEG;
        }
        if self.v {
            st = st | STATUS_OVR;
        }
        // if self.b {
        if brk {
            st = st | STATUS_BRK;
        }
        if self.d {
            st = st | STATUS_DCM;
        }
        if self.i {
            st = st | STATUS_INT;
        }
        if self.z {
            st = st | STATUS_ZER;
        }
        if self.c {
            st = st | STATUS_CAR;
        }

        st
    }

    fn set_status(&mut self, status: u8, affect_brk: bool) {
        // let mut st = STATUS_OVR;

        self.n = status & STATUS_NEG == STATUS_NEG;
        self.v = status & STATUS_OVR == STATUS_OVR;
        if (affect_brk) {
            self.b = status & STATUS_BRK == STATUS_BRK;
        }
        self.d = status & STATUS_DCM == STATUS_DCM;
        self.i = status & STATUS_INT == STATUS_INT;
        self.z = status & STATUS_ZER == STATUS_ZER;
        self.c = status & STATUS_CAR == STATUS_CAR;
    }
}

// use std::rc::Rc;
// use std::cell::RefCell;

// use crate::addressable::Addressable;
// use crate::cpu_pins::CpuPins;
// use crate::cpu_table::CpuTable;
// use crate::mem::Mem;

// const STACK_BASE: u16 = 0x0100_u16;
// const VECTOR_NMI: u16 = 0xfffa_u16;
// const VECTOR_RESET: u16 = 0xfffc_u16;
// const VECTOR_IRQBRK: u16 = 0xfffe_u16;
// const STATUS_NEG : u8 = 0x80_u8;
// const STATUS_OVR : u8 = 0x40_u8;
// const STATUS_RES : u8 = 0x20_u8;
// const STATUS_BRK : u8 = 0x10_u8;
// const STATUS_DCM : u8 = 0x08_u8;
// const STATUS_INT : u8 = 0x04_u8;
// const STATUS_ZER : u8 = 0x02_u8;
// const STATUS_CAR : u8 = 0x01_u8;

// const MC_SEQ_BREAK : [fn(&mut Cpu); 14] = [
//     Cpu::mc_no_op,     Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,  Cpu::mc_no_op,
//     Cpu::mc_push_pch,  Cpu::mc_no_op,
//     Cpu::mc_push_pcl,  Cpu::mc_no_op,
//     Cpu::mc_push_p,    Cpu::mc_no_op,
//     Cpu::mc_fetch_pcl, Cpu::mc_no_op,
//     Cpu::mc_fetch_pch, Cpu::mc_no_op,
// ];

// const MC_SEQ_RTI : [fn(&mut Cpu); 12] = [
//     Cpu::mc_no_op,    Cpu::mc_no_op,
//     Cpu::mc_read_pc,  Cpu::mc_no_op,
//     Cpu::mc_inc_s,    Cpu::mc_no_op,
//     Cpu::mc_pull_p,   Cpu::mc_no_op,
//     Cpu::mc_pull_pcl, Cpu::mc_no_op,
//     Cpu::mc_pull_pch, Cpu::mc_no_op,
// ];

// const MC_SEQ_RTS : [fn(&mut Cpu); 12] = [
//     Cpu::mc_no_op,    Cpu::mc_no_op,
//     Cpu::mc_read_pc,  Cpu::mc_no_op,
//     Cpu::mc_inc_s,    Cpu::mc_no_op,
//     Cpu::mc_pull_pcl, Cpu::mc_no_op,
//     Cpu::mc_pull_pch, Cpu::mc_no_op,
//     Cpu::mc_inc_pc,   Cpu::mc_no_op,
// ];

// const MC_SEQ_PHX : [fn(&mut Cpu); 6] = [
//     Cpu::mc_no_op,       Cpu::mc_no_op,
//     Cpu::mc_read_pc,     Cpu::mc_no_op,
//     Cpu::mc_op_sentinel, Cpu::mc_no_op,
// ];

// const MC_SEQ_PLX : [fn(&mut Cpu); 8] = [
//     Cpu::mc_no_op,       Cpu::mc_no_op,
//     Cpu::mc_read_pc,     Cpu::mc_no_op,
//     Cpu::mc_inc_s,       Cpu::mc_no_op,
//     Cpu::mc_op_sentinel, Cpu::mc_no_op,
// ];

// const MC_JSR : [fn(&mut Cpu); 12] = [
//     Cpu::mc_no_op,          Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,       Cpu::mc_no_op,
//     Cpu::mc_predecrement_s, Cpu::mc_no_op,
//     Cpu::mc_push_pch,       Cpu::mc_no_op,
//     Cpu::mc_push_pcl,       Cpu::mc_no_op,
//     Cpu::mc_jsr,            Cpu::mc_no_op,
// ];

// const MC_IMM_ACC_IMPL: [fn(&mut Cpu); 4] = [
//     Cpu::mc_no_op,        Cpu::mc_no_op,
//     Cpu::mc_op_sentinel,  Cpu::mc_no_op,
// ];

// const MC_ABS_JMP: [fn(&mut Cpu); 6] = [
//     Cpu::mc_no_op,    Cpu::mc_no_op,
//     Cpu::mc_fetch_pc, Cpu::mc_addr_reg_lo,
//     Cpu::mc_fetch_pc, Cpu::mc_addr_reg_hi_set_pc,
// ];

// const MC_ABS_READ: [fn(&mut Cpu); 8] = [
//     Cpu::mc_no_op,         Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,      Cpu::mc_addr_reg_lo,
//     Cpu::mc_fetch_pc,      Cpu::mc_addr_reg_hi,
//     Cpu::mc_read_addr_reg, Cpu::mc_op_sentinel,
// ];

// const MC_ABS_WRITE: [fn(&mut Cpu); 8] = [
//     Cpu::mc_no_op,       Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,    Cpu::mc_addr_reg_lo,
//     Cpu::mc_fetch_pc,    Cpu::mc_addr_reg_hi,
//     Cpu::mc_op_sentinel, Cpu::mc_no_op,
// ];

// const MC_ABS_MODIFY: [fn(&mut Cpu); 12] = [
//     Cpu::mc_no_op,          Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,       Cpu::mc_addr_reg_lo,
//     Cpu::mc_fetch_pc,       Cpu::mc_addr_reg_hi,
//     Cpu::mc_read_addr_reg,  Cpu::mc_data_reg,
//     Cpu::mc_write_addr_reg, Cpu::mc_op_sentinel,
//     Cpu::mc_write_addr_reg, Cpu::mc_no_op,
// ];

// const MC_ZP_READ: [fn(&mut Cpu); 6] = [
//     Cpu::mc_no_op,          Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,       Cpu::mc_addr_reg_lo_zp,
//     Cpu::mc_read_addr_reg,  Cpu::mc_op_sentinel,
// ];

// const MC_ZP_WRITE: [fn(&mut Cpu); 6] = [
//     Cpu::mc_no_op,       Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,    Cpu::mc_addr_reg_lo_zp,
//     Cpu::mc_op_sentinel, Cpu::mc_write,
// ];

// const MC_ZP_MODIFY: [fn(&mut Cpu); 10] = [
//     Cpu::mc_no_op,          Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,       Cpu::mc_addr_reg_lo_zp,
//     Cpu::mc_read_addr_reg,  Cpu::mc_no_op, // Cpu::mc_op_sentinel,?
//     Cpu::mc_write_addr_reg, Cpu::mc_op_sentinel,
//     Cpu::mc_write_addr_reg, Cpu::mc_no_op,
// ];

// const MC_ZP_XY_READ_WRITE: [fn(&mut Cpu); 8] = [
//     Cpu::mc_no_op,         Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,      Cpu::mc_addr_reg_lo_zp,
//     Cpu::mc_read_addr_reg, Cpu::mc_wrapping_add_reg_lo, // Gets x or y from mode
//     Cpu::mc_read_addr_reg, Cpu::mc_op_sentinel,
// ];

// const MC_ZP_X_MODIFY: [fn(&mut Cpu); 12] = [
//     Cpu::mc_no_op,          Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,       Cpu::mc_addr_reg_lo_zp,
//     Cpu::mc_read_addr_reg,  Cpu::mc_wrapping_add_reg_lo, // Gets x or y from mode
//     Cpu::mc_read_addr_reg,  Cpu::mc_no_op,
//     Cpu::mc_write_addr_reg, Cpu::mc_op_sentinel,
//     Cpu::mc_write_addr_reg, Cpu::mc_no_op,
// ];

// const MC_ABS_XY_READ: [fn(&mut Cpu); 10] = [
//     Cpu::mc_no_op,       Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,    Cpu::mc_addr_reg_lo,
//     Cpu::mc_fetch_pc,    Cpu::mc_addr_reg_hi_idx,
//     Cpu::mc_op_sentinel, Cpu::mc_no_op,
//     Cpu::mc_op_sentinel, Cpu::mc_no_op, // Only if page boundary crossed?
// ];

// const MC_ABS_XY_WRITE: [fn(&mut Cpu); 10] = [
//     Cpu::mc_no_op,         Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,      Cpu::mc_addr_reg_lo,
//     Cpu::mc_fetch_pc,      Cpu::mc_addr_reg_hi_idx,
//     Cpu::mc_read_addr_reg, Cpu::mc_no_op,
//     Cpu::mc_op_sentinel,   Cpu::mc_write,
// ];

// const MC_ABS_X_MODIFY: [fn(&mut Cpu); 14] = [
//     Cpu::mc_no_op,         Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,      Cpu::mc_addr_reg_lo,
//     Cpu::mc_fetch_pc,      Cpu::mc_addr_reg_hi_idx,
//     Cpu::mc_read_addr_reg, Cpu::mc_no_op,
//     Cpu::mc_read_addr_reg, Cpu::mc_no_op,
//     Cpu::mc_write,         Cpu::mc_op_sentinel,
//     Cpu::mc_write,         Cpu::mc_no_op,
// ];

// const MC_REL_BRANCH: [fn(&mut Cpu); 10] = [
//     Cpu::mc_no_op,         Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,      Cpu::mc_op_sentinel,
//     Cpu::mc_branch,        Cpu::mc_no_op, // Only if branch taken.
//     Cpu::mc_fix_pch,       Cpu::mc_no_op, // Only if pch was broken?
//     Cpu::mc_fetch_pc,      Cpu::mc_no_op, // Only if branch to different page ???
// ];

// const MC_INDEXED_INDIRECT_X_READ: [fn(&mut Cpu); 12] = [
//     Cpu::mc_no_op,             Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,          Cpu::mc_read_addr_reg,
//     Cpu::mc_read_addr_reg,     Cpu::mc_wrapping_add_reg_lo,
//     Cpu::mc_read_addr_reg,     Cpu::mc_inc_addr_reg,
//     Cpu::mc_read_addr_reg_hi,  Cpu::mc_no_op,
//     Cpu::mc_read,              Cpu::mc_op_sentinel,
// ];

// const MC_INDEXED_INDIRECT_X_WRITE: [fn(&mut Cpu); 12] = [
//     Cpu::mc_no_op,             Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,          Cpu::mc_read_addr_reg,
//     Cpu::mc_read_addr_reg,     Cpu::mc_wrapping_add_reg_lo,
//     Cpu::mc_read_addr_reg,     Cpu::mc_inc_addr_reg,
//     Cpu::mc_read_addr_reg_hi,  Cpu::mc_no_op,
//     Cpu::mc_op_sentinel,       Cpu::mc_op_write,
// ];

// const MC_INDEXED_INDIRECT_X_MODIFY: [fn(&mut Cpu); 16] = [
//     Cpu::mc_no_op,             Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,          Cpu::mc_read_addr_reg,
//     Cpu::mc_read_addr_reg,     Cpu::mc_wrapping_add_reg_lo,
//     Cpu::mc_read_addr_reg,     Cpu::mc_inc_addr_reg,
//     Cpu::mc_read_addr_reg_hi,  Cpu::mc_no_op,
//     Cpu::mc_read,              Cpu::mc_no_op,
//     Cpu::mc_write,             Cpu::mc_op_sentinel,
//     Cpu::mc_write,             Cpu::mc_no_op,
// ];

// const MC_INDIRECT_INDEXED_Y_READ: [fn(&mut Cpu); 12] = [
//     Cpu::mc_no_op,             Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,          Cpu::mc_read_addr_reg,
//     Cpu::mc_read_addr_reg,     Cpu::mc_inc_addr_reg,
//     Cpu::mc_read_addr_reg,     Cpu::mc_wrapping_add_reg_lo,
//     Cpu::mc_read_addr_reg_hi,  Cpu::mc_op_sentinel,
//     Cpu::mc_no_op,             Cpu::mc_no_op,  // ? Only if page wraparound
// ];

// const MC_INDIRECT_INDEXED_Y_WRITE: [fn(&mut Cpu); 12] = [
//     Cpu::mc_no_op,             Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,          Cpu::mc_read_addr_reg,
//     Cpu::mc_read_addr_reg,     Cpu::mc_inc_addr_reg,
//     Cpu::mc_read_addr_reg,     Cpu::mc_wrapping_add_reg_lo,
//     Cpu::mc_read_addr_reg_hi,  Cpu::mc_no_op,
//     Cpu::mc_op_sentinel,       Cpu::mc_write,
// ];

// const MC_INDIRECT_INDEXED_Y_MODIFY: [fn(&mut Cpu); 16] = [
//     Cpu::mc_no_op,             Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,          Cpu::mc_read_addr_reg,
//     Cpu::mc_read_addr_reg,     Cpu::mc_inc_addr_reg,
//     Cpu::mc_read_addr_reg,     Cpu::mc_wrapping_add_reg_lo,
//     Cpu::mc_read_addr_reg_hi,  Cpu::mc_no_op,
//     Cpu::mc_no_op,             Cpu::mc_no_op,  // ?
//     Cpu::mc_write,             Cpu::mc_no_op,
//     Cpu::mc_op_sentinel,       Cpu::mc_write,
// ];

// const MC_ABS_INDIRECT: [fn(&mut Cpu); 10] = [
//     Cpu::mc_no_op,            Cpu::mc_no_op,
//     Cpu::mc_fetch_pc,         Cpu::mc_addr_reg_lo,
//     Cpu::mc_fetch_pc,         Cpu::mc_addr_reg_hi,
//     Cpu::mc_read_addr_reg_lo, Cpu::mc_wrapping_add_reg_lo,
//     Cpu::mv_read_addr_reg_hi, Cpu::mc_op_sentinel,
// ];



// pub struct Cpu {
//     // Cpu registers and flags
//     pub pc: u16,
//     pub a: u8,
//     pub x: u8,
//     pub y: u8,
//     pub s: u8,

//     pub n: bool,
//     pub v: bool,
//     pub d: bool,
//     pub z: bool,
//     pub c: bool,
//     pub b: bool,
//     pub i: bool,

//     pub cpu_pins: Rc<RefCell<CpuPins>>,

//     // Instruction dispatch table
//     // dispatch: [fn(&mut Cpu, &mut Mem); 256],

//     microcode: [fn(&mut Cpu); 16],
//     microcode_index: 0,
//     microcode_lookup: [[fn(&mut Cpu); 16]; 256],
// }


// pub struct CpuMc {
//     // Cpu registers and flags
//     pub pc: u16,
//     pub a: u8,
//     pub x: u8,
//     pub y: u8,
//     pub s: u8,

//     pub n: bool,
//     pub v: bool,
//     pub d: bool,
//     pub z: bool,
//     pub c: bool,
//     pub b: bool,
//     pub i: bool,

//     pub cpu_pins: Rc<RefCell<CpuPins>>,

//     microcode: [fn(&mut Cpu); 16],
//     microcode_index: 0,
//     microcode_lookup: [[fn(&mut Cpu); 16]; 256],
// }

// impl CpuMc {
//     pub fn new(cpu_pins: Rc<RefCell<CpuPins>>) -> Cpu {
//         let mut new_cpu = Cpu {
//             pc: 0x0000,
//             a: 0x00,
//             x: 0x00,
//             y: 0x00,
//             s: 0xff,
//             n: false,
//             v: false,
//             d: false,
//             z: false,
//             c: false,
//             b: false,
//             i: false,
//         };

//         CpuMc::build_microcode_table();

//         new_cpu
//     }

//     fn build_microcode_table(&self) {
//         for opcode in 0..255 {
//             let (op, mode) = self.cpu_table.opcode_info[opcode];
//             let access_type = self.cpu_table.access_type(op);



//         }
//     }

//     fn microcode_sequence_for(op: Op, addr_mode: AddrMode, op_mode: OpMode) -> [fn(&mut Cpu); 16] {
//         let mut microcode_base_sequence = match (addr_mode, op_mode) {
//             (AddrMode::ABS, OpMode::READ) => MC_ABS_WRITE,
//             (AddrMode::ABS, OpMode::WRITE) => MC_ABS_WRITE,
//             (AddrMode::ABS, OpMode::MODIFY) => MC_ABS_MODIFY,
//             (AddrMode::ZP, OpMode::READ) => MC_ZP_READ,
//             (AddrMode::ZP, OpMode::WRITE) => MC_ZP_WRITE,
//             (AddrMode::ZP, OpMode::MODIFY) => MC_ZP_MODIFY,
//             (AddrMode::ZPX, OpMode::READ) => MC_ZP_XY_READ_WRITE,
//             (AddrMode::ZPY, OpMode::READ) => MC_ZP_XY_READ_WRITE,
//             (AddrMode::ZPX, OpMode::WRITE) => MC_ZP_XY_READ_WRITE,
//             (AddrMode::ZPY, OpMode::WRITE) => MC_ZP_XY_READ_WRITE,
//             (AddrMode::ZPX, OpMode::MODIFY) => MC_ZP_X_MODIFY,
//             (AddrMode::ABX, OpMode::READ) => MC_ABS_XY_READ,
//             (AddrMode::ABY, OpMode::READ) => MC_ABS_XY_READ,
//             (AddrMode::ABX, OpMode::WRITE) => MC_ABS_XY_WRITE,
//             (AddrMode::ABY, OpMode::WRITE) => MC_ABS_XY_READ,

//         }


//     }
// }

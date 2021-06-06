


#[derive(Clone, Copy, Debug)]
pub enum Op {
    ADC, AND, ASL, BCC, BCS, BEQ, BIT, BMI, BNE, BPL, BRK, BVC, BVS, CLC,
    CLD, CLI, CLV, CMP, CPX, CPY, DEC, DEX, DEY, EOR, HLT, INC, INX, INY, JMP,
    JSR, LDA, LDX, LDY, LSR, NOP, ORA, PHA, PHP, PLA, PLP, ROL, ROR, RTI,
    RTS, SBC, SEC, SED, SEI, SLO, STA, STX, STY, TAX, TAY, TSX, TXA, TXS, TYA,
    // Unofficial
    ALR, ANC, ARR, AHX, AXS, DCP, ISC, LAS, LAX, RLA, RRA, SAX, SHX, SHY, SRE, TAS, XAA,
}

#[derive(Clone, Copy, Debug)]
pub enum AddrMode {
    ABS, ABX, ABY, IMM, IMP, IND, IZX, IZY, REL, ZP, ZPA, ZPX, ZPY,
}

pub enum AccessType {
    CUSTOM, READ, WRITE, MODIFY,
}

pub struct CpuTable {
    pub opcode_info: [(Op, Mode); 256],
}

impl CpuTable {
    pub fn new() -> CpuTable {
        CpuTable {
            opcode_info: [
                // 0x00 - 0x0F
                (Op::BRK, Mode::IMP), (Op::ORA, Mode::IZX), (Op::HLT, Mode::IMP), (Op::SLO, Mode::IZX),
                (Op::NOP, Mode::ZP),  (Op::ORA, Mode::ZP),  (Op::ASL, Mode::ZP),  (Op::SLO, Mode::ZP),
                (Op::PHP, Mode::IMP), (Op::ORA, Mode::IMM), (Op::ASL, Mode::IMP), (Op::ANC, Mode::IMM),
                (Op::NOP, Mode::ABS), (Op::ORA, Mode::ABS), (Op::ASL, Mode::ABS), (Op::SLO, Mode::ABS),

                // 0x10 - 0x1F
                (Op::BPL, Mode::REL), (Op::ORA, Mode::IZY), (Op::HLT, Mode::IMP), (Op::SLO, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::ORA, Mode::ZPX), (Op::ASL, Mode::ZPX), (Op::SLO, Mode::ZPX),
                (Op::CLC, Mode::IMP), (Op::ORA, Mode::ABY), (Op::NOP, Mode::IMP), (Op::SLO, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::ORA, Mode::ABX), (Op::ASL, Mode::ABX), (Op::SLO, Mode::ABX),

                // 0x20 - 0x2F
                (Op::JSR, Mode::ABS), (Op::AND, Mode::IZX), (Op::HLT, Mode::IMP), (Op::RLA, Mode::IZX),
                (Op::BIT, Mode::ZP),  (Op::AND, Mode::ZP),  (Op::ROL, Mode::ZP),  (Op::RLA, Mode::ZP),
                (Op::PLP, Mode::IMP), (Op::AND, Mode::IMM), (Op::ROL, Mode::IMP), (Op::ANC, Mode::IMM),
                (Op::BIT, Mode::ABS), (Op::AND, Mode::ABS), (Op::ROL, Mode::ABS), (Op::RLA, Mode::ABS),

                // 0x30 - 0x3F
                (Op::BMI, Mode::REL), (Op::AND, Mode::IZY), (Op::HLT, Mode::IMP), (Op::RLA, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::AND, Mode::ZPX), (Op::ROL, Mode::ZPX), (Op::RLA, Mode::ZPX),
                (Op::SEC, Mode::IMP), (Op::AND, Mode::ABY), (Op::NOP, Mode::IMP), (Op::RLA, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::AND, Mode::ABX), (Op::ROL, Mode::ABX), (Op::RLA, Mode::ABX),

                // 0x40 - 0x4F
                (Op::RTI, Mode::IMP), (Op::EOR, Mode::IZX), (Op::HLT, Mode::IMP), (Op::SRE, Mode::IZX),
                (Op::NOP, Mode::ZP),  (Op::EOR, Mode::ZP),  (Op::LSR, Mode::ZP),  (Op::SRE, Mode::ZP),
                (Op::PHA, Mode::IMP), (Op::EOR, Mode::IMM), (Op::LSR, Mode::IMP), (Op::ALR, Mode::IMM),
                (Op::JMP, Mode::ABS), (Op::EOR, Mode::ABS), (Op::LSR, Mode::ABS), (Op::SRE, Mode::ABS),

                // 0x50 - 0x5F
                (Op::BVC, Mode::REL), (Op::EOR, Mode::IZY), (Op::HLT, Mode::IMP), (Op::SRE, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::EOR, Mode::ZPX), (Op::LSR, Mode::ZPX), (Op::SRE, Mode::ZPX),
                (Op::CLI, Mode::IMP), (Op::EOR, Mode::ABY), (Op::NOP, Mode::IMP), (Op::SRE, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::EOR, Mode::ABX), (Op::LSR, Mode::ABX), (Op::SRE, Mode::ABX),

                // 0x60 - 0x6F
                (Op::RTS, Mode::IMP), (Op::ADC, Mode::IZX), (Op::HLT, Mode::IMP), (Op::RRA, Mode::IZX),
                (Op::NOP, Mode::ZP),  (Op::ADC, Mode::ZP),  (Op::ROR, Mode::ZP),  (Op::RRA, Mode::ZP),
                (Op::PLA, Mode::IMP), (Op::ADC, Mode::IMM), (Op::ROR, Mode::IMP), (Op::ARR, Mode::IMM),
                (Op::JMP, Mode::IND), (Op::ADC, Mode::ABS), (Op::ROR, Mode::ABS), (Op::RRA, Mode::ABS),

                // 0x70 - 0x7F
                (Op::BVS, Mode::REL), (Op::ADC, Mode::IZY), (Op::HLT, Mode::IMP), (Op::RRA, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::ADC, Mode::ZPX), (Op::ROR, Mode::ZPX), (Op::RRA, Mode::ZPX),
                (Op::SEI, Mode::IMP), (Op::ADC, Mode::ABY), (Op::NOP, Mode::IMP), (Op::RRA, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::ADC, Mode::ABX), (Op::ROR, Mode::ABX), (Op::RRA, Mode::ABX),

                // 0x80 - 0x8F
                (Op::NOP, Mode::IMM), (Op::STA, Mode::IZX), (Op::NOP, Mode::IMM), (Op::SAX, Mode::IZX),
                (Op::STY, Mode::ZP),  (Op::STA, Mode::ZP),  (Op::STX, Mode::ZP),  (Op::SAX, Mode::ZP),
                (Op::DEY, Mode::IMP), (Op::NOP, Mode::IMM), (Op::TXA, Mode::IMP), (Op::XAA, Mode::IMM),
                (Op::STY, Mode::ABS), (Op::STA, Mode::ABS), (Op::STX, Mode::ABS), (Op::SAX, Mode::ABS),

                // 0x90 - 0x9F
                (Op::BCC, Mode::REL), (Op::STA, Mode::IZY), (Op::HLT, Mode::IMP), (Op::AHX, Mode::IZY),
                (Op::STY, Mode::ZPX), (Op::STA, Mode::ZPX), (Op::STX, Mode::ZPY), (Op::SAX, Mode::ZPY),
                (Op::TYA, Mode::IMP), (Op::STA, Mode::ABY), (Op::TXS, Mode::IMP), (Op::TAS, Mode::ABY),
                (Op::SHY, Mode::ABX), (Op::STA, Mode::ABX), (Op::SHX, Mode::ABY), (Op::AHX, Mode::ABY),

                // 0xA0 - 0xAF
                (Op::LDY, Mode::IMM), (Op::LDA, Mode::IZX), (Op::LDX, Mode::IMM), (Op::LAX, Mode::IZX),
                (Op::LDY, Mode::ZP),  (Op::LDA, Mode::ZP),  (Op::LDX, Mode::ZP),  (Op::LAX, Mode::ZP),
                (Op::TAY, Mode::IMP), (Op::LDA, Mode::IMM), (Op::TAX, Mode::IMP), (Op::LAX, Mode::IMM),
                (Op::LDY, Mode::ABS), (Op::LDA, Mode::ABS), (Op::LDX, Mode::ABS), (Op::LAX, Mode::ABS),

                // 0xB0 - 0xBF
                (Op::BCS, Mode::REL), (Op::LDA, Mode::IZY), (Op::HLT, Mode::IMP), (Op::LAX, Mode::IZY),
                (Op::LDY, Mode::ZPX), (Op::LDA, Mode::ZPX), (Op::LDX, Mode::ZPY), (Op::LAX, Mode::ZPY),
                (Op::CLV, Mode::IMP), (Op::LDA, Mode::ABY), (Op::TSX, Mode::IMP), (Op::LAS, Mode::ABY),
                (Op::LDY, Mode::ABX), (Op::LDA, Mode::ABX), (Op::LDX, Mode::ABX), (Op::LAX, Mode::ABX),

                // 0xC0 - 0xCF
                (Op::CPY, Mode::IMM), (Op::CMP, Mode::IZX), (Op::NOP, Mode::IMM), (Op::DCP, Mode::IZX),
                (Op::CPY, Mode::ZP),  (Op::CMP, Mode::ZP),  (Op::DEC, Mode::ZP),  (Op::DCP, Mode::ZP),
                (Op::INY, Mode::IMP), (Op::CMP, Mode::IMM), (Op::DEX, Mode::IMP), (Op::AXS, Mode::IMM),
                (Op::CPY, Mode::ABS), (Op::CMP, Mode::ABS), (Op::DEC, Mode::ABS), (Op::DCP, Mode::ABS),

                // 0xD0 - 0xDF
                (Op::BNE, Mode::REL), (Op::CMP, Mode::IZY), (Op::HLT, Mode::IMP), (Op::DCP, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::CMP, Mode::ZPX), (Op::DEC, Mode::ZPX), (Op::DCP, Mode::ZPX),
                (Op::CLD, Mode::IMP), (Op::CMP, Mode::ABY), (Op::NOP, Mode::IMP), (Op::DCP, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::CMP, Mode::ABX), (Op::DEC, Mode::ABX), (Op::DCP, Mode::ABX),

                // 0xE0 - 0xEF
                (Op::CPX, Mode::IMM), (Op::SBC, Mode::IZX), (Op::NOP, Mode::IMP), (Op::ISC, Mode::IZX),
                (Op::CPX, Mode::ZP),  (Op::SBC, Mode::ZP),  (Op::INC, Mode::ZP),  (Op::ISC, Mode::ZP),
                (Op::INX, Mode::IMP), (Op::SBC, Mode::IMM), (Op::NOP, Mode::IMP), (Op::SBC, Mode::IMM),
                (Op::CPX, Mode::ABS), (Op::SBC, Mode::ABS), (Op::INC, Mode::ABS), (Op::ISC, Mode::ABS),

                // 0xF0 - 0xFF
                (Op::BEQ, Mode::REL), (Op::SBC, Mode::IZY), (Op::HLT, Mode::IMP), (Op::ISC, Mode::IZY),
                (Op::NOP, Mode::ZPX), (Op::SBC, Mode::ZPX), (Op::INC, Mode::ZPX), (Op::ISC, Mode::ZPX),
                (Op::SED, Mode::IMP), (Op::SBC, Mode::ABY), (Op::NOP, Mode::IMP), (Op::ISC, Mode::ABY),
                (Op::NOP, Mode::ABX), (Op::SBC, Mode::ABX), (Op::INC, Mode::ABX), (Op::ISC, Mode::ABX),
            ],
        }
    }

    pub fn access_type(&self, op: Op) => AccessType {
        match op {
            Op.LDA | Op.LDX | Op.LDY | Op.EOR | Op.AND | Op.ORA |
            Op.ADC | Op.SBC | Op.CMP | Op.BIT | Op.LAX | Op.LAE |
            Op.SHS | Op.NOP
            => AccessType.READ,

            Op.STA | Op.STX | Op.STY | Op.SHA | Op.SHX | Op.SHY
            => AccessType.WRITE,

            Op.ASL | Op.LSR | Op.ROL | Op.ROR | Op.INC | Op.DEC |
            Op.SLO | Op.SRE | Op.RLA | Op.RRA | Op.ISB | Op.DCP
            => AccessType.MODIFY,

            _ => AccessType.CUSTOM,
        }
    }
}


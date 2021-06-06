
pub struct CpuPins {
    // pub address_bus: u16,
    // pub data_bus: u8,

    // pub clock_0_in: bool,
    // pub clock_1_out: bool,
    // pub clock_2_out: bool,
    pub irq: bool,
    pub nmi: bool,
    pub rdy: bool,
    // pub res: bool,
    // pub rw: bool,
    // pub so: bool,
    // pub sync: bool,

    pub halt: bool,  // Atari 8-bit ("Sally") only
}

impl CpuPins {
    fn new() -> CpuPins {
        CpuPins {
            // address_bus: 0,
            // data_bus: 0,

            irq: false,
            nmi: false,
            rdy: false,
            // res: false,
            // sync: false,
            halt: false,
        }
    }
}

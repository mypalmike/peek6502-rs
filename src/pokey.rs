/// POKEY - Potentiometer Keyboard Integrated Circuit
/// Handles sound generation, keyboard input, serial I/O, and timers.
///
/// Memory map: $D200-$D2FF
pub struct Pokey {
    // Audio frequency registers
    audf: [u8; 4],      // $D200, $D202, $D204, $D206

    // Audio control registers
    audc: [u8; 4],      // $D201, $D203, $D205, $D207

    // Control registers
    audctl: u8,         // $D208 - Audio control
    stimer: u8,         // $D209 - Start timers
    skrest: u8,         // $D20A - Reset serial port status
    potgo: u8,          // $D20B - Start paddle scan
    serout: u8,         // $D20D - Serial port output
    irqen: u8,          // $D20E - Interrupt enable
    skctl: u8,          // $D20F - Serial port control

    // Status/Input registers (read-only)
    pot: [u8; 8],       // $D200-$D207 - Paddle controllers (alternate read)
    allpot: u8,         // $D208 - Pot port status
    kbcode: u8,         // $D209 - Keyboard code
    random: u8,         // $D20A - Random number
    serin: u8,          // $D20D - Serial port input
    irqst: u8,          // $D20E - IRQ status
    skstat: u8,         // $D20F - Serial port status

    // Internal state
    timers: [u16; 4],   // Internal timer counters
    random_seed: u8,    // For random number generation
}

impl Pokey {
    pub fn new() -> Pokey {
        Pokey {
            audf: [0; 4],
            audc: [0; 4],
            audctl: 0,
            stimer: 0,
            skrest: 0,
            potgo: 0,
            serout: 0,
            irqen: 0,
            skctl: 0,
            pot: [0xFF; 8],
            allpot: 0,
            kbcode: 0xFF,
            random: 0,
            serin: 0,
            irqst: 0,
            skstat: 0,
            timers: [0; 4],
            random_seed: 0xFF,
        }
    }

    /// Execute one machine cycle of POKEY operation
    pub fn tick(&mut self) {
        // Update timers
        for i in 0..4 {
            if self.timers[i] > 0 {
                self.timers[i] -= 1;
            } else {
                // Timer expired, reload from frequency register
                self.timers[i] = self.audf[i] as u16;
                // TODO: Generate audio sample, trigger IRQ if enabled
            }
        }

        // Update random number generator (simple LFSR)
        self.random_seed = ((self.random_seed << 1) | ((self.random_seed >> 7) ^ (self.random_seed >> 5) & 1)) & 0xFF;
        self.random = self.random_seed;

        // TODO: Handle keyboard scanning, serial I/O, etc.
    }

    /// Read from a POKEY register
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr & 0x0F {
            0x00 => self.pot[0],
            0x01 => self.pot[1],
            0x02 => self.pot[2],
            0x03 => self.pot[3],
            0x04 => self.pot[4],
            0x05 => self.pot[5],
            0x06 => self.pot[6],
            0x07 => self.pot[7],
            0x08 => self.allpot,
            0x09 => self.kbcode,
            0x0A => self.random,
            0x0D => self.serin,
            0x0E => self.irqst,
            0x0F => self.skstat,
            _ => 0xFF,
        }
    }

    /// Write to a POKEY register
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr & 0x0F {
            0x00 => self.audf[0] = val,
            0x01 => self.audc[0] = val,
            0x02 => self.audf[1] = val,
            0x03 => self.audc[1] = val,
            0x04 => self.audf[2] = val,
            0x05 => self.audc[2] = val,
            0x06 => self.audf[3] = val,
            0x07 => self.audc[3] = val,
            0x08 => self.audctl = val,
            0x09 => {
                self.stimer = val;
                // Reset all timers
                for i in 0..4 {
                    self.timers[i] = self.audf[i] as u16;
                }
            }
            0x0A => self.skrest = val,
            0x0B => self.potgo = val,
            0x0D => self.serout = val,
            0x0E => self.irqen = val,
            0x0F => self.skctl = val,
            _ => {}
        }
    }
}

/// PIA - Peripheral Interface Adapter (6520)
/// Handles joystick input, console switches, and OS ROM banking.
///
/// Memory map: $D300-$D3FF
pub struct Pia {
    // Port A - Joystick ports 1 & 2, console switches
    porta: u8,          // $D300 - Port A data
    ddra: u8,           // $D301 - Port A data direction (0=input, 1=output)

    // Port B - Joystick ports 3 & 4, OS ROM control
    portb: u8,          // $D302 - Port B data
    ddrb: u8,           // $D303 - Port B data direction

    // Input state (what's actually on the pins)
    porta_input: u8,
    portb_input: u8,
}

impl Pia {
    pub fn new() -> Pia {
        Pia {
            porta: 0xFF,
            ddra: 0,
            portb: 0xFF,
            ddrb: 0,
            porta_input: 0xFF,  // No joystick input
            portb_input: 0xFF,  // No joystick input
        }
    }

    /// Execute one machine cycle of PIA operation
    pub fn tick(&mut self) {
        // PIA is mostly passive, responding to reads/writes
        // Could add joystick scanning logic here in the future
    }

    /// Read from a PIA register
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr & 0x03 {
            0x00 => {
                // PORTA - bits set as input (0 in DDRA) read from porta_input
                // bits set as output (1 in DDRA) read from porta
                (self.porta & self.ddra) | (self.porta_input & !self.ddra)
            }
            0x01 => self.ddra,
            0x02 => {
                // PORTB - same logic as PORTA
                (self.portb & self.ddrb) | (self.portb_input & !self.ddrb)
            }
            0x03 => self.ddrb,
            _ => 0xFF,
        }
    }

    /// Write to a PIA register
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr & 0x03 {
            0x00 => self.porta = val,
            0x01 => self.ddra = val,
            0x02 => self.portb = val,
            0x03 => self.ddrb = val,
            _ => {}
        }
    }

    /// Set joystick/console input state (called by emulator input handling)
    pub fn set_porta_input(&mut self, val: u8) {
        self.porta_input = val;
    }

    pub fn set_portb_input(&mut self, val: u8) {
        self.portb_input = val;
    }
}

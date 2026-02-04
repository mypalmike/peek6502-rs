/// PIA - Peripheral Interface Adapter (6520)
/// Handles joystick input, console switches, and OS ROM banking.
///
/// Memory map: $D300-$D3FF
///
/// The 6520 PIA has 4 registers selected by RS1 (addr bit 1) and RS0 (addr bit 0):
///   $D300 (RS1=0, RS0=0): PORTA data or DDRA (selected by PACTL bit 2)
///   $D301 (RS1=0, RS0=1): PACTL (Port A control register)
///   $D302 (RS1=1, RS0=0): PORTB data or DDRB (selected by PBCTL bit 2)
///   $D303 (RS1=1, RS0=1): PBCTL (Port B control register)
pub struct Pia {
    // Port A - Joystick ports 1 & 2
    porta: u8,          // Port A output register
    ddra: u8,           // Port A data direction (0=input, 1=output)
    pactl: u8,          // Port A control register

    // Port B - Joystick ports 3 & 4, console switches
    portb: u8,          // Port B output register
    ddrb: u8,           // Port B data direction
    pbctl: u8,          // Port B control register

    // Input state (what's actually on the pins)
    porta_input: u8,
    portb_input: u8,
}

impl Pia {
    pub fn new() -> Pia {
        Pia {
            porta: 0xFF,
            ddra: 0,
            pactl: 0,           // Bit 2 = 0: $D300 accesses DDRA initially
            portb: 0x00,        // PORTB will be set by machine-specific initialization
            ddrb: 0,
            pbctl: 0,           // Bit 2 = 0: $D302 accesses DDRB initially
            porta_input: 0xFF,  // No joystick input (active low)
            portb_input: 0xFF,  // No console keys pressed (active low), no special boot mode
        }
    }

    /// Execute one machine cycle of PIA operation
    pub fn tick(&mut self) {
        // PIA is mostly passive, responding to reads/writes
    }

    /// Read from a PIA register
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr & 0x03 {
            0x00 => {
                if (self.pactl & 0x04) != 0 {
                    // PACTL bit 2 set: read PORTA data register
                    // Input pins (DDR=0) read from external input, output pins (DDR=1) read latch
                    (self.porta & self.ddra) | (self.porta_input & !self.ddra)
                } else {
                    // PACTL bit 2 clear: read DDRA
                    self.ddra
                }
            }
            0x01 => {
                // $D301: Read PORTB data register (XL memory control)
                if (self.pbctl & 0x04) != 0 {
                    // PBCTL bit 2 set: read PORTB data register
                    (self.portb & self.ddrb) | (self.portb_input & !self.ddrb)
                } else {
                    // PBCTL bit 2 clear: read DDRB
                    self.ddrb
                }
            }
            0x02 => {
                // $D302: Read PACTL (bits 7,6 are IRQ flags, read-only; cleared on read)
                self.pactl
            }
            0x03 => {
                // Read PBCTL
                self.pbctl
            }
            _ => 0xFF,
        }
    }

    /// Write to a PIA register
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr & 0x03 {
            0x00 => {
                if (self.pactl & 0x04) != 0 {
                    // PACTL bit 2 set: write PORTA data register
                    self.porta = val;
                } else {
                    // PACTL bit 2 clear: write DDRA
                    self.ddra = val;
                }
            }
            0x01 => {
                // $D301: Write PORTB (XL memory control: bit 0=OS ROM, bit 1=BASIC ROM)
                if (self.pbctl & 0x04) != 0 {
                    // PBCTL bit 2 set: write PORTB data register
                    self.portb = val;
                } else {
                    // PBCTL bit 2 clear: write DDRB
                    self.ddrb = val;
                }
            }
            0x02 => {
                // $D302: Write PACTL (bits 7,6 are read-only IRQ flags)
                self.pactl = (self.pactl & 0xC0) | (val & 0x3F);
            }
            0x03 => {
                // Write PBCTL (bits 7,6 are read-only IRQ flags)
                self.pbctl = (self.pbctl & 0xC0) | (val & 0x3F);
            }
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

    /// Check if PIA is asserting the NMI line
    pub fn is_nmi_asserted(&self) -> bool {
        false
    }

    /// Get the actual PORTB data register value (for banking control)
    pub fn get_portb(&self) -> u8 {
        self.portb
    }

    /// Set PORTB data register (for machine-specific initialization)
    pub fn set_portb(&mut self, value: u8) {
        self.portb = value;
    }
}

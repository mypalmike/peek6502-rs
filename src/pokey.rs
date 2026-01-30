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

    // Serial output state
    serial_out_timer: u16,    // Countdown for serial output completion
    serial_out_active: bool,  // Whether serial output is in progress

    // Keyboard state
    last_key_code: u8,  // Last key code pressed (for change detection)
    shift_pressed: bool, // Track shift key state
    ctrl_pressed: bool,  // Track control key state
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
            allpot: 0xFF,  // All pot lines high
            kbcode: 0xFF,  // No key pressed
            random: 0,
            serin: 0,
            irqst: 0xFF,   // No IRQs pending (bits high = no IRQ)
            skstat: 0xFF,  // All status bits high = keyboard ready, no errors
            timers: [0; 4],
            random_seed: 0xFF,
            serial_out_timer: 0,
            serial_out_active: false,
            last_key_code: 0xFF, // No key pressed initially
            shift_pressed: false,
            ctrl_pressed: false,
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
                // Timer underflow IRQ: timer 0→bit 0, timer 1→bit 1, timer 3→bit 2
                let irq_bit = match i {
                    0 => Some(0),
                    1 => Some(1),
                    3 => Some(2),
                    _ => None,
                };
                if let Some(bit) = irq_bit {
                    let mask = 1 << bit;
                    if (self.irqen & mask) != 0 {
                        self.irqst &= !mask;
                    }
                }
            }
        }

        // Serial output timer
        if self.serial_out_active {
            if self.serial_out_timer > 0 {
                self.serial_out_timer -= 1;
            } else {
                self.serial_out_active = false;
                // Serial output transmission complete - clear IRQST bit 3
                if (self.irqen & 0x08) != 0 {
                    self.irqst &= !0x08;
                }
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
            0x0D => {
                self.serout = val;
                // Immediate: clear "output data needed" IRQ (bit 4)
                if (self.irqen & 0x10) != 0 {
                    self.irqst &= !0x10;
                }
                self.serial_out_active = true;
                self.serial_out_timer = 1000; // ~19200 baud approximation
            }
            0x0E => self.irqen = val,
            0x0F => self.skctl = val,
            _ => {}
        }
    }

    /// Handle a key press event
    /// Returns true if IRQ line should be asserted
    pub fn key_press(&mut self, atari_key_code: u8, shift: bool, ctrl: bool) -> bool {
        // Only trigger interrupt on actual key code change
        // (ignore if only shift/ctrl modifiers changed)
        let key_code_changed = (atari_key_code & 0x3F) != (self.last_key_code & 0x3F);

        if key_code_changed {
            self.last_key_code = atari_key_code;
            self.kbcode = atari_key_code;

            // Clear SKSTAT bit 2 (key ready, 0 = key available)
            self.skstat &= !0x04;

            // Update keyboard IRQ if enabled
            if (self.irqen & 0x40) != 0 {
                // Check if IRQ is ready (bit 6 of IRQST set)
                if (self.irqst & 0x40) != 0 {
                    // Clear IRQST bit 6 (keyboard IRQ)
                    self.irqst &= !0x40;
                    return true;  // Assert IRQ line
                } else {
                    // IRQ overflow - keyboard IRQ already pending
                    // Set SKSTAT bit 6 (keyboard overrun)
                    self.skstat &= !0x40;
                }
            }
        }

        // Update shift/ctrl state in SKSTAT
        self.shift_pressed = shift;
        self.ctrl_pressed = ctrl;

        // Update SKSTAT bit 3 (shift key status, 0 = pressed, 1 = not pressed)
        if shift {
            self.skstat &= !0x08;
        } else {
            self.skstat |= 0x08;
        }

        false  // Don't assert IRQ
    }

    /// Check if any enabled POKEY interrupt is active
    /// Returns true if IRQ line should be asserted
    pub fn irq_active(&self) -> bool {
        // For each interrupt source, check if enabled (IRQEN) and active (IRQST low)
        // IRQST bits are inverted: 0 = IRQ active, 1 = no IRQ
        // IRQEN bits: 1 = enabled, 0 = disabled

        // Check each interrupt source:
        // Bit 0: Timer 1 IRQ
        // Bit 1: Timer 2 IRQ
        // Bit 2: Timer 4 IRQ
        // Bit 3: Serial output transmission complete
        // Bit 4: Serial output data needed
        // Bit 5: Serial input data ready
        // Bit 6: Keyboard IRQ
        // Bit 7: Break key IRQ

        for bit in 0..8 {
            let mask = 1 << bit;
            let enabled = (self.irqen & mask) != 0;
            let active = (self.irqst & mask) == 0;  // IRQST is inverted

            if enabled && active {
                return true;
            }
        }

        false
    }

    /// Write to IRQEN register and return new IRQ line state
    /// Per Altirra hardware docs: "the status bit for a disabled interrupt is always locked to a 1"
    /// This means when IRQEN bits are cleared, corresponding IRQST bits are set (interrupt acknowledged)
    pub fn write_irqen(&mut self, val: u8) -> bool {
        // For each bit that is being disabled (was 1, now 0), set corresponding IRQST bit to 1
        let newly_disabled = self.irqen & !val;  // Bits that were enabled, now disabled
        self.irqst |= newly_disabled;  // Set those IRQST bits to 1 (inactive)

        self.irqen = val;
        self.irq_active()
    }

    /// Handle key release event
    /// Returns true if IRQ line should be asserted
    pub fn key_release(&mut self) -> bool {
        self.kbcode = 0xFF;  // No key pressed
        self.skstat |= 0x04;  // Set bit 2 (key not ready, 1 = no key)
        self.last_key_code = 0xFF;
        self.irq_active()  // Return current IRQ state
    }
}

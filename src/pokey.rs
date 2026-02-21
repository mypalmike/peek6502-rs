use crate::pokey_audio::PokeyAudio;
use crate::audio_buffer::AudioBuffer;

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

    // Audio synthesis engine and per-frame output buffer
    pub audio: PokeyAudio,
    pub audio_buffer: AudioBuffer,

    // Internal state
    timers: [u16; 4],   // Internal timer counters
    random_seed: u8,    // For random number generation

    // Serial input state
    serin_data_ready: bool, // Byte in SERIN waiting to trigger IRQ when enabled

    // Serial output state
    // Two-stage model: SEROUT register -> shift register
    serout_pending: bool,      // Byte waiting in SEROUT to be loaded to shift register
    serout_load_timer: u16,    // Cycles until SEROUT is loaded to shift register
    shift_out_active: bool,    // Shift register is transmitting
    shift_out_timer: u16,      // Cycles until shift register completes

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
            serin_data_ready: false,
            serout_pending: false,
            serout_load_timer: 0,
            shift_out_active: false,
            shift_out_timer: 0,
            last_key_code: 0xFF, // No key pressed initially
            shift_pressed: false,
            ctrl_pressed: false,
            // NTSC Atari 800: 1.79 MHz CPU, 44100 Hz audio output
            audio: PokeyAudio::new(1_789_790, 44100),
            audio_buffer: AudioBuffer::new(),
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

        // Serial output: two-stage model (SEROUT register -> shift register)
        //
        // Stage 1: SEROUT load timer - delay before byte loads into shift register
        if self.serout_pending && self.serout_load_timer > 0 {
            self.serout_load_timer -= 1;
            if self.serout_load_timer == 0 {
                // Load byte from SEROUT to shift register
                self.serout_pending = false;
                self.shift_out_active = true;
                self.shift_out_timer = 930;  // ~10 bit cells at 19200 baud

                // Assert SEROUT_NEED IRQ (bit 4) - SEROUT is now ready for next byte
                if (self.irqen & 0x10) != 0 {
                    self.irqst &= !0x10;
                }
            }
        }

        // Stage 2: Shift register transmitting
        if self.shift_out_active {
            if self.shift_out_timer > 0 {
                self.shift_out_timer -= 1;
            } else {
                // Shift register finished
                if self.serout_pending {
                    // Another byte waiting - load it immediately (back-to-back transmission)
                    self.serout_pending = false;
                    self.shift_out_timer = 930;
                    // Assert SEROUT_NEED IRQ (bit 4)
                    if (self.irqen & 0x10) != 0 {
                        self.irqst &= !0x10;
                    }
                } else {
                    // No more bytes - shift register goes idle
                    self.shift_out_active = false;
                }
            }
        }

        // SEROUT_DONE (bit 3) is level-sensitive: asserted when shift register is idle
        // Unlike other IRQs, it's not latched - directly reflects shift register state
        if !self.shift_out_active && (self.irqen & 0x08) != 0 {
            self.irqst &= !0x08;  // Assert SEROUT_DONE
        } else {
            self.irqst |= 0x08;   // Deassert SEROUT_DONE (shift register busy)
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
            0x0D => {
                // Reading SERIN has NO side effects per hardware docs
                // It does NOT clear the serial input IRQ
                // The IRQ is cleared when the next byte STARTS arriving
                self.serin
            }
            0x0E => self.irqst,
            0x0F => self.skstat,
            _ => 0xFF,
        }
    }

    /// Write to a POKEY register
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr & 0x0F {
            0x00 => { self.audf[0] = val; self.audio.update_register(0x00, val); }
            0x01 => { self.audc[0] = val; self.audio.update_register(0x01, val); }
            0x02 => { self.audf[1] = val; self.audio.update_register(0x02, val); }
            0x03 => { self.audc[1] = val; self.audio.update_register(0x03, val); }
            0x04 => { self.audf[2] = val; self.audio.update_register(0x04, val); }
            0x05 => { self.audc[2] = val; self.audio.update_register(0x05, val); }
            0x06 => { self.audf[3] = val; self.audio.update_register(0x06, val); }
            0x07 => { self.audc[3] = val; self.audio.update_register(0x07, val); }
            0x08 => { self.audctl = val; self.audio.update_register(0x08, val); }
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
                // Write to SEROUT register
                // Per Altirra docs: only one byte can be pending in SEROUT
                // If a second byte is written before the first loads, it replaces it
                self.serout = val;
                self.serout_pending = true;

                // Start the load timer (~1 bit cell = ~93 cycles at 19200 baud)
                // This is the delay before byte loads from SEROUT to shift register
                if !self.shift_out_active {
                    // Shift register idle - start load timer
                    self.serout_load_timer = 93;
                }
                // If shift register is active, byte will be loaded when current byte completes

                // Clear SEROUT_NEED IRQ (bit 4 = 1) since we now have data pending
                self.irqst |= 0x10;
            }
            0x0E => self.irqen = val,
            0x0F => self.skctl = val,
            _ => {}
        }
    }

    /// Handle a key press event
    /// Returns true if IRQ line should be asserted
    pub fn key_press(&mut self, atari_key_code: u8, shift: bool, ctrl: bool) -> bool {
        // Trigger interrupt on key code change
        let key_code_changed = atari_key_code != self.last_key_code;

        // Combine base key code with modifier bits
        let full_key_code = atari_key_code
            | if shift { 0x40 } else { 0 }
            | if ctrl { 0x80 } else { 0 };

        if key_code_changed {
            self.last_key_code = atari_key_code;
            self.kbcode = full_key_code;

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

        // Check if enabling serial input IRQ with data ready
        // This handles the case where a byte arrived before the IRQ was enabled
        self.try_trigger_serin_irq();

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

    /// Trigger Break key IRQ (bit 7)
    /// Returns true if IRQ line should be asserted
    pub fn break_key_press(&mut self) -> bool {
        // Update Break key IRQ if enabled (bit 7 of IRQEN)
        if (self.irqen & 0x80) != 0 {
            // Check if IRQ is ready (bit 7 of IRQST set)
            if (self.irqst & 0x80) != 0 {
                // Clear IRQST bit 7 (Break key IRQ)
                self.irqst &= !0x80;
                return true;  // Assert IRQ line
            }
        }
        false  // Don't assert IRQ
    }

    // ========================================================================
    // Serial I/O Support (for SIO integration)
    // ========================================================================

    /// Queue a byte into SERIN register (from SIO device response)
    ///
    /// This simulates the complete reception of a byte from the SIO bus:
    /// 1. The byte is loaded into SERIN
    /// 2. serin_data_ready flag is set (byte waiting for IRQ)
    /// 3. If serial input IRQ is enabled, trigger it immediately
    /// 4. If not enabled, the IRQ will trigger when OS enables it later
    ///
    /// Per Altirra Hardware Manual:
    /// - Reading SERIN has NO side effects (doesn't clear IRQ)
    /// - The IRQ is cleared when the NEXT byte starts arriving (shift register active)
    pub fn queue_serin_byte(&mut self, byte: u8) {
        // Clear any previous pending IRQ (new byte arrival clears old)
        self.irqst |= 0x20;  // Set bit 5 to 1 = no IRQ

        // Load the new byte
        self.serin = byte;
        self.serin_data_ready = true;

        // Try to trigger IRQ (will only work if IRQEN bit 5 is set)
        self.try_trigger_serin_irq();
    }

    /// Try to trigger SERIN IRQ if data is ready and IRQ is enabled
    fn try_trigger_serin_irq(&mut self) {
        if self.serin_data_ready && (self.irqen & 0x20) != 0 {
            self.irqst &= !0x20;  // Clear bit 5 to trigger IRQ (active low)
            self.serin_data_ready = false;  // IRQ consumed
        }
    }

    /// Get the current SEROUT byte (for SIO controller to read)
    /// This is called when the OS writes a byte to SEROUT that needs to be sent to SIO
    pub fn get_serout_byte(&self) -> u8 {
        self.serout
    }

    // ========================================================================
    // Audio Generation
    // ========================================================================

    /// Generate audio for `ticks` CPU cycles, appending samples to the internal buffer.
    /// Called once per scanline (ticks=114) from Atari800::tick_scanline().
    pub fn generate_audio(&mut self, ticks: u32) {
        self.audio.generate(ticks, &mut self.audio_buffer);
    }
}

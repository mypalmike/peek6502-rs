use crate::mem::Mem;

/// ANTIC - Alphanumeric Television Interface Controller
/// Handles display list processing, DMA, and video timing for Atari 8-bit computers.
///
/// Memory map: $D400-$D4FF
pub struct Antic {
    // Display list pointer
    dlist_ptr: u16,
    dlist_index: u16,  // Current position in display list

    // Screen memory pointer (for current mode line)
    screen_ptr: u16,

    // DMA control
    dma_enabled: bool,

    // Scanline tracking
    scanline: u16,       // Current scanline (0-261 for NTSC)
    horizontal_pos: u8,  // Horizontal position within scanline

    // Display list state
    current_mode: u8,    // Current ANTIC mode being displayed
    mode_line: u8,       // Which line within the current mode (0-7 for text modes)
    lines_remaining: u8, // How many more lines of this mode instruction

    // ANTIC registers
    dmactl: u8,     // $D400 - DMA control
    chactl: u8,     // $D401 - Character mode control
    dlistl: u8,     // $D402 - Display list pointer low
    dlisth: u8,     // $D403 - Display list pointer high
    hscrol: u8,     // $D404 - Horizontal scroll
    vscrol: u8,     // $D405 - Vertical scroll
    pmbase: u8,     // $D407 - Player/missile base address
    chbase: u8,     // $D409 - Character set base address
    wsync: u8,      // $D40A - Wait for horizontal sync
    vcount: u8,     // $D40B - Vertical line counter
    penh: u8,       // $D40C - Light pen horizontal position
    penv: u8,       // $D40D - Light pen vertical position
    nmien: u8,      // $D40E - NMI enable
    nmires: u8,     // $D40F - NMI reset/status

    // NMI line state
    nmi_asserted: bool,  // true = ANTIC is asserting NMI line

    // Cycle tracking for scanline advancement
    cycle_accumulator: u32,  // Accumulated CPU cycles since last scanline advance

    // Scanline buffer (pixels to be displayed)
    pub scanline_buffer: [u8; 384],  // Color indices for current scanline
}

impl Antic {
    pub fn new() -> Antic {
        Antic {
            dlist_ptr: 0,
            dlist_index: 0,
            screen_ptr: 0,
            dma_enabled: false,
            scanline: 0,
            horizontal_pos: 0,
            current_mode: 0,
            mode_line: 0,
            lines_remaining: 0,
            dmactl: 0,
            chactl: 0,
            dlistl: 0,
            dlisth: 0,
            hscrol: 0,
            vscrol: 0,
            pmbase: 0,
            chbase: 0,
            wsync: 0,
            vcount: 0,
            penh: 0,
            penv: 0,
            nmien: 0,
            nmires: 0,
            nmi_asserted: false,
            cycle_accumulator: 0,
            scanline_buffer: [0; 384],
        }
    }

    /// Execute one machine cycle of ANTIC operation.
    /// Returns true if ANTIC is performing DMA (CPU should be halted).
    pub fn tick(&mut self, _mem: &mut Mem) -> bool {
        self.horizontal_pos += 1;

        // One scanline = 114 color clocks (NTSC)
        if self.horizontal_pos >= 114 {
            self.horizontal_pos = 0;
            self.scanline += 1;
            self.vcount = (self.scanline & 0xff) as u8;

            // Reset at end of frame (262 scanlines for NTSC)
            if self.scanline >= 262 {
                self.scanline = 0;
            }
        }

        // TODO: Implement actual DMA logic
        // For now, just return false (no DMA active)
        false
    }

    /// Read from an ANTIC register
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr & 0x0F {
            0x0B => self.vcount,    // VCOUNT is readable
            0x0C => self.penh,      // Light pen H
            0x0D => self.penv,      // Light pen V
            0x0F => {
                // NMIST - NMI status
                // Bit 6 = VBI occurred
                // Bit 7 = DLI occurred
                // Bit 5 = reset key pressed
                self.nmires
            }
            _ => 0xFF,              // Other registers are write-only
        }
    }

    /// Write to an ANTIC register
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr & 0x0F {
            0x00 => {
                self.dmactl = val;
                self.dma_enabled = (val & 0x20) != 0; // Bit 5 enables DMA
            }
            0x01 => self.chactl = val,
            0x02 => {
                self.dlistl = val;
                self.update_dlist_ptr();
            }
            0x03 => {
                self.dlisth = val;
                self.update_dlist_ptr();
            }
            0x04 => self.hscrol = val,
            0x05 => self.vscrol = val,
            0x07 => self.pmbase = val,
            0x09 => self.chbase = val,
            0x0A => self.wsync = val,   // CPU write to WSYNC halts until HSYNC
            0x0E => {
                self.nmien = val;
            }
            0x0F => {
                // Writing to NMIRES clears NMI status bits
                self.nmires = 0;
            }
            _ => {}
        }
    }

    /// Set VBI (Vertical Blank Interrupt) flag in NMIST
    /// Called by Atari800 when triggering VBI
    pub fn set_vbi_flag(&mut self) {
        self.nmires |= 0x40;  // Set bit 6 = VBI occurred
    }

    /// Clear VBI flag (called when NMI is acknowledged)
    pub fn clear_vbi_flag(&mut self) {
        self.nmires &= !0x40;  // Clear bit 6
    }

    /// Check if VBI NMI is enabled
    /// Returns true if NMIEN bit 6 (0x40) is set
    pub fn is_vbi_enabled(&self) -> bool {
        (self.nmien & 0x40) != 0
    }

    fn update_dlist_ptr(&mut self) {
        self.dlist_ptr = (self.dlistl as u16) | ((self.dlisth as u16) << 8);
        self.dlist_index = self.dlist_ptr;  // Reset to start of display list
    }

    /// Reset display list processing state for the start of a new frame.
    /// On real hardware this happens during vertical blank when ANTIC reloads
    /// the display list pointer.
    pub fn start_frame(&mut self) {
        self.dlist_index = self.dlist_ptr;
        self.lines_remaining = 0;
        self.mode_line = 0;
    }

    /// Simulate one frame of ANTIC video timing (for instruction-level emulation)
    /// This advances through all scanlines to keep VCOUNT realistic for OS timing loops
    /// Should be called before each frame render
    pub fn simulate_frame_timing(&mut self) {
        // Simulate that a full frame has passed - cycle through all 262 scanlines
        // This allows OS busy-wait loops on VCOUNT to complete
        // We start at scanline 0 each frame for simplicity
        self.scanline = 0;
        self.vcount = 0;
    }

    /// Advance to a specific scanline (for simulating mid-frame timing)
    pub fn advance_to_scanline(&mut self, target: u16) {
        if target < 262 {
            self.scanline = target;
            self.vcount = (target & 0xff) as u8;
        }
    }

    /// Advance to next scanline (wraps at 262)
    pub fn advance_scanline(&mut self) {
        self.scanline = (self.scanline + 1) % 262;
        self.vcount = (self.scanline / 2) as u8;  // VCOUNT = scanline / 2

        // Pulse NMI at start of VBLANK (scanline 248) if VBI is enabled
        // VBLANK spans scanlines 248-261, then 0-9 (22 lines total)
        // NMI is edge-triggered, so we assert for one scanline then deassert
        if self.scanline == 248 && self.is_vbi_enabled() {
            self.nmi_asserted = true;
            self.nmires |= 0x40;  // Set VBI flag in NMIST
        }

        // Deassert NMI after one scanline (creates falling edge for CPU to detect)
        if self.scanline == 249 {
            self.nmi_asserted = false;
        }
    }

    /// Tick ANTIC by the specified number of CPU cycles
    /// ANTIC advances scanlines every 114 cycles (one scanline = 114 color clocks for NTSC)
    /// This keeps VCOUNT realistic for OS timing loops
    /// Returns true if a frame just completed (scanline wrapped to 0)
    pub fn tick_cycles(&mut self, cycles: u32) -> bool {
        self.cycle_accumulator += cycles;
        let mut frame_complete = false;

        // Advance scanline every 114 cycles
        while self.cycle_accumulator >= 114 {
            self.cycle_accumulator -= 114;
            let prev_scanline = self.scanline;
            self.advance_scanline();

            // Frame completes when scanline wraps from 261 to 0
            if prev_scanline == 261 && self.scanline == 0 {
                frame_complete = true;
            }
        }

        frame_complete
    }

    /// Check if ANTIC is asserting the NMI line
    /// Returns current line state (no side effects)
    pub fn is_nmi_asserted(&self) -> bool {
        self.nmi_asserted
    }

    /// Clear NMI flag in NMIST register (called when software reads NMIST)
    /// This does NOT affect the NMI line - that pulses automatically
    pub fn clear_nmi(&mut self) {
        // Reading NMIST ($D40F) clears the NMI status bits but not the line
        self.nmires = 0;
    }

    /// Get current scanline (for debugging and timing)
    pub fn get_scanline(&self) -> u16 {
        self.scanline
    }

    /// Process one scanline using the display list
    /// This should be called once per scanline (during horizontal blank)
    pub fn process_scanline(&mut self, mem: &Mem) {
        // Clear scanline buffer to background
        self.scanline_buffer.fill(0);

        // Only process if DMA is enabled
        // if !self.dma_enabled {
        //     return;
        // }

        // If we need to fetch a new display list instruction
        if self.lines_remaining == 0 {
            self.fetch_display_list_instruction(mem);
        }

        // Generate scanline data based on current mode
        if self.current_mode == 0x00 {
            // Blank line - already filled with 0
        } else if self.current_mode == 0x02 {
            // Mode 2: 40-column text, 8 scanlines per character row
            self.render_mode2_scanline(mem);
        }
        // TODO: Add other modes

        // Move to next line
        if self.lines_remaining > 0 {
            self.lines_remaining -= 1;
            self.mode_line += 1;
        }
    }

    /// Fetch next display list instruction
    fn fetch_display_list_instruction(&mut self, mem: &Mem) {
        let instruction = mem.get_byte(self.dlist_index);
        self.dlist_index += 1;

        // Check for LMS (Load Memory Scan) bit (bit 6)
        let has_lms = (instruction & 0x40) != 0;

        // Check for DLI (Display List Interrupt) bit (bit 7)
        let _has_dli = (instruction & 0x80) != 0;

        // Extract mode (bits 0-3)
        let mode = instruction & 0x0F;

        // Check for JVB (Jump with Vertical Blank) instruction
        if (instruction & 0x0F) == 0x01 {
            // JVB - jump to new display list address
            let new_addr_lo = mem.get_byte(self.dlist_index);
            let new_addr_hi = mem.get_byte(self.dlist_index + 1);
            self.dlist_index = ((new_addr_hi as u16) << 8) | (new_addr_lo as u16);

            // Re-fetch instruction at new location
            self.fetch_display_list_instruction(mem);
            return;
        }

        self.current_mode = mode;
        self.mode_line = 0;

        // If LMS bit is set, read screen memory address
        if has_lms {
            let screen_lo = mem.get_byte(self.dlist_index);
            let screen_hi = mem.get_byte(self.dlist_index + 1);
            self.screen_ptr = ((screen_hi as u16) << 8) | (screen_lo as u16);
            self.dlist_index += 2;
        }

        // Set number of scanlines for this instruction
        self.lines_remaining = match mode {
            0x00 => {
                // Blank lines - bits 6-4 specify count (0-7 maps to 1-8 blank lines)
                let count = ((instruction >> 4) & 0x07) as u8;
                count + 1
            }
            0x02..=0x07 => 8,  // Text modes: 8 scanlines per character row
            0x08..=0x0F => {
                // Graphics modes - varies by mode
                8  // Simplified for now
            }
            _ => 1,
        };
    }

    /// Render one scanline of ANTIC mode 2 (40-column text)
    fn render_mode2_scanline(&mut self, mem: &Mem) {
        let char_base = if self.chbase == 0 {
            // CHBASE=0 means use OS ROM character set at $E000
            0xE000u16
        } else {
            // Otherwise use CHBASE register (bits 7-1 = address bits 15-9)
            (self.chbase as u16) << 8
        };

        // Render 40 characters
        for char_col in 0..40 {
            // Read character code from screen RAM
            let char_code = mem.get_byte(self.screen_ptr + char_col);

            // Get character bitmap for this scanline
            // Bit 7 = inverse video, bits 0-6 = character index
            let inverse = (char_code & 0x80) != 0;
            let font_index = (char_code & 0x7F) as u16;
            let char_addr = char_base + font_index * 8 + (self.mode_line as u16);
            let char_data = if inverse { mem.get_byte(char_addr) ^ 0xFF } else { mem.get_byte(char_addr) };

            // Convert character bitmap to pixels (8 pixels per character)
            for bit in 0..8 {
                let pixel_on = (char_data & (1 << (7 - bit))) != 0;
                let pixel_x = (char_col * 8 + bit) as usize;

                // Color index: 0 = background, 1 = foreground
                self.scanline_buffer[pixel_x] = if pixel_on { 1 } else { 0 };
            }
        }

        // If this was the last scanline of this character row, advance screen pointer
        if self.mode_line == 7 {
            self.screen_ptr = self.screen_ptr.wrapping_add(40);
        }
    }
}

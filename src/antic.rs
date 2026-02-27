use crate::bus::ReadBus;

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
    pub scanline: u16,       // Current scanline (0-261 for NTSC)
    horizontal_pos: u8,      // Horizontal position within scanline

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

    // Scanline buffer (pixels to be displayed)
    pub scanline_buffer: [u8; 384],  // Color indices for current scanline

    // Player/Missile DMA data (fetched from memory each scanline)
    // [Missiles, P0, P1, P2, P3] - one byte per PM object per scanline
    pub pm_data: [u8; 5],

    // DLI (Display List Interrupt) flag from current display list instruction
    has_dli: bool,

    // Scroll bits from current display list instruction
    has_hsc: bool,  // HSC bit (bit 4) - horizontal fine scroll enabled
    has_vsc: bool,  // VSC bit (bit 5) - vertical fine scroll enabled (future use)
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
            scanline_buffer: [0; 384],
            pm_data: [0; 5],
            has_dli: false,
            has_hsc: false,
            has_vsc: false,
        }
    }

    /// Tick ANTIC one machine cycle. Called in lockstep with CPU.
    /// On cycle 0 of each scanline: deasserts NMI, processes display list,
    /// renders scanline, fetches PM data, asserts VBI if appropriate.
    /// On all other cycles: advances horizontal position.
    pub fn tick(&mut self, bus: &dyn ReadBus) {
        if self.horizontal_pos == 0 {
            // --- Cycle 0: start of scanline ---

            // Deassert NMI from previous scanline (clean edge for next)
            // Also clear NMIST bits — on real ANTIC, NMIST reflects current NMI
            // state, not a persistent latch. The OS DLI path relies on this:
            // it doesn't write NMIRES, so stale DLI bits must auto-clear
            // before VBI fires, otherwise VBI is misinterpreted as DLI.
            self.nmi_asserted = false;
            // Do NOT clear nmires here. NMIST must retain its value until software
            // writes NMIRES ($D40F). The OS NMI handler reads NMIST to distinguish
            // DLI (bit 7) from VBI (bit 6).

            // Reset display list at start of visible area
            if self.scanline == 0 {
                self.start_frame();
            }

            // Visible scanlines: process display list and render (only when DMA enabled)
            if self.scanline < 240 && self.dma_enabled {
                self.process_scanline(bus);
                self.fetch_pm_data(bus, self.scanline as usize);
            }

            // VBI at scanline 248
            if self.scanline == 248 && self.is_vbi_enabled() {
                self.nmi_asserted = true;
                self.nmires = 0x40;  // VBI: replace NMIST so OS sees correct type
            }
        }

        // Advance horizontal position
        self.horizontal_pos += 1;

        // End of scanline — advance to next
        if self.horizontal_pos >= 114 {
            self.horizontal_pos = 0;
            self.scanline = (self.scanline + 1) % 262;
            self.vcount = (self.scanline / 2) as u8;
        }
    }

    /// Read from an ANTIC register
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr & 0x0F {
            0x0B => self.vcount,    // VCOUNT
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

    /// Check if VBI NMI is enabled
    /// Returns true if NMIEN bit 6 (0x40) is set
    pub fn is_vbi_enabled(&self) -> bool {
        (self.nmien & 0x40) != 0
    }

    /// Set scanline for testing purposes
    /// This is a test-only method to allow tests to set a specific scanline
    pub fn set_scanline_for_test(&mut self, scanline: u16) {
        self.scanline = scanline;
    }

    fn update_dlist_ptr(&mut self) {
        self.dlist_ptr = (self.dlistl as u16) | ((self.dlisth as u16) << 8);
        self.dlist_index = self.dlist_ptr;  // Reset to start of display list
    }

    /// Fetch PM graphics data from memory for the specified scanline
    /// This is called once per scanline during PM DMA
    /// scanline: the current display scanline (0-239 for visible area)
    pub fn fetch_pm_data(&mut self, bus: &dyn ReadBus, scanline: usize) {
        // Clear PM data
        self.pm_data = [0; 5];

        // Check if PM DMA is enabled in DMACTL
        let missile_dma_enabled = (self.dmactl & 0x08) != 0;  // Bit 3
        let player_dma_enabled = (self.dmactl & 0x04) != 0;   // Bit 2
        // DMACTL bit 4: 0 = double-line resolution, 1 = single-line resolution
        let single_line_mode = (self.dmactl & 0x10) != 0;     // Bit 4

        if !missile_dma_enabled && !player_dma_enabled {
            return;  // No PM DMA enabled
        }

        // Note: We no longer skip when PMBASE==0 because the OS sets PMBASE during
        // boot and programs may legitimately use low PM base addresses

        // Calculate base address from PMBASE with alignment masking
        let pm_base = if single_line_mode {
            ((self.pmbase & 0xF8) as u16) << 8
        } else {
            ((self.pmbase & 0xFC) as u16) << 8
        };

        // Calculate scanline offset using the passed scanline parameter.
        // On real hardware, PM memory is indexed by the physical scanline counter.
        // The visible display starts at scanline 8 (VCOUNT=4), so we must add 8
        // to match the vertical positioning games expect.
        let adjusted_scanline = scanline + 8;
        let scanline_offset = if single_line_mode {
            // Single-line resolution: one byte per scanline
            adjusted_scanline as u16
        } else {
            // Double-line resolution: each byte used for 2 scanlines
            (adjusted_scanline / 2) as u16
        };

        // Fetch missile data if enabled
        if missile_dma_enabled {
            let missile_offset = if single_line_mode {
                0x300  // Single-line mode offset
            } else {
                0x180  // Double-line mode offset
            };
            let missile_addr = pm_base + missile_offset + scanline_offset;
            self.pm_data[0] = bus.read(missile_addr);
        }

        // Fetch player data if enabled
        if player_dma_enabled {
            for p in 0..4 {
                let player_offset = if single_line_mode {
                    // Single-line: P0=$400, P1=$500, P2=$600, P3=$700
                    0x400 + (p * 0x100)
                } else {
                    // Double-line: P0=$200, P1=$280, P2=$300, P3=$380
                    0x200 + (p * 0x80)
                };
                let player_addr = pm_base + player_offset + scanline_offset;
                self.pm_data[1 + p as usize] = bus.read(player_addr);
            }
        }
    }

    /// Reset display list processing state for the start of a new frame.
    /// On real hardware this happens during vertical blank when ANTIC reloads
    /// the display list pointer.
    pub fn start_frame(&mut self) {
        self.dlist_index = self.dlist_ptr;
        self.lines_remaining = 0;
        self.mode_line = 0;
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

    /// Get current ANTIC mode (used by GTIA for mode F/GTIA mode detection)
    pub fn get_current_mode(&self) -> u8 {
        self.current_mode
    }

    /// Get DMACTL value (for debugging)
    pub fn get_dmactl(&self) -> u8 {
        self.dmactl
    }

    /// Get CHBASE value (for debugging)
    pub fn get_chbase(&self) -> u8 {
        self.chbase
    }

    /// Get NMIEN value (for debugging)
    pub fn get_nmien(&self) -> u8 {
        self.nmien
    }

    /// Get PMBASE value (for debugging)
    pub fn get_pmbase(&self) -> u8 {
        self.pmbase
    }

    /// Mode table entry
    fn mode_info(mode: u8) -> (u8, u16) {
        // Returns (scanlines_per_row, bytes_per_row)
        match mode {
            0x02 => (8, 40),
            0x03 => (10, 40),
            0x04 => (8, 40),
            0x05 => (16, 40),
            0x06 => (8, 20),
            0x07 => (16, 20),
            0x08 => (8, 40),
            0x09 => (4, 20),
            0x0A => (4, 20),
            0x0B => (2, 20),
            0x0C => (1, 20),
            0x0D => (2, 40),
            0x0E => (1, 40),
            0x0F => (1, 40),
            _ => (1, 0),
        }
    }

    /// Process one scanline using the display list
    /// This should be called once per scanline (during horizontal blank)
    pub fn process_scanline(&mut self, bus: &dyn ReadBus) {
        // Clear scanline buffer to background
        self.scanline_buffer.fill(0);

        // If we need to fetch a new display list instruction
        if self.lines_remaining == 0 {
            self.fetch_display_list_instruction(bus);
        }

        // Generate scanline data based on current mode
        let hsc_extra = self.hsc_extra_bytes();
        match self.current_mode {
            0x00 => {} // Blank line - already filled with 0
            0x02 | 0x03 => self.render_text_hires(bus),
            0x04 | 0x05 => self.render_text_multicolor(bus),
            0x06 | 0x07 => self.render_text_wide(bus),
            0x08 => self.render_bitmap_1bpp(bus, 40 + hsc_extra, 1, 2, 0),
            0x09 => self.render_bitmap_1bpp(bus, 20 + hsc_extra, 4, 1, 0),
            0x0A => self.render_bitmap_2bpp(bus, 20 + hsc_extra, 4),
            0x0B => self.render_bitmap_1bpp(bus, 20 + hsc_extra, 2, 2, 0),
            0x0C => self.render_bitmap_1bpp(bus, 20 + hsc_extra, 2, 2, 0),
            0x0D => self.render_bitmap_2bpp(bus, 40 + hsc_extra, 2),
            0x0E => self.render_bitmap_2bpp(bus, 40 + hsc_extra, 2),
            0x0F => self.render_bitmap_1bpp(bus, 40 + hsc_extra, 1, 2, 0),
            _ => {}
        }

        // Apply horizontal fine scroll offset
        self.apply_hscrol();

        // Move to next line and advance screen pointer on last scanline of row
        if self.lines_remaining > 0 {
            let (scanlines, bytes_per_row) = Self::mode_info(self.current_mode);
            let hsc_extra = self.hsc_extra_bytes();
            if self.mode_line == scanlines - 1 && self.current_mode >= 0x02 {
                self.screen_ptr = self.screen_ptr.wrapping_add(bytes_per_row + hsc_extra);
            }
            self.lines_remaining -= 1;
            self.mode_line += 1;
        }

        // Fire DLI on the last line of a DLI-flagged mode instruction
        if self.has_dli && self.lines_remaining == 0 {
            if (self.nmien & 0x80) != 0 {
                self.nmi_asserted = true;
                self.nmires = 0x80;  // DLI: replace NMIST so OS sees correct type
            }
        }
    }

    /// Fetch next display list instruction
    fn fetch_display_list_instruction(&mut self, bus: &dyn ReadBus) {
        let instruction = bus.read(self.dlist_index);
        self.dlist_index += 1;

        // Extract mode (bits 0-3)
        let mode = instruction & 0x0F;

        // Check for DLI (Display List Interrupt) bit (bit 7)
        self.has_dli = (instruction & 0x80) != 0;

        // Parse scroll bits (only meaningful for modes >= 0x02)
        self.has_hsc = (instruction & 0x10) != 0 && mode >= 0x02;
        self.has_vsc = (instruction & 0x20) != 0 && mode >= 0x02;

        // Check for JVB (Jump with Vertical Blank) instruction
        if mode == 0x01 {
            // JVB - jump to new display list address
            let new_addr_lo = bus.read(self.dlist_index);
            let new_addr_hi = bus.read(self.dlist_index + 1);
            self.dlist_index = ((new_addr_hi as u16) << 8) | (new_addr_lo as u16);

            // Re-fetch instruction at new location
            self.fetch_display_list_instruction(bus);
            return;
        }

        self.current_mode = mode;
        self.mode_line = 0;

        // LMS (Load Memory Scan) bit (bit 6) - only valid for non-blank modes
        // Blank lines (mode 0) use bits 6-4 for scan line count, not LMS
        let has_lms = mode != 0x00 && (instruction & 0x40) != 0;

        // If LMS bit is set, read screen memory address
        if has_lms {
            let screen_lo = bus.read(self.dlist_index);
            let screen_hi = bus.read(self.dlist_index + 1);
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
            0x02..=0x0F => Self::mode_info(mode).0,
            _ => 1,
        };
    }

    /// Returns extra bytes to fetch when HSC (horizontal scroll) is enabled.
    /// When scrolling, ANTIC fetches one width class wider than normal.
    fn hsc_extra_bytes(&self) -> u16 {
        if !self.has_hsc {
            return 0;
        }
        let (_scanlines, bytes_per_row) = Self::mode_info(self.current_mode);
        match bytes_per_row {
            40 => 8,  // Normal → wide: fetch 48 bytes
            20 => 4,  // Narrow → normal: fetch 24 bytes
            _ => 0,
        }
    }

    /// Apply HSCROL fine scroll offset by shifting scanline buffer left.
    /// On real hardware, HSCROL=0 means maximum offset (4 chars into extra data),
    /// and HSCROL=15 means minimum offset. Higher HSCROL shifts display RIGHT.
    /// Each HSCROL unit = 1 color clock = 2 hi-res pixels in the buffer.
    fn apply_hscrol(&mut self) {
        if !self.has_hsc {
            return;
        }
        // Invert: HSCROL=0 → max shift (32px), HSCROL=15 → min shift (2px)
        let shift = (16 - (self.hscrol & 0x0F) as usize) * 2;
        self.scanline_buffer.copy_within(shift.., 0);
        let len = self.scanline_buffer.len();
        self.scanline_buffer[len - shift..].fill(0);
    }

    /// Get character set base address
    fn char_base(&self) -> u16 {
        if self.chbase == 0 {
            0xE000u16
        } else {
            (self.chbase as u16) << 8
        }
    }

    /// Render one scanline of text hi-res modes (2, 3)
    /// Mode 2: 40 chars, 8 scanlines/row, inverse video via bit 7
    /// Mode 3: 40 chars, 10 scanlines/row, lowercase descenders
    fn render_text_hires(&mut self, bus: &dyn ReadBus) {
        let char_base = self.char_base();
        let extra = self.hsc_extra_bytes();

        for char_col in 0u16..(40 + extra) {
            let char_code = bus.read(self.screen_ptr + char_col);
            let inverse = (char_code & 0x80) != 0;
            let font_index = (char_code & 0x7F) as u16;

            // For mode 3, scanlines 8-9 use descender data
            // Characters 0-31 and 96-127 show descenders; others are blank
            let font_row = self.mode_line;
            let char_data = if font_row < 8 {
                let char_addr = char_base + font_index * 8 + (font_row as u16);
                bus.read(char_addr)
            } else if self.current_mode == 0x03 {
                // Descender rows (8-9): only lowercase chars (96-127 in font = 0x60-0x7F)
                // Map to next page of character set
                if font_index >= 0x60 {
                    let char_addr = char_base + 0x400 + (font_index - 0x60) * 8 + ((font_row - 8) as u16);
                    bus.read(char_addr)
                } else {
                    0
                }
            } else {
                0
            };

            let char_data = if inverse { char_data ^ 0xFF } else { char_data };

            for bit in 0u16..8 {
                let pixel_on = (char_data & (1 << (7 - bit))) != 0;
                let pixel_x = (char_col * 8 + bit) as usize;
                if pixel_x < 384 {
                    // foreground = COLPF1 (index 2), background = COLPF2 (index 3)
                    self.scanline_buffer[pixel_x] = if pixel_on { 2 } else { 3 };
                }
            }
        }
    }

    /// Render one scanline of text multi-color modes (4, 5)
    /// 40 chars, 2-bit pixel pairs from font data
    /// Colors: 00=COLBK, 01=COLPF0, 10=COLPF1, 11=COLPF2 (or COLPF3 if inverse)
    fn render_text_multicolor(&mut self, bus: &dyn ReadBus) {
        let char_base = self.char_base();
        let extra = self.hsc_extra_bytes();
        // Mode 5 (16 scanlines/row) = double-height, each font row shows on 2 scanlines
        // Mode 4 (8 scanlines/row) = single-height
        let font_row = if self.current_mode == 0x05 {
            (self.mode_line / 2) as u16
        } else {
            self.mode_line as u16
        };

        for char_col in 0u16..(40 + extra) {
            let char_code = bus.read(self.screen_ptr + char_col);
            let inverse = (char_code & 0x80) != 0;
            let font_index = (char_code & 0x7F) as u16;
            let char_addr = char_base + font_index * 8 + font_row;
            let char_data = bus.read(char_addr);

            // Each byte = 4 pixel pairs, each 2 color clocks wide = 8 color clocks
            for pair in 0u16..4 {
                let bits = (char_data >> (6 - pair * 2)) & 0x03;
                let color_index = match bits {
                    0b00 => 0, // COLBK
                    0b01 => 1, // COLPF0
                    0b10 => 2, // COLPF1
                    0b11 => if inverse { 4 } else { 3 }, // COLPF3 or COLPF2
                    _ => 0,
                };
                // Each pixel pair is 2 color clocks wide
                let pixel_x = (char_col * 8 + pair * 2) as usize;
                if pixel_x + 1 < 384 {
                    self.scanline_buffer[pixel_x] = color_index;
                    self.scanline_buffer[pixel_x + 1] = color_index;
                }
            }
        }
    }

    /// Render one scanline of text wide modes (6, 7)
    /// 20 chars, bits 6-7 select color, bits 0-5 = char index (64 chars)
    /// Each pixel is 2 color clocks wide = 160 visible pixels
    fn render_text_wide(&mut self, bus: &dyn ReadBus) {
        let char_base = self.char_base();
        let extra = self.hsc_extra_bytes();
        // Mode 7 (16 scanlines/row) = double-height, each font row shows on 2 scanlines
        // Mode 6 (8 scanlines/row) = single-height
        let font_row = if self.current_mode == 0x07 {
            (self.mode_line / 2) as u16
        } else {
            self.mode_line as u16
        };

        for char_col in 0u16..(20 + extra) {
            let char_code = bus.read(self.screen_ptr + char_col);
            let color_select = (char_code >> 6) & 0x03;
            let font_index = (char_code & 0x3F) as u16;
            let char_addr = char_base + font_index * 8 + font_row;
            let char_data = bus.read(char_addr);

            // Color index: 1=COLPF0, 2=COLPF1, 3=COLPF2, 4=COLPF3
            let fg_index = color_select + 1; // maps 0->1, 1->2, 2->3, 3->4

            for bit in 0u16..8 {
                let pixel_on = (char_data & (1 << (7 - bit))) != 0;
                // Each pixel is 2 color clocks wide, 20 chars * 16 clocks = 320
                let pixel_x = (char_col * 16 + bit * 2) as usize;
                if pixel_x + 1 < 384 {
                    let color = if pixel_on { fg_index } else { 0 }; // bg = COLBK
                    self.scanline_buffer[pixel_x] = color;
                    self.scanline_buffer[pixel_x + 1] = color;
                }
            }
        }
    }

    /// Render one scanline of 1bpp bitmap modes (8, 9, B, C, F)
    fn render_bitmap_1bpp(&mut self, bus: &dyn ReadBus, bytes_per_row: u16, pixel_width: u16, fg_index: u8, bg_index: u8) {
        for byte_col in 0..bytes_per_row {
            let data = bus.read(self.screen_ptr + byte_col);
            for bit in 0u16..8 {
                let pixel_on = (data & (1 << (7 - bit))) != 0;
                let base_x = (byte_col * 8 * pixel_width + bit * pixel_width) as usize;
                let color = if pixel_on { fg_index } else { bg_index };
                for w in 0..(pixel_width as usize) {
                    if base_x + w < 384 {
                        self.scanline_buffer[base_x + w] = color;
                    }
                }
            }
        }
    }

    /// Render one scanline of 2bpp bitmap modes (A, D, E)
    /// 2-bit pairs → indices 0 (COLBK), 1 (COLPF0), 2 (COLPF1), 3 (COLPF2)
    fn render_bitmap_2bpp(&mut self, bus: &dyn ReadBus, bytes_per_row: u16, pixel_width: u16) {
        for byte_col in 0..bytes_per_row {
            let data = bus.read(self.screen_ptr + byte_col);
            for pair in 0u16..4 {
                let bits = (data >> (6 - pair * 2)) & 0x03;
                let color_index = bits; // 0=COLBK, 1=COLPF0, 2=COLPF1, 3=COLPF2
                let base_x = (byte_col * 4 * pixel_width + pair * pixel_width) as usize;
                for w in 0..(pixel_width as usize) {
                    if base_x + w < 384 {
                        self.scanline_buffer[base_x + w] = color_index;
                    }
                }
            }
        }
    }
}

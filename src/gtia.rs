use crate::framebuffer::Framebuffer;

// Horizontal overscan: 32 pixels on left and right
const OVERSCAN_LEFT: usize = 32;

/// GTIA - Graphics Television Interface Adaptor
/// Generates video output, handles player/missile graphics, and collision detection.
///
/// Memory map: $D000-$D01F
pub struct Gtia {
    // Player/Missile colors (write)
    colpm: [u8; 4],     // $D012-$D015

    // Playfield colors (write)
    colpf: [u8; 4],     // $D016-$D019
    colbk: u8,          // $D01A - Background color

    // Control registers
    prior: u8,          // $D01B - Priority selection
    vdelay: u8,         // $D01C - Vertical delay
    gractl: u8,         // $D01D - Graphics control
    hitclr: u8,         // $D01E - Clear collision registers
    consol: u8,         // $D01F - Console switches (write: speaker latch)
    pub consol_input: u8,   // $D01F - Console button state (read: physical buttons)

    // Collision detection (read-only)
    m0pf: u8,           // $D000 - Missile 0 to playfield (read)
    m1pf: u8,           // $D001 - Missile 1 to playfield (read)
    m2pf: u8,           // $D002 - Missile 2 to playfield (read)
    m3pf: u8,           // $D003 - Missile 3 to playfield (read)
    p0pf: u8,           // $D004 - Player 0 to playfield (read)
    p1pf: u8,           // $D005 - Player 1 to playfield (read)
    p2pf: u8,           // $D006 - Player 2 to playfield (read)
    p3pf: u8,           // $D007 - Player 3 to playfield (read)
    m0pl: u8,           // $D008 - Missile 0 to player (read)
    m1pl: u8,           // $D009 - Missile 1 to player (read)
    m2pl: u8,           // $D00A - Missile 2 to player (read)
    m3pl: u8,           // $D00B - Missile 3 to player (read)
    p0pl: u8,           // $D00C - Player 0 to player (read)
    p1pl: u8,           // $D00D - Player 1 to player (read)
    p2pl: u8,           // $D00E - Player 2 to player (read)
    p3pl: u8,           // $D00F - Player 3 to player (read)

    // Player/Missile position registers (write - same addresses as collision reads)
    hposp: [u8; 4],     // $D000-$D003 write - Player horizontal positions
    hposm: [u8; 4],     // $D004-$D007 write - Missile horizontal positions

    // Player/Missile size registers (write - same addresses as collision reads)
    sizep: [u8; 4],     // $D008-$D00B write - Player sizes (0/2=1x, 1=2x, 3=4x)
    sizem: u8,          // $D00C write - Missile sizes (2 bits per missile)

    // Player/Missile graphics registers (write)
    grafp: [u8; 4],     // $D00D-$D010 write - Player graphics patterns
    grafm: u8,          // $D011 write - Missile graphics (2 bits per missile)

    // Paddle/joystick triggers (read-only)
    trig: [u8; 4],      // $D010-$D013

    // Current pixel position
    pixel_x: u16,
    pixel_y: u16,

    // PM scanline buffer (rendered PM objects, one byte per pixel)
    // Bit 0-3 = M0-M3, Bit 4-7 = P0-P3
    pm_scanline: [u8; 384],

    // Width expansion lookup tables (pre-computed for fast rendering)
    // [size_mode][byte_value] -> 32-bit expanded pattern
    grafp_lookup: [[u32; 256]; 4],

    // Framebuffer - GTIA owns the final pixel output
    pub framebuffer: Framebuffer,
}

impl Gtia {
    pub fn new() -> Gtia {
        let mut gtia = Gtia {
            colpm: [0; 4],
            colpf: [0; 4],
            colbk: 0,
            prior: 0,
            vdelay: 0,
            gractl: 0,
            hitclr: 0,
            consol: 0x07,
            consol_input: 0x07,  // Bits 0-2: START, SELECT, OPTION (1 = not pressed)
            m0pf: 0,
            m1pf: 0,
            m2pf: 0,
            m3pf: 0,
            p0pf: 0,
            p1pf: 0,
            p2pf: 0,
            p3pf: 0,
            m0pl: 0,
            m1pl: 0,
            m2pl: 0,
            m3pl: 0,
            p0pl: 0,
            p1pl: 0,
            p2pl: 0,
            p3pl: 0,
            hposp: [0; 4],
            hposm: [0; 4],
            sizep: [0; 4],
            sizem: 0,
            grafp: [0; 4],
            grafm: 0,
            trig: [1, 1, 1, 0],   // TRIG0-2: 1 (not pressed), TRIG3: 0 (no cartridge present)
            pixel_x: 0,
            pixel_y: 0,
            pm_scanline: [0; 384],
            grafp_lookup: [[0; 256]; 4],
            framebuffer: Framebuffer::new(384, 240),  // 384x240 NTSC visible area
        };

        // Pre-compute width expansion lookup tables
        gtia.init_grafp_lookup();

        gtia
    }

    /// Initialize width expansion lookup tables for all size modes
    fn init_grafp_lookup(&mut self) {
        for byte_val in 0..=255 {
            // Size 0/2 (1x): 8 bits → 8 bits (each bit stays 1 bit)
            let mut expanded_1x = 0u32;
            for bit in 0..8 {
                if (byte_val & (1 << (7 - bit))) != 0 {
                    expanded_1x |= 1 << (7 - bit);
                }
            }
            self.grafp_lookup[0][byte_val] = expanded_1x;
            self.grafp_lookup[2][byte_val] = expanded_1x;

            // Size 1 (2x): 8 bits → 16 bits (each bit becomes 2 bits)
            let mut expanded_2x = 0u32;
            for bit in 0..8 {
                if (byte_val & (1 << (7 - bit))) != 0 {
                    expanded_2x |= 0b11 << ((7 - bit) * 2);
                }
            }
            self.grafp_lookup[1][byte_val] = expanded_2x;

            // Size 3 (4x): 8 bits → 32 bits (each bit becomes 4 bits)
            let mut expanded_4x = 0u32;
            for bit in 0..8 {
                if (byte_val & (1 << (7 - bit))) != 0 {
                    expanded_4x |= 0b1111 << ((7 - bit) * 4);
                }
            }
            self.grafp_lookup[3][byte_val] = expanded_4x;
        }
    }

    /// Execute one machine cycle of GTIA operation
    pub fn tick(&mut self) {
        // Generate one color clock of video output
        self.pixel_x += 1;

        // Assuming 228 color clocks per scanline (NTSC)
        if self.pixel_x >= 228 {
            self.pixel_x = 0;
            self.pixel_y += 1;

            // 262 scanlines per frame (NTSC)
            if self.pixel_y >= 262 {
                self.pixel_y = 0;
            }
        }

        // TODO: Implement actual video generation
    }

    /// Read from a GTIA register
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr & 0x1F {
            // Collision registers
            0x00 => self.m0pf,
            0x01 => self.m1pf,
            0x02 => self.m2pf,
            0x03 => self.m3pf,
            0x04 => self.p0pf,
            0x05 => self.p1pf,
            0x06 => self.p2pf,
            0x07 => self.p3pf,
            0x08 => self.m0pl,
            0x09 => self.m1pl,
            0x0A => self.m2pl,
            0x0B => self.m3pl,
            0x0C => self.p0pl,
            0x0D => self.p1pl,
            0x0E => self.p2pl,
            0x0F => self.p3pl,
            // Trigger inputs
            0x10 => self.trig[0],
            0x11 => self.trig[1],
            0x12 => self.trig[2],
            0x13 => self.trig[3],
            // Console switches
            0x1F => self.consol_input,
            // Color registers (0x16-0x1A) and other write-only registers return 0xFF
            _ => 0xFF,
        }
    }

    /// Write to a GTIA register
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr & 0x1F {
            // $D000-$D003: Player horizontal positions (write) / Missile-to-playfield collision (read)
            0x00 => self.hposp[0] = val,
            0x01 => self.hposp[1] = val,
            0x02 => self.hposp[2] = val,
            0x03 => self.hposp[3] = val,
            // $D004-$D007: Missile horizontal positions (write) / Player-to-playfield collision (read)
            0x04 => self.hposm[0] = val,
            0x05 => self.hposm[1] = val,
            0x06 => self.hposm[2] = val,
            0x07 => self.hposm[3] = val,
            // $D008-$D00B: Player sizes (write) / Missile-to-player collision (read)
            0x08 => self.sizep[0] = val,
            0x09 => self.sizep[1] = val,
            0x0A => self.sizep[2] = val,
            0x0B => self.sizep[3] = val,
            // $D00C: Missile sizes (write) / Player-to-player collision (read)
            0x0C => self.sizem = val,
            // $D00D-$D010: Player graphics patterns (write) / Player-to-player collision (read)
            0x0D => self.grafp[0] = val,
            0x0E => self.grafp[1] = val,
            0x0F => self.grafp[2] = val,
            0x10 => self.grafp[3] = val,
            // $D011: Missile graphics (write)
            0x11 => self.grafm = val,
            // Player/Missile colors
            0x12 => self.colpm[0] = val,
            0x13 => self.colpm[1] = val,
            0x14 => self.colpm[2] = val,
            0x15 => self.colpm[3] = val,
            // Playfield colors
            0x16 => self.colpf[0] = val,
            0x17 => self.colpf[1] = val,
            0x18 => self.colpf[2] = val,
            0x19 => self.colpf[3] = val,
            0x1A => self.colbk = val,
            // Control registers
            0x1B => self.prior = val,
            0x1C => self.vdelay = val,
            0x1D => self.gractl = val,
            0x1E => {
                // Writing to HITCLR clears all collision registers
                self.hitclr = val;
                self.m0pf = 0;
                self.m1pf = 0;
                self.m2pf = 0;
                self.m3pf = 0;
                self.p0pf = 0;
                self.p1pf = 0;
                self.p2pf = 0;
                self.p3pf = 0;
                self.m0pl = 0;
                self.m1pl = 0;
                self.m2pl = 0;
                self.m3pl = 0;
                self.p0pl = 0;
                self.p1pl = 0;
                self.p2pl = 0;
                self.p3pl = 0;
            }
            0x1F => self.consol = val,
            _ => {}
        }
    }

    /// Convert Atari color value to RGB
    /// Atari color format: bits 7-4 = hue (0-15), bits 3-1 = luminance (0-7), bit 0 ignored
    pub fn color_to_rgb(&self, atari_color: u8) -> (u8, u8, u8) {
        // Extract hue and luminance
        let hue = (atari_color >> 4) & 0x0F;
        let lum = (atari_color >> 1) & 0x07;

        // Atari NTSC color palette (simplified approximation)
        // This is a basic palette - real hardware varies based on NTSC artifacts
        ATARI_PALETTE[((hue as usize) << 3) | (lum as usize)]
    }

    /// Clear framebuffer to black
    /// Called at the start of each frame
    /// ANTIC will render all 240 scanlines including background color from display list
    pub fn clear_framebuffer(&mut self) {
        // Clear entire framebuffer to black
        // Left/right borders remain black, vertical overscan controlled by display list
        self.framebuffer.clear();
    }

    /// Apply PM DMA data to GRAFP/GRAFM registers, matching real hardware behavior.
    /// On real hardware, ANTIC DMA writes directly to the GRAFP/GRAFM registers.
    /// VDELAY causes even-scanline DMA writes to be skipped (register retains previous value).
    /// pm_dma_data: [Missiles, P0, P1, P2, P3] from ANTIC DMA
    /// scanline: current visible scanline number
    pub fn apply_pm_dma(&mut self, pm_dma_data: &[u8; 5], scanline: usize) {
        let is_odd_scanline = (scanline & 1) != 0;

        // Players: GRACTL bit 1 enables player DMA writes to GRAFP
        if (self.gractl & 0b10) != 0 {
            for p in 0..4 {
                let vdelay_bit = 0x10 << p;
                if is_odd_scanline || (self.vdelay & vdelay_bit) == 0 {
                    // Odd scanline: always write DMA data
                    // Even scanline without VDELAY: write DMA data
                    self.grafp[p] = pm_dma_data[1 + p];
                }
                // Even scanline with VDELAY: skip write, register retains previous value
            }
        }

        // Missiles: GRACTL bit 0 enables missile DMA writes to GRAFM
        if (self.gractl & 0b01) != 0 {
            // Missile VDELAY uses bits 0-3 of VDELAY register
            // Each bit controls one missile's 2-bit field in GRAFM
            // On even scanlines, masked missile bits retain their previous value
            if is_odd_scanline {
                // Odd scanline: always write all missile data
                self.grafm = pm_dma_data[0];
            } else {
                // Even scanline: selectively update based on per-missile VDELAY
                let mut new_grafm = self.grafm;
                for m in 0..4 {
                    let vdelay_bit = 1 << m;  // VDELAY bits 0-3 = missiles 0-3
                    let mask = 0x03u8 << (m * 2);   // 2-bit mask for this missile
                    if (self.vdelay & vdelay_bit) == 0 {
                        // No VDELAY for this missile: update from DMA
                        new_grafm = (new_grafm & !mask) | (pm_dma_data[0] & mask);
                    }
                    // VDELAY set: keep previous value (don't update this missile's bits)
                }
                self.grafm = new_grafm;
            }
        }
    }

    /// Generate PM graphics for the current scanline
    /// Fills pm_scanline buffer with PM object bits
    /// Always reads from GRAFP/GRAFM registers (DMA data should already be applied via apply_pm_dma)
    ///
    /// Coordinate system: HPOS values are in color clocks (0-255).
    /// The visible playfield starts at color clock 48 and is 160 color clocks wide.
    /// Each color clock = 2 hi-res pixels, so we double the width and offset.
    fn generate_pm_scanline(&mut self) {
        // Clear PM scanline buffer
        self.pm_scanline = [0; 384];

        // Check if PM graphics are enabled
        let players_enabled = (self.gractl & 0b10) != 0;
        let missiles_enabled = (self.gractl & 0b01) != 0;

        // PM coordinate offset: HPOS 32 = left edge of wide playfield = pm_scanline[0]
        // Each color clock = 2 hi-res pixels
        // For compositing, pm_scanline[overscan_left + x] aligns with antic_pixels[x]
        // Wide (overscan_left=0): HPOS 32 = pm_scanline[0] = antic_pixels[0]
        // Normal (overscan_left=32): HPOS 48 = pm_scanline[32] = antic_pixels[0]
        // Narrow (overscan_left=64): HPOS 64 = pm_scanline[64] = antic_pixels[0]
        const PM_HPOS_OFFSET: i32 = 32;

        // Render players (if enabled)
        if players_enabled {
            for p in 0..4 {
                // Convert HPOS (color clocks) to screen pixels
                // screen_x = (hpos - 48) * 2
                let hpos = self.hposp[p] as i32;
                let screen_x = (hpos - PM_HPOS_OFFSET) * 2;
                let size_mode = (self.sizep[p] & 0x03) as usize;

                // Always read from GRAFP registers
                let graf = self.grafp[p];

                // Expand graphics pattern using lookup table
                let expanded = self.grafp_lookup[size_mode][graf as usize];

                // Width in screen pixels (each color clock = 2 hi-res pixels)
                // 1x = 8 color clocks = 16 pixels
                // 2x = 16 color clocks = 32 pixels
                // 4x = 32 color clocks = 64 pixels
                let (width_pixels, pattern_bits) = match size_mode {
                    0 | 2 => (16, 8),    // 1x width: 8 bits expanded to 16 pixels
                    1 => (32, 16),       // 2x width: 16 bits expanded to 32 pixels
                    3 => (64, 32),       // 4x width: 32 bits expanded to 64 pixels
                    _ => (16, 8),
                };

                // Place expanded bits in scanline buffer (each pattern bit = 2 screen pixels)
                for bit in 0..pattern_bits {
                    if ((expanded >> (pattern_bits - 1 - bit)) & 1) != 0 {
                        // Each pattern bit becomes 2 screen pixels (for hi-res alignment)
                        let pixels_per_bit = width_pixels / pattern_bits;
                        for dx in 0..pixels_per_bit {
                            let x = screen_x + (bit as i32) * (pixels_per_bit as i32) + (dx as i32);
                            if x >= 0 && x < 384 {
                                self.pm_scanline[x as usize] |= 1 << (4 + p);  // Set player bit (P0-P3 = bits 4-7)
                            }
                        }
                    }
                }
            }
        }

        // Render missiles (if enabled)
        if missiles_enabled {
            // Always read from GRAFM register
            let grafm = self.grafm;

            for m in 0..4 {
                // Convert HPOS (color clocks) to screen pixels
                let hpos = self.hposm[m] as i32;
                let screen_x = (hpos - PM_HPOS_OFFSET) * 2;
                let size_bits = (self.sizem >> (m * 2)) & 0x03;

                // Extract 2-bit missile pattern from GRAFM
                let graf_bits = (grafm >> (m * 2)) & 0x03;

                // Width in screen pixels (each color clock = 2 hi-res pixels)
                // Missiles are 2 color clocks wide at 1x
                // 1x = 2 color clocks = 4 pixels
                // 2x = 4 color clocks = 8 pixels
                // 4x = 8 color clocks = 16 pixels
                let width_pixels = match size_bits {
                    0 | 2 => 4,    // 1x width
                    1 => 8,        // 2x width
                    3 => 16,       // 4x width
                    _ => 4,
                };

                // Expand the 2-bit pattern across the width
                // Each bit of the 2-bit pattern covers half the width
                for bit in 0..2 {
                    if ((graf_bits >> (1 - bit)) & 1) != 0 {
                        let half_width = width_pixels / 2;
                        for dx in 0..half_width {
                            let x = screen_x + (bit as i32) * half_width + dx;
                            if x >= 0 && x < 384 {
                                self.pm_scanline[x as usize] |= 1 << m;  // Set missile bit (M0-M3 = bits 0-3)
                            }
                        }
                    }
                }
            }
        }
    }

    /// Colorize an ANTIC scanline and write to framebuffer
    /// Called once per scanline during frame rendering
    /// antic_mode: current ANTIC display mode (needed for GTIA mode 9/10/11 detection)
    /// dmactl: ANTIC DMACTL register (bits 1-0 = playfield width: 01=narrow, 10=normal, 11=wide)
    /// Note: PM DMA data should be applied to registers via apply_pm_dma() before calling this
    pub fn render_scanline(&mut self, scanline_y: usize, antic_pixels: &[u8; 384], antic_mode: u8, dmactl: u8) {
        let gtia_mode = (self.prior >> 6) & 0x03;

        // Determine playfield width from DMACTL bits 1-0
        let (overscan_left, render_width) = match dmactl & 0x03 {
            0x03 => (0usize, 384usize),   // Wide: 48 bytes → 384 pixels, no border
            0x01 => (64usize, 256usize),  // Narrow: 32 bytes → 256 pixels
            _    => (OVERSCAN_LEFT, 320usize), // Normal (0x02) or default: 40 bytes → 320 pixels
        };

        // Generate PM graphics for this scanline (reads from GRAFP/GRAFM registers)
        self.generate_pm_scanline();

        // GTIA modes 9/10/11: reinterpret ANTIC mode F data
        if gtia_mode != 0 && antic_mode == 0x0F {
            // Group every 4 ANTIC pixels into one 4-bit value
            let group_count = render_width / 4;
            for group in 0..group_count {
                let base = group * 4;
                let nybble = ((antic_pixels[base] & 1) << 3)
                    | ((antic_pixels[base + 1] & 1) << 2)
                    | ((antic_pixels[base + 2] & 1) << 1)
                    | (antic_pixels[base + 3] & 1);

                let atari_color = match gtia_mode {
                    1 => {
                        // GTIA mode 9: 16 luminances, hue from COLBK
                        (self.colbk & 0xF0) | (nybble << 1)
                    }
                    2 => {
                        // GTIA mode 10: 9 colors from registers
                        match nybble {
                            0 => self.colpm[0],
                            1 => self.colpm[1],
                            2 => self.colpm[2],
                            3 => self.colpm[3],
                            4 => self.colpf[0],
                            5 => self.colpf[1],
                            6 => self.colpf[2],
                            7 => self.colpf[3],
                            _ => self.colbk,
                        }
                    }
                    3 => {
                        // GTIA mode 11: 16 hues, luminance from COLBK
                        (nybble << 4) | (self.colbk & 0x0F)
                    }
                    _ => self.colbk,
                };

                let (r, g, b) = self.color_to_rgb(atari_color);
                // Each logical pixel is 4 color clocks wide
                for dx in 0..4 {
                    let x = group * 4 + dx;
                    self.framebuffer.set_pixel(overscan_left + x, scanline_y, r, g, b);
                }
            }
        } else {
            // Standard color index mapping with PM compositing
            for x in 0..render_width {
                let pf_index = antic_pixels[x];
                let pm_bits = self.pm_scanline[overscan_left + x];

                // Composite PM over playfield using priority
                let final_index = self.composite_pixel(pf_index, pm_bits);
                let (r, g, b) = self.get_color_for_index(final_index);
                self.framebuffer.set_pixel(overscan_left + x, scanline_y, r, g, b);

                // Update collision registers
                self.update_collisions(pm_bits, pf_index);
            }
        }
    }

    /// Composite a pixel using PM graphics and playfield with priority
    /// Returns final color index (0-8)
    ///
    /// PRIOR register bits 0-3 select one of four priority modes (one bit set at a time):
    ///   $01: PM0 > PM1 > PM2 > PM3 > PF0 > PF1 > PF2 > PF3 > BAK
    ///   $02: PM0 > PM1 > PF0 > PF1 > PF2 > PF3 > PM2 > PM3 > BAK
    ///   $04: PF0 > PF1 > PM0 > PM1 > PM2 > PM3 > PF2 > PF3 > BAK
    ///   $08: PF0 > PF1 > PF2 > PF3 > PM0 > PM1 > PM2 > PM3 > BAK
    ///   $00: default, same as $01
    fn composite_pixel(&self, pf_index: u8, pm_bits: u8) -> u8 {
        // 5th player mode: missiles combine into a single object using COLPF3
        let (fifth_player, pm_bits_for_groups) = if self.prior & 0x10 != 0 {
            (pm_bits & 0x0F != 0, pm_bits & 0xF0)
        } else {
            (false, pm_bits)
        };

        // Extract PM group presence and colors
        // PM0/1 group: P0 > P1 > M0 > M1
        let pm01_color = if (pm_bits_for_groups & 0x10) != 0 { 5 }  // P0
            else if (pm_bits_for_groups & 0x20) != 0 { 6 }           // P1
            else if (pm_bits_for_groups & 0x01) != 0 { 5 }           // M0 (uses COLPM0)
            else if (pm_bits_for_groups & 0x02) != 0 { 6 }           // M1 (uses COLPM1)
            else { 0 };
        // PM2/3 group: P2 > P3 > M2 > M3
        let pm23_color = if (pm_bits_for_groups & 0x40) != 0 { 7 }   // P2
            else if (pm_bits_for_groups & 0x80) != 0 { 8 }            // P3
            else if (pm_bits_for_groups & 0x04) != 0 { 7 }            // M2 (uses COLPM2)
            else if (pm_bits_for_groups & 0x08) != 0 { 8 }            // M3 (uses COLPM3)
            else { 0 };

        let has_pm01 = pm01_color != 0 || fifth_player;
        let has_pm23 = pm23_color != 0;
        // Resolve PM01 color: actual player color wins, then 5th player (COLPF3 = index 4)
        let pm01_resolved = if pm01_color != 0 { pm01_color } else { 4 };
        let has_pf01 = pf_index == 1 || pf_index == 2;
        let has_pf = pf_index > 0;

        // Priority mode from individual bits 0-3 (lowest set bit wins)
        let priority_bits = self.prior & 0x0F;

        if priority_bits & 0x01 != 0 || priority_bits == 0 {
            // $01 (or $00 default): All PM in front of all PF
            // PM0 > PM1 > PM2 > PM3 > PF0 > PF1 > PF2 > PF3 > BAK
            if has_pm01 { pm01_resolved }
            else if has_pm23 { pm23_color }
            else { pf_index }
        } else if priority_bits & 0x02 != 0 {
            // $02: PM0/PM1 in front, then PF, then PM2/PM3 behind
            // PM0 > PM1 > PF0 > PF1 > PF2 > PF3 > PM2 > PM3 > BAK
            if has_pm01 { pm01_resolved }
            else if has_pf { pf_index }
            else if has_pm23 { pm23_color }
            else { 0 }
        } else if priority_bits & 0x04 != 0 {
            // $04: PF0/PF1 in front, then all PM, then PF2/PF3 behind
            // PF0 > PF1 > PM0 > PM1 > PM2 > PM3 > PF2 > PF3 > BAK
            if has_pf01 { pf_index }
            else if has_pm01 { pm01_resolved }
            else if has_pm23 { pm23_color }
            else { pf_index } // PF2/PF3 or BAK
        } else {
            // $08: All PF in front of all PM
            // PF0 > PF1 > PF2 > PF3 > PM0 > PM1 > PM2 > PM3 > BAK
            if has_pf { pf_index }
            else if has_pm01 { pm01_resolved }
            else if has_pm23 { pm23_color }
            else { 0 }
        }
    }

    /// Update collision registers based on PM and playfield at a pixel
    fn update_collisions(&mut self, pm_bits: u8, pf_index: u8) {
        if pm_bits == 0 {
            return;  // No PM objects, no collisions
        }

        // Extract individual PM bits
        let m = [
            (pm_bits & 0x01) != 0,
            (pm_bits & 0x02) != 0,
            (pm_bits & 0x04) != 0,
            (pm_bits & 0x08) != 0,
        ];
        let p = [
            (pm_bits & 0x10) != 0,
            (pm_bits & 0x20) != 0,
            (pm_bits & 0x40) != 0,
            (pm_bits & 0x80) != 0,
        ];

        // Missile-to-playfield collisions (MxPF)
        if pf_index > 0 && pf_index <= 4 {
            let pf_bit = 1 << (pf_index - 1);
            for i in 0..4 {
                if m[i] {
                    match i {
                        0 => self.m0pf |= pf_bit,
                        1 => self.m1pf |= pf_bit,
                        2 => self.m2pf |= pf_bit,
                        3 => self.m3pf |= pf_bit,
                        _ => {}
                    }
                }
            }
        }

        // Player-to-playfield collisions (PxPF)
        if pf_index > 0 && pf_index <= 4 {
            let pf_bit = 1 << (pf_index - 1);
            for i in 0..4 {
                if p[i] {
                    match i {
                        0 => self.p0pf |= pf_bit,
                        1 => self.p1pf |= pf_bit,
                        2 => self.p2pf |= pf_bit,
                        3 => self.p3pf |= pf_bit,
                        _ => {}
                    }
                }
            }
        }

        // Missile-to-player collisions (MxPL)
        for i in 0..4 {
            if m[i] {
                let mut pl_bits = 0u8;
                for j in 0..4 {
                    if p[j] {
                        pl_bits |= 1 << j;
                    }
                }
                if pl_bits != 0 {
                    match i {
                        0 => self.m0pl |= pl_bits,
                        1 => self.m1pl |= pl_bits,
                        2 => self.m2pl |= pl_bits,
                        3 => self.m3pl |= pl_bits,
                        _ => {}
                    }
                }
            }
        }

        // Player-to-player collisions (PxPL)
        for i in 0..4 {
            if p[i] {
                let mut other_pl_bits = 0u8;
                for j in 0..4 {
                    if j != i && p[j] {
                        other_pl_bits |= 1 << j;
                    }
                }
                if other_pl_bits != 0 {
                    match i {
                        0 => self.p0pl |= other_pl_bits,
                        1 => self.p1pl |= other_pl_bits,
                        2 => self.p2pl |= other_pl_bits,
                        3 => self.p3pl |= other_pl_bits,
                        _ => {}
                    }
                }
            }
        }
    }

    /// Get RGB color for a color index
    fn get_color_for_index(&self, index: u8) -> (u8, u8, u8) {
        let atari_color = match index {
            0 => self.colbk,           // Background
            1 => self.colpf[0],        // Playfield 0
            2 => self.colpf[1],        // Playfield 1
            3 => self.colpf[2],        // Playfield 2
            4 => self.colpf[3],        // Playfield 3
            5 => self.colpm[0],        // Player/Missile 0
            6 => self.colpm[1],        // Player/Missile 1
            7 => self.colpm[2],        // Player/Missile 2
            8 => self.colpm[3],        // Player/Missile 3
            _ => self.colbk,           // Fallback
        };
        self.color_to_rgb(atari_color)
    }

    /// Save framebuffer as PPM image file
    pub fn save_framebuffer(&self, filename: &str) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(filename)?;

        // PPM header
        writeln!(file, "P6")?;
        writeln!(file, "{} {}", self.framebuffer.width, self.framebuffer.height)?;
        writeln!(file, "255")?;

        // Write RGB pixel data
        file.write_all(&self.framebuffer.pixels)?;

        Ok(())
    }

    /// Set joystick trigger state (for input handling)
    /// port: 0-3 for TRIG0-TRIG3
    /// value: 0 = pressed, 1 = not pressed (active low)
    pub fn set_trigger(&mut self, port: usize, value: u8) {
        if port < 4 {
            self.trig[port] = value;
        }
    }

    // Debug accessors for write-only registers
    pub fn get_gractl(&self) -> u8 { self.gractl }
    pub fn get_hposp(&self, p: usize) -> u8 { self.hposp[p.min(3)] }
    pub fn get_sizep(&self, p: usize) -> u8 { self.sizep[p.min(3)] }
    pub fn get_colpm(&self, p: usize) -> u8 { self.colpm[p.min(3)] }
    pub fn get_grafp(&self, p: usize) -> u8 { self.grafp[p.min(3)] }
}

/// Atari 800 NTSC color palette (16 hues × 8 luminance levels = 128 colors)
/// Each entry is (R, G, B) where values are 0-255
const ATARI_PALETTE: [(u8, u8, u8); 128] = [
    // Hue 0 (Gray)
    (0, 0, 0), (25, 25, 25), (55, 55, 55), (79, 79, 79),
    (109, 109, 109), (139, 139, 139), (169, 169, 169), (255, 255, 255),

    // Hue 1 (Gold)
    (65, 45, 0), (89, 67, 0), (119, 97, 11), (143, 121, 35),
    (173, 151, 65), (203, 181, 95), (233, 211, 125), (255, 255, 195),

    // Hue 2 (Orange)
    (105, 35, 0), (129, 59, 0), (159, 89, 0), (183, 113, 23),
    (213, 143, 53), (243, 173, 83), (255, 203, 113), (255, 255, 183),

    // Hue 3 (Red-Orange)
    (105, 20, 0), (129, 44, 0), (159, 74, 0), (183, 98, 19),
    (213, 128, 49), (243, 158, 79), (255, 188, 109), (255, 255, 179),

    // Hue 4 (Pink/Rose)
    (85, 0, 20), (109, 0, 44), (139, 15, 74), (163, 39, 98),
    (193, 69, 128), (223, 99, 158), (253, 129, 188), (255, 199, 255),

    // Hue 5 (Purple)
    (65, 0, 60), (89, 0, 84), (119, 5, 114), (143, 29, 138),
    (173, 59, 168), (203, 89, 198), (233, 119, 228), (255, 189, 255),

    // Hue 6 (Blue-Purple)
    (35, 0, 85), (59, 0, 109), (89, 0, 139), (113, 23, 163),
    (143, 53, 193), (173, 83, 223), (203, 113, 253), (255, 183, 255),

    // Hue 7 (Blue)
    (0, 0, 100), (0, 24, 124), (0, 54, 154), (17, 78, 178),
    (47, 108, 208), (77, 138, 238), (107, 168, 255), (177, 238, 255),

    // Hue 8 (Blue)
    (0, 20, 105), (0, 44, 129), (0, 74, 159), (0, 98, 183),
    (23, 128, 213), (53, 158, 243), (83, 188, 255), (153, 255, 255),

    // Hue 9 (Cyan)
    (0, 40, 85), (0, 64, 109), (0, 94, 139), (0, 118, 163),
    (3, 148, 193), (33, 178, 223), (63, 208, 253), (133, 255, 255),

    // Hue 10 (Cyan-Green)
    (0, 55, 45), (0, 79, 69), (0, 109, 99), (0, 133, 123),
    (0, 163, 153), (19, 193, 183), (49, 223, 213), (119, 255, 255),

    // Hue 11 (Green)
    (0, 60, 0), (0, 84, 0), (0, 114, 11), (0, 138, 35),
    (0, 168, 65), (27, 198, 95), (57, 228, 125), (127, 255, 195),

    // Hue 12 (Yellow-Green)
    (20, 65, 0), (44, 89, 0), (74, 119, 0), (98, 143, 0),
    (128, 173, 23), (158, 203, 53), (188, 233, 83), (255, 255, 153),

    // Hue 13 (Orange-Green)
    (45, 60, 0), (69, 84, 0), (99, 114, 0), (123, 138, 0),
    (153, 168, 15), (183, 198, 45), (213, 228, 75), (255, 255, 145),

    // Hue 14 (Light Orange)
    (60, 55, 0), (84, 79, 0), (114, 109, 0), (138, 133, 0),
    (168, 163, 0), (198, 193, 27), (228, 223, 57), (255, 255, 127),

    // Hue 15 (Yellow)
    (65, 50, 0), (89, 74, 0), (119, 104, 0), (143, 128, 0),
    (173, 158, 21), (203, 188, 51), (233, 218, 81), (255, 255, 151),
];

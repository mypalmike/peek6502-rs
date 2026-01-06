use crate::framebuffer::Framebuffer;

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
    consol: u8,         // $D01F - Console switches

    // Collision detection (read-only)
    m0pf: u8,           // $D000 - Missile 0 to playfield
    m1pf: u8,           // $D001 - Missile 1 to playfield
    m2pf: u8,           // $D002 - Missile 2 to playfield
    m3pf: u8,           // $D003 - Missile 3 to playfield
    p0pf: u8,           // $D004 - Player 0 to playfield
    p1pf: u8,           // $D005 - Player 1 to playfield
    p2pf: u8,           // $D006 - Player 2 to playfield
    p3pf: u8,           // $D007 - Player 3 to playfield
    m0pl: u8,           // $D008 - Missile 0 to player
    m1pl: u8,           // $D009 - Missile 1 to player
    m2pl: u8,           // $D00A - Missile 2 to player
    m3pl: u8,           // $D00B - Missile 3 to player
    p0pl: u8,           // $D00C - Player 0 to player
    p1pl: u8,           // $D00D - Player 1 to player
    p2pl: u8,           // $D00E - Player 2 to player
    p3pl: u8,           // $D00F - Player 3 to player

    // Paddle/joystick triggers (read-only)
    trig: [u8; 4],      // $D010-$D013

    // Current pixel position
    pixel_x: u16,
    pixel_y: u16,

    // Framebuffer - GTIA owns the final pixel output
    pub framebuffer: Framebuffer,
}

impl Gtia {
    pub fn new() -> Gtia {
        Gtia {
            colpm: [0; 4],
            colpf: [0; 4],
            colbk: 0,
            prior: 0,
            vdelay: 0,
            gractl: 0,
            hitclr: 0,
            consol: 0,
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
            trig: [1; 4],   // 1 = not pressed
            pixel_x: 0,
            pixel_y: 0,
            framebuffer: Framebuffer::new(320, 192),
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
            0x1F => self.consol,
            // Color registers (0x16-0x1A) and other write-only registers return 0xFF
            _ => 0xFF,
        }
    }

    /// Write to a GTIA register
    pub fn write_register(&mut self, addr: u16, val: u8) {
        match addr & 0x1F {
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

    /// Clear framebuffer to background color
    /// Called at the start of each frame
    pub fn clear_framebuffer(&mut self) {
        let (r, g, b) = self.color_to_rgb(self.colbk);
        self.framebuffer.clear_color(r, g, b);
    }

    /// Colorize an ANTIC scanline and write to framebuffer
    /// Called once per scanline during frame rendering
    pub fn render_scanline(&mut self, scanline_y: usize, antic_pixels: &[u8; 384]) {
        for x in 0..320 {
            let color_index = antic_pixels[x];
            let (r, g, b) = self.get_color_for_index(color_index);
            self.framebuffer.set_pixel(x, scanline_y, r, g, b);
        }
    }

    /// Get RGB color for a color index (0-3)
    /// Private method - accesses color registers directly without read_register() hack
    fn get_color_for_index(&self, index: u8) -> (u8, u8, u8) {
        let atari_color = match index {
            0 => self.colbk,           // Background
            1 => self.colpf[0],        // Playfield 0
            2 => self.colpf[1],        // Playfield 1
            3 => self.colpf[2],        // Playfield 2
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

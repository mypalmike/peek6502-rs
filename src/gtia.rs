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
            // Write-only registers return 0xFF
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
}

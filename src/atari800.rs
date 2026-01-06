use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::mem::Mem;
use crate::debugger::Debugger;
use crate::antic::Antic;
use crate::gtia::Gtia;
use crate::pokey::Pokey;
use crate::pia::Pia;

pub struct Atari800 {
    // Core components
    cpu: Cpu,
    mem: Mem,

    // Custom chips
    antic: Antic,
    pub gtia: Gtia,  // Public for SDL access to framebuffer
    pokey: Pokey,
    pia: Pia,

    // Debugger
    debugger: Debugger,

    // Cycle tracking
    master_cycle: u64,
    cpu_halted: bool,
}

impl Atari800 {
    pub fn new() -> Atari800 {
        let mut atari800 = Atari800 {
            cpu: Cpu::new(),
            mem: Mem::new(0xC000, false),  // ROM at $C000-$FFFF, load OS ROM
            antic: Antic::new(),
            gtia: Gtia::new(),
            pokey: Pokey::new(),
            pia: Pia::new(),
            debugger: Debugger::new(),
            master_cycle: 0,
            cpu_halted: false,
        };

        // Reset CPU after construction to load PC from reset vector
        // Use mem::replace to temporarily take ownership of CPU
        let mut cpu = std::mem::replace(&mut atari800.cpu, Cpu::new());
        cpu.reset(&mut atari800);  // atari800 implements Bus
        atari800.cpu = cpu;

        // Set up test pattern
        atari800.setup_test_pattern();

        atari800
    }

    pub fn tick(&mut self) {
        // For now, keep debugger-driven execution
        // TODO: Integrate with cycle-accurate execution below

        // We need to temporarily take ownership of cpu and debugger to call tick
        // because we can't borrow self mutably while also passing self as Bus
        let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());
        let mut debugger = std::mem::replace(&mut self.debugger, Debugger::new());

        debugger.tick(&mut cpu, self);

        self.cpu = cpu;
        self.debugger = debugger;

        // Cycle-accurate execution (commented out for now to avoid breaking debugger)
        // self.tick_cycle_accurate();
    }

    /// Cycle-accurate tick - executes one machine cycle
    #[allow(dead_code)]
    fn tick_cycle_accurate(&mut self) {
        // ANTIC runs first and decides if it needs DMA
        let dma_active = self.antic.tick(&mut self.mem);

        if dma_active {
            // ANTIC is using the bus - CPU is halted
            self.cpu_halted = true;
        } else {
            // CPU can execute - executes one cycle
            self.cpu_halted = false;

            // Use mem::replace to temporarily take ownership of CPU
            let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());
            cpu.tick(self);  // CPU now tracks its own multi-cycle state
            self.cpu = cpu;
        }

        // GTIA always runs (generates video)
        self.gtia.tick();

        // POKEY runs (sound, timers, serial I/O)
        self.pokey.tick();

        // PIA runs (joystick input)
        self.pia.tick();

        self.master_cycle += 1;
    }

    /// Set up a test pattern in screen memory AND display list
    fn setup_test_pattern(&mut self) {
        // Screen memory at $4000 (40 chars × 24 lines = 960 bytes)
        let screen_base = 0x4000u16;
        let dlist_base = 0x0600u16;

        // Set GTIA colors
        self.gtia.write_register(0xD01A, 0x00);  // Background: black (COLBK)
        self.gtia.write_register(0xD016, 0x0F);  // Playfield 0: white (COLPF0)
        self.gtia.write_register(0xD017, 0x0F);  // Playfield 1: white (COLPF1) - for text luminance

        // Write "HELLO ATARI 800" centered on first line
        // Convert from ASCII to ATASCII screen codes
        let text = "     HELLO ATARI 800     ";
        for (i, ch) in text.chars().enumerate() {
            let screen_code = Self::ascii_to_atascii(ch);
            self.mem.set_byte(screen_base + i as u16, screen_code);
        }

        // Fill rest of screen with spaces (ATASCII 0x00)
        for i in text.len()..960 {
            self.mem.set_byte(screen_base + i as u16, 0x00);  // Space = 0x00 in ATASCII
        }

        // Build display list at $0600
        let mut dlist_offset = 0u16;

        // 24 blank lines (3 × 8 lines each)
        self.mem.set_byte(dlist_base + dlist_offset, 0x70); dlist_offset += 1;
        self.mem.set_byte(dlist_base + dlist_offset, 0x70); dlist_offset += 1;
        self.mem.set_byte(dlist_base + dlist_offset, 0x70); dlist_offset += 1;

        // Mode 2 (40-column text) with LMS (Load Memory Scan) - first line
        self.mem.set_byte(dlist_base + dlist_offset, 0x42); dlist_offset += 1;  // Mode 2 + LMS
        self.mem.set_byte(dlist_base + dlist_offset, (screen_base & 0xFF) as u8); dlist_offset += 1;
        self.mem.set_byte(dlist_base + dlist_offset, (screen_base >> 8) as u8); dlist_offset += 1;

        // 23 more lines of Mode 2 (no LMS needed, ANTIC auto-increments)
        for _ in 0..23 {
            self.mem.set_byte(dlist_base + dlist_offset, 0x02); dlist_offset += 1;
        }

        // JVB (Jump with Vertical Blank) - jump back to start of display list
        self.mem.set_byte(dlist_base + dlist_offset, 0x41); dlist_offset += 1;
        self.mem.set_byte(dlist_base + dlist_offset, (dlist_base & 0xFF) as u8); dlist_offset += 1;
        self.mem.set_byte(dlist_base + dlist_offset, (dlist_base >> 8) as u8);

        // Set ANTIC registers
        self.antic.write_register(0xD402, (dlist_base & 0xFF) as u8);  // DLISTL
        self.antic.write_register(0xD403, (dlist_base >> 8) as u8);    // DLISTH
        self.antic.write_register(0xD409, 0x00);  // CHBASE = 0 (use built-in font)
        self.antic.write_register(0xD400, 0x22);  // DMACTL = enable DMA, normal width
    }

    /// Convert ASCII character to ATASCII screen code (internal code)
    fn ascii_to_atascii(ch: char) -> u8 {
        match ch {
            ' ' => 0x00,  // Space
            '!' => 0x01,
            '"' => 0x02,
            '#' => 0x03,
            '$' => 0x04,
            '%' => 0x05,
            '&' => 0x06,
            '\'' => 0x07,
            '(' => 0x08,
            ')' => 0x09,
            '*' => 0x0A,
            '+' => 0x0B,
            ',' => 0x0C,
            '-' => 0x0D,
            '.' => 0x0E,
            '/' => 0x0F,
            '0'..='9' => (ch as u8) - b'0' + 0x10,  // Digits 0-9 = 0x10-0x19
            ':' => 0x1A,
            ';' => 0x1B,
            '<' => 0x1C,
            '=' => 0x1D,
            '>' => 0x1E,
            '?' => 0x1F,
            '@' => 0x20,
            'A'..='Z' => (ch as u8) - b'A' + 0x21,  // Uppercase A-Z = 0x21-0x3A
            '[' => 0x3B,
            '\\' => 0x3C,
            ']' => 0x3D,
            '^' => 0x3E,
            '_' => 0x3F,
            '`' => 0x60,
            'a'..='z' => (ch as u8) - b'a' + 0x41,  // Lowercase a-z = 0x41-0x5A
            _ => 0x00,  // Default to space
        }
    }

    /// Render one complete frame using ANTIC display list processing
    /// This simulates one full frame (192 visible scanlines for our simplified display)
    pub fn render(&mut self) {
        // Clear framebuffer to background color
        self.gtia.clear_framebuffer();

        // Process each scanline through ANTIC and GTIA
        for scanline in 0..192 {
            // ANTIC generates color indices from display list
            self.antic.process_scanline(&self.mem);

            // GTIA colorizes and writes to framebuffer
            self.gtia.render_scanline(scanline, &self.antic.scanline_buffer);
        }
    }

    /// Save framebuffer as PPM image file
    /// Delegates to GTIA which owns the framebuffer
    pub fn save_framebuffer(&self, filename: &str) -> std::io::Result<()> {
        self.gtia.save_framebuffer(filename)
    }

    /// Trigger Vertical Blank Interrupt (VBI)
    /// Should be called after each frame render
    /// This is essential for Atari OS and most software to function
    pub fn trigger_vbi(&mut self) {
        // Use mem::replace to temporarily take ownership of CPU
        let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());
        cpu.nmi(self);  // self implements Bus
        self.cpu = cpu;
    }
}

impl Bus for Atari800 {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            // GTIA registers ($D000-$D01F)
            0xD000..=0xD01F => self.gtia.read_register(addr),

            // POKEY registers ($D200-$D2FF)
            0xD200..=0xD2FF => self.pokey.read_register(addr),

            // PIA registers ($D300-$D3FF)
            0xD300..=0xD3FF => self.pia.read_register(addr),

            // ANTIC registers ($D400-$D4FF)
            0xD400..=0xD4FF => self.antic.read_register(addr),

            // Regular memory (RAM/ROM)
            _ => self.mem.get_byte(addr),
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // GTIA registers ($D000-$D01F)
            0xD000..=0xD01F => self.gtia.write_register(addr, val),

            // POKEY registers ($D200-$D2FF)
            0xD200..=0xD2FF => self.pokey.write_register(addr, val),

            // PIA registers ($D300-$D3FF)
            0xD300..=0xD3FF => self.pia.write_register(addr, val),

            // ANTIC registers ($D400-$D4FF)
            0xD400..=0xD4FF => self.antic.write_register(addr, val),

            // Regular memory (RAM/ROM)
            _ => self.mem.set_byte(addr, val),
        }
    }
}

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

    // NMI line (6502 hardware interrupt line)
    nmi_line: bool,

    // IRQ line (6502 hardware interrupt line, shared by POKEY, PIA, PBI)
    irq_line: bool,

    // Cartridge ROM (mapped at $A000-$BFFF for 8KB, $8000-$BFFF for 16KB)
    cart_rom: Option<Vec<u8>>,
    cart_base: u16,
}

impl Atari800 {
    pub fn new() -> Atari800 {
        Atari800::with_cart(None)
    }

    pub fn with_cart(cart_path: Option<&str>) -> Atari800 {
        let is_xex = cart_path.map_or(false, |p| {
            let lower = p.to_lowercase();
            lower.ends_with(".xex") || lower.ends_with(".exe") || lower.ends_with(".com")
        });

        let (cart_rom, cart_base) = if !is_xex {
            match cart_path {
                Some(path) => {
                    let data = std::fs::read(path)
                        .unwrap_or_else(|e| panic!("Failed to load cartridge '{}': {}", path, e));

                    let (rom, base) = if data.len() >= 16 && &data[0..4] == b"CART" {
                        // CART format: 4-byte magic, 4-byte type (big-endian), 4-byte checksum, 4 reserved
                        let cart_type = (data[4] as u32) << 24
                            | (data[5] as u32) << 16
                            | (data[6] as u32) << 8
                            | data[7] as u32;
                        let rom = data[16..].to_vec();
                        let base = match cart_type {
                            1 => { // Standard 8 KB
                                assert!(rom.len() == 0x2000, "CART type 1 expects 8KB ROM, got {}", rom.len());
                                0xA000u16
                            }
                            2 => { // Standard 16 KB
                                assert!(rom.len() == 0x4000, "CART type 2 expects 16KB ROM, got {}", rom.len());
                                0x8000u16
                            }
                            _ => panic!("Unsupported CART type {} — only types 1 (8KB) and 2 (16KB) are supported", cart_type),
                        };
                        println!("Loaded CART type {} ({}KB) from {}", cart_type, rom.len() / 1024, path);
                        (rom, base)
                    } else {
                        // Raw ROM image — determine mapping from size
                        let base = match data.len() {
                            0x2000 => 0xA000u16, // 8KB  -> $A000-$BFFF
                            0x4000 => 0x8000u16, // 16KB -> $8000-$BFFF
                            other => panic!("Unsupported raw cartridge size: {} bytes (expected 8192 or 16384)", other),
                        };
                        println!("Loaded {}KB raw cartridge from {}", data.len() / 1024, path);
                        (data, base)
                    };
                    (Some(rom), base)
                }
                None => (None, 0xA000),
            }
        } else {
            (None, 0xA000)
        };

        let mut atari800 = Atari800 {
            cpu: Cpu::new(),
            mem: Mem::new(0xD800, false),
            antic: Antic::new(),
            gtia: Gtia::new(),
            pokey: Pokey::new(),
            pia: Pia::new(),
            debugger: Debugger::new(),
            master_cycle: 0,
            cpu_halted: false,
            nmi_line: false,
            irq_line: false,
            cart_rom,
            cart_base,
        };

        // Reset CPU after construction to load PC from reset vector
        // Use mem::replace to temporarily take ownership of CPU
        let mut cpu = std::mem::replace(&mut atari800.cpu, Cpu::new());
        cpu.reset(&mut atari800);  // atari800 implements Bus
        atari800.cpu = cpu;

        // Load XEX file if specified
        if is_xex {
            atari800.load_xex(cart_path.unwrap());
        }

        atari800
    }

    /// Load an Atari XEX/EXE/COM binary load file into RAM.
    /// Parses segments, loads data, executes INIT routines, and sets RUN address.
    fn load_xex(&mut self, path: &str) {
        let data = std::fs::read(path)
            .unwrap_or_else(|e| panic!("Failed to load XEX '{}': {}", path, e));

        let mut pos = 0;

        // First segment must start with $FFFF header
        if data.len() < 2 || data[0] != 0xFF || data[1] != 0xFF {
            panic!("Not a valid XEX file: missing $FFFF header");
        }
        pos += 2;

        let mut segment_count = 0;

        while pos + 4 <= data.len() {
            // Optional $FFFF header for subsequent segments
            if pos + 2 <= data.len() && data[pos] == 0xFF && data[pos + 1] == 0xFF {
                pos += 2;
            }

            if pos + 4 > data.len() {
                break;
            }

            let start = data[pos] as u16 | (data[pos + 1] as u16) << 8;
            let end = data[pos + 2] as u16 | (data[pos + 3] as u16) << 8;
            pos += 4;

            if end < start {
                panic!("XEX segment error: end ${:04X} < start ${:04X}", end, start);
            }

            let len = (end - start + 1) as usize;
            if pos + len > data.len() {
                panic!("XEX segment ${:04X}-${:04X} exceeds file size", start, end);
            }

            // Load segment data into RAM
            for i in 0..len {
                self.mem.ram[(start as usize) + i] = data[pos + i];
            }
            segment_count += 1;
            println!("  Segment {}: ${:04X}-${:04X} ({} bytes)", segment_count, start, end, len);
            pos += len;

            // Check for INIT address at $02E2
            let init_addr = self.mem.ram[0x02E2] as u16 | (self.mem.ram[0x02E3] as u16) << 8;
            if init_addr != 0 {
                println!("  INIT: ${:04X}", init_addr);
                // Push a return address that points to a BRK instruction.
                // We place BRK at $0100 (bottom of stack page, rarely used).
                self.mem.ram[0x0100] = 0x00; // BRK
                let ret_addr: u16 = 0x0100 - 1; // RTS pops addr and adds 1
                let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());
                // Set up CPU to JSR to init routine
                cpu.s = 0xFB;
                cpu.pc = init_addr;
                // Push return address onto stack (high byte first, then low)
                cpu.s = cpu.s.wrapping_sub(1);
                self.mem.ram[0x0100 + cpu.s as usize + 1] = (ret_addr >> 8) as u8;
                cpu.s = cpu.s.wrapping_sub(1);
                self.mem.ram[0x0100 + cpu.s as usize + 1] = ret_addr as u8;

                // Execute until we hit our BRK sentinel
                let mut max_cycles = 10_000_000u32;
                loop {
                    if cpu.pc == 0x0100 {
                        break;
                    }
                    cpu.tick(&mut *self);
                    max_cycles -= 1;
                    if max_cycles == 0 {
                        println!("  WARNING: INIT routine at ${:04X} did not return after 10M cycles", init_addr);
                        break;
                    }
                }
                self.cpu = cpu;

                // Clear INIT vector
                self.mem.ram[0x02E2] = 0;
                self.mem.ram[0x02E3] = 0;
            }
        }

        // Check for RUN address at $02E0
        let run_addr = self.mem.ram[0x02E0] as u16 | (self.mem.ram[0x02E1] as u16) << 8;
        if run_addr != 0 {
            println!("  RUN: ${:04X}", run_addr);
            self.cpu.pc = run_addr;
        } else {
            println!("  No RUN address specified");
        }

        println!("Loaded XEX from {} ({} segments)", path, segment_count);
    }

    pub fn tick(&mut self) {
        // For debugger mode - uses interactive debugger
        // We need to temporarily take ownership of cpu and debugger to call tick
        // because we can't borrow self mutably while also passing self as Bus
        let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());
        let mut debugger = std::mem::replace(&mut self.debugger, Debugger::new());

        debugger.tick(&mut cpu, self);

        self.cpu = cpu;
        self.debugger = debugger;
    }

    /// Execute one CPU instruction without debugger (for normal emulation)
    /// Returns true if ANTIC completed a frame (for rendering/speed limiting)
    pub fn tick_cpu(&mut self) -> bool {
        // Take ownership of CPU temporarily
        let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());

        // Execute one instruction (returns 1 cycle per tick)
        let mut cycles = 0;

        // Execute until instruction completes (may take multiple cycles)
        cpu.tick(self);
        cycles += 1;

        // Continue until instruction finishes
        while cpu.cycles_remaining > 0 {
            cpu.tick(self);
            cycles += 1;
        }

        // Restore CPU
        self.cpu = cpu;

        // Tick POKEY for each CPU cycle (timers, serial I/O)
        for _ in 0..cycles {
            self.pokey.tick();
        }
        self.irq_line = self.pokey.irq_active();

        // Update ANTIC video timing based on cycles executed
        // ANTIC manages its own scanline advancement and signals frame completion
        let frame_complete = self.antic.tick_cycles(cycles);

        // Update NMI line AFTER ANTIC advances so CPU sees the assertion
        // on the next instruction (NMI is edge-triggered in cpu.tick())
        self.nmi_line = self.antic.is_nmi_asserted() || self.pia.is_nmi_asserted();

        frame_complete
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
    pub fn setup_test_pattern(&mut self) {
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
        // Debug: Print screen memory once at frame 300
        static mut FRAME_COUNT: u32 = 0;
        unsafe {
            FRAME_COUNT += 1;
            if FRAME_COUNT == 300 {
                // Check first 3 lines of screen memory
                eprintln!("\n=== SCREEN MEMORY DUMP ===");
                for line in 0..3 {
                    eprint!("Line {}: ", line);
                    for i in 0..40 {
                        let ch = self.mem.get_byte(0xCC40 + line * 40 + i);
                        if ch == 0x00 {
                            eprint!("_");
                        } else {
                            eprint!("{:02X}", ch);
                        }
                    }
                    eprintln!();
                }

                // Also check DOSVEC
                let dosvec = self.mem.get_byte(0x000A) as u16 | ((self.mem.get_byte(0x000B) as u16) << 8);
                eprintln!("DOSVEC = ${:04X}", dosvec);
            }
        }

        // Clear framebuffer to background color
        self.gtia.clear_framebuffer();

        // Reset ANTIC display list state for new frame (simulates vertical blank)
        self.antic.start_frame();

        // Process scanlines through ANTIC and GTIA
        // NTSC visible area: 240 scanlines of display list processing,
        // but only 192 scanlines rendered to framebuffer.
        // The first 8 scanlines are top blank (VBLANK end), then 24 blank from
        // the display list's $70 instructions, then 192 visible lines, then bottom blank.
        // We process 24 blank scanlines first (display list overhead), then render 192.
        let blank_lines = 24;
        for _ in 0..blank_lines {
            self.antic.process_scanline(&self.mem);
        }
        for scanline in 0..192 {
            self.antic.process_scanline(&self.mem);
            self.gtia.render_scanline(scanline, &self.antic.scanline_buffer, self.antic.get_current_mode());
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
        // Set VBI flag in ANTIC so OS can read NMIST
        self.antic.set_vbi_flag();

        // Only trigger NMI if VBI is enabled in NMIEN register (bit 6)
        // This matches real hardware behavior - OS enables VBI after initialization
        if self.antic.is_vbi_enabled() {
            // Trigger NMI on CPU
            let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());
            cpu.nmi(self);  // self implements Bus
            self.cpu = cpu;
        }
    }

    /// Enable CPU execution tracing for debugging
    /// Traces the specified number of instructions to stderr
    pub fn enable_cpu_trace(&mut self, count: u32) {
        self.cpu.trace_remaining = count;
    }

    /// Handle keyboard key press event
    /// Calls POKEY to update keyboard registers and manage IRQ line
    pub fn handle_key_press(&mut self, atari_key_code: u8, shift: bool, ctrl: bool) {
        if self.pokey.key_press(atari_key_code, shift, ctrl) {
            self.irq_line = true;
        }
    }

    /// Handle keyboard key release event
    /// Clears the keyboard code register
    pub fn handle_key_release(&mut self) {
        self.irq_line = self.pokey.key_release();
    }

    /// Handle Break key press (triggers POKEY IRQ bit 7)
    pub fn handle_break_key(&mut self) {
        if self.pokey.break_key_press() {
            self.irq_line = true;
        }
    }

    /// Press a console button (OPTION/SELECT/START)
    /// bit 0 = START, bit 1 = SELECT, bit 2 = OPTION (active-low: 0 = pressed)
    pub fn console_press(&mut self, bit: u8) {
        self.gtia.consol_input &= !(1 << bit);
    }

    /// Release a console button
    pub fn console_release(&mut self, bit: u8) {
        self.gtia.consol_input |= 1 << bit;
    }

    /// Advance ANTIC scanline counter (for simulating video timing in instruction-level mode)
    /// Called periodically during CPU execution to keep VCOUNT realistic
    pub fn advance_scanline(&mut self) {
        self.antic.advance_scanline();
    }

    /// Get current scanline from ANTIC (for timing and debugging)
    pub fn get_scanline(&self) -> u16 {
        self.antic.get_scanline()
    }

    /// Read memory byte (for debugging)
    pub fn read_mem(&self, addr: u16) -> u8 {
        self.mem.get_byte(addr)
    }
}

impl Bus for Atari800 {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            // GTIA registers ($D000-$D0FF) - mirrors due to incomplete address decoding
            0xD000..=0xD0FF => self.gtia.read_register(addr),

            // Unused I/O space ($D100-$D1FF)
            0xD100..=0xD1FF => 0xFF,

            // POKEY registers ($D200-$D2FF)
            0xD200..=0xD2FF => self.pokey.read_register(addr),

            // PIA registers ($D300-$D3FF)
            0xD300..=0xD3FF => self.pia.read_register(addr),

            // ANTIC registers ($D400-$D4FF)
            0xD400..=0xD4FF => self.antic.read_register(addr),

            // Unused I/O space ($D500-$D7FF)
            0xD500..=0xD7FF => 0xFF,

            // Cartridge space
            0x8000..=0xBFFF if self.cart_rom.is_some() && addr >= self.cart_base => {
                let rom = self.cart_rom.as_ref().unwrap();
                rom[(addr - self.cart_base) as usize]
            }
            0xA000..=0xBFFF => 0xFF, // No cartridge

            // Unmapped space ($C000-$CFFF) - no RAM here on 48K Atari 800
            0xC000..=0xCFFF => 0xFF,

            // Regular memory (RAM/ROM)
            _ => self.mem.get_byte(addr),
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            // GTIA registers ($D000-$D0FF) - mirrors due to incomplete address decoding
            0xD000..=0xD0FF => self.gtia.write_register(addr, val),

            // Unused I/O space ($D100-$D1FF) - ignore writes
            0xD100..=0xD1FF => {}

            // POKEY registers ($D200-$D2FF)
            0xD200..=0xD2FF => {
                // Special handling for IRQEN register ($D20E)
                if (addr & 0x0F) == 0x0E {
                    self.irq_line = self.pokey.write_irqen(val);
                } else {
                    self.pokey.write_register(addr, val);
                }
            }

            // PIA registers ($D300-$D3FF)
            0xD300..=0xD3FF => self.pia.write_register(addr, val),

            // ANTIC registers ($D400-$D4FF)
            0xD400..=0xD4FF => self.antic.write_register(addr, val),

            // Unused I/O space ($D500-$D7FF) - ignore writes
            0xD500..=0xD7FF => {}

            // Cartridge space - ROM, ignore writes
            0x8000..=0xBFFF if self.cart_rom.is_some() && addr >= self.cart_base => {}
            0xA000..=0xBFFF => {}

            // Unmapped space ($C000-$CFFF) - no RAM here on 48K Atari 800
            0xC000..=0xCFFF => {}

            // Regular memory (RAM/ROM)
            _ => self.mem.set_byte(addr, val),
        }
    }

    fn nmi_asserted(&self) -> bool {
        self.nmi_line
    }

    fn irq_asserted(&self) -> bool {
        self.irq_line
    }
}

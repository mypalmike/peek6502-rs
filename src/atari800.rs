use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::mem::Mem;  // Still needed for temp_mem compatibility layer
use crate::debugger::Debugger;
use crate::antic::Antic;
use crate::gtia::Gtia;
use crate::pokey::Pokey;
use crate::pia::Pia;
use crate::joystick::{JoystickInput, KeyboardJoystick};
use crate::memory_map::MemoryMap;
use crate::memory_region::{MemoryRegionType, RamRegion, RomRegion, IoRegion, ChipType};
use crate::banking::{BankController, BankingScheme};
use crate::machine_config::{MachineConfig, MachineType};
use crate::rom_scanner::RomDatabase;

pub struct Atari800 {
    // Core components
    cpu: Cpu,

    // NEW: Memory architecture
    memory_map: MemoryMap,
    bank_controller: BankController,
    config: MachineConfig,

    // Custom chips
    antic: Antic,
    pub gtia: Gtia,  // Public for SDL access to framebuffer
    pokey: Pokey,
    pia: Pia,

    // Input devices
    joystick: Box<dyn JoystickInput>,

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
        Atari800::with_config(MachineType::Atari800, None)
    }

    pub fn with_cart(cart_path: Option<&str>) -> Atari800 {
        Atari800::with_config(MachineType::Atari800, cart_path)
    }

    /// Create a minimal Atari800 for render/animation tests
    /// No CPU execution, no OS ROM, no banking - just ANTIC/GTIA testing
    pub fn for_render_test() -> Atari800 {
        // Simple memory map - just 64KB RAM, no ROMs
        let mut memory_map = MemoryMap::new();
        memory_map.add_region(MemoryRegionType::Ram(RamRegion::new(
            0x0000,
            0xFFFF,
            0,
        )));

        // No banking needed for tests
        let bank_controller = BankController::new(BankingScheme::None);

        // Minimal config
        let config = MachineConfig {
            machine_type: MachineType::Atari800,
            ram_size: 64 * 1024,
            os_rom_path: None,
            os_rom_start: 0xD800,
            os_rom_size: 0,
            basic_rom_path: None,
            basic_rom_size: 0,
            banking_scheme: BankingScheme::None,
        };

        Atari800 {
            cpu: Cpu::new(),
            memory_map,
            bank_controller,
            config,
            antic: Antic::new(),
            gtia: Gtia::new(),
            pokey: Pokey::new(),
            pia: Pia::new(),
            joystick: Box::new(KeyboardJoystick::new()),
            debugger: Debugger::new(),
            master_cycle: 0,
            cpu_halted: false,
            nmi_line: false,
            irq_line: false,
            cart_rom: None,
            cart_base: 0xA000,
        }
    }

    pub fn with_config(machine_type: MachineType, cart_path: Option<&str>) -> Atari800 {
        // Scan ROMs directory to find ROM files by MD5
        println!("Scanning roms/ directory for ROM files...");
        let rom_db = RomDatabase::scan("roms");

        // Get machine configuration with ROM paths from database
        let config = MachineConfig::for_machine(machine_type, &rom_db);

        // Check if loading a XEX executable
        let is_xex = cart_path.map_or(false, |p| {
            let lower = p.to_lowercase();
            lower.ends_with(".xex") || lower.ends_with(".exe") || lower.ends_with(".com")
        });

        // Load cartridge ROM if specified (and not a XEX)
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

        // Build memory map
        let mut memory_map = MemoryMap::new();

        // 1. Add RAM (entire address space up to ram_size, lowest priority)
        memory_map.add_region(MemoryRegionType::Ram(RamRegion::new(
            0x0000,
            (config.ram_size as u16).saturating_sub(1).min(0xFFFF),
            0,  // Lowest priority
        )));

        // 2. Add I/O regions (high priority, overlays RAM)
        memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD000,
            0xD0FF,
            ChipType::Gtia,
            100,
        )));
        memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD200,
            0xD2FF,
            ChipType::Pokey,
            100,
        )));
        memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD300,
            0xD3FF,
            ChipType::Pia,
            100,
        )));
        memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD400,
            0xD4FF,
            ChipType::Antic,
            100,
        )));

        // 3. Load and add OS ROM (medium priority)
        let os_rom_data = if let Some(ref os_rom_path) = config.os_rom_path {
            std::fs::read(os_rom_path)
                .unwrap_or_else(|e| {
                    eprintln!("Warning: Could not load OS ROM '{}': {}", os_rom_path, e);
                    eprintln!("Using empty ROM (system may not boot)");
                    vec![0xFF; config.os_rom_size]
                })
        } else {
            eprintln!("Warning: OS ROM not found in roms/ directory");
            eprintln!("Using empty ROM (system may not boot)");
            vec![0xFF; config.os_rom_size]
        };

        // 4. Set up banking if needed
        let mut bank_controller = BankController::new(config.banking_scheme);

        // For banked systems (800XL), ROM goes into bank controller
        // For non-banked systems (800), ROM goes into memory map
        if let Some(ref basic_path) = config.basic_rom_path {
            let basic_data = std::fs::read(basic_path)
                .unwrap_or_else(|e| {
                    eprintln!("Warning: Could not load BASIC ROM '{}': {}", basic_path, e);
                    eprintln!("Using empty ROM (BASIC will not work)");
                    vec![0xFF; config.basic_rom_size]
                });
            bank_controller.add_basic_rom(basic_data);
            bank_controller.add_os_rom_banking(os_rom_data);
            // Don't add OS ROM to memory_map - it's handled by banking
        } else {
            // No banking - add OS ROM directly to memory map
            memory_map.add_region(MemoryRegionType::Rom(RomRegion::new(
                config.os_rom_start,
                os_rom_data,
                50,
            )));
        }

        // Create system
        let mut atari800 = Atari800 {
            cpu: Cpu::new(),
            memory_map,
            bank_controller,
            config,
            antic: Antic::new(),
            gtia: Gtia::new(),
            pokey: Pokey::new(),
            pia: Pia::new(),
            joystick: Box::new(KeyboardJoystick::new()),
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
                self.write(start + i as u16, data[pos + i]);
            }
            segment_count += 1;
            println!("  Segment {}: ${:04X}-${:04X} ({} bytes)", segment_count, start, end, len);
            pos += len;

            // Check for INIT address at $02E2
            let init_addr = self.read(0x02E2) as u16 | (self.read(0x02E3) as u16) << 8;
            if init_addr != 0 {
                println!("  INIT: ${:04X}", init_addr);
                // Push a return address that points to a BRK instruction.
                // We place BRK at $0100 (bottom of stack page, rarely used).
                self.write(0x0100, 0x00); // BRK
                let ret_addr: u16 = 0x0100 - 1; // RTS pops addr and adds 1
                let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());
                // Set up CPU to JSR to init routine
                cpu.s = 0xFB;
                cpu.pc = init_addr;
                // Push return address onto stack (high byte first, then low)
                cpu.s = cpu.s.wrapping_sub(1);
                self.write(0x0100 + cpu.s as u16 + 1, (ret_addr >> 8) as u8);
                cpu.s = cpu.s.wrapping_sub(1);
                self.write(0x0100 + cpu.s as u16 + 1, ret_addr as u8);

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
                self.write(0x02E2, 0);
                self.write(0x02E3, 0);
            }
        }

        // Check for RUN address at $02E0
        let run_addr = self.read(0x02E0) as u16 | (self.read(0x02E1) as u16) << 8;
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

    /// Execute CPU for one scanline worth of cycles (per-scanline execution)
    /// Returns true if ANTIC completed a frame (for rendering/speed limiting)
    pub fn tick_cpu(&mut self) -> bool {
        const CYCLES_PER_SCANLINE: u32 = 114;  // One scanline = 114 color clocks

        // Take ownership of CPU temporarily
        let mut cpu = std::mem::replace(&mut self.cpu, Cpu::new());

        let mut cycles_this_scanline = 0;

        // Execute CPU for one scanline worth of cycles
        while cycles_this_scanline < CYCLES_PER_SCANLINE {
            // Execute one CPU cycle
            cpu.tick(self);
            cycles_this_scanline += 1;

            // Tick POKEY for this cycle
            self.pokey.tick();
        }

        // Restore CPU
        self.cpu = cpu;

        // Update IRQ line from POKEY
        self.irq_line = self.pokey.irq_active();

        // Advance ANTIC one scanline
        self.antic.advance_scanline();

        // Update NMI line AFTER ANTIC advances (edge-triggered)
        self.nmi_line = self.antic.is_nmi_asserted() || self.pia.is_nmi_asserted();

        // Check if frame completed (scanline wrapped to 0)
        self.antic.scanline == 0
    }

    /// Cycle-accurate tick - executes one machine cycle
    /// NOTE: Currently disabled - would need refactoring to work with new memory architecture
    #[allow(dead_code)]
    fn _tick_cycle_accurate_disabled(&mut self) {
        // ANTIC runs first and decides if it needs DMA
        // NOTE: This would need to be refactored to use Bus instead of direct Mem access
        let dma_active = false; // self.antic.tick(&mut self.mem);

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
            self.write(screen_base + i as u16, screen_code);
        }

        // Fill rest of screen with spaces (ATASCII 0x00)
        for i in text.len()..960 {
            self.write(screen_base + i as u16, 0x00);  // Space = 0x00 in ATASCII
        }

        // Build display list at $0600
        let mut dlist_offset = 0u16;

        // 24 blank lines (3 × 8 lines each)
        self.write(dlist_base + dlist_offset, 0x70); dlist_offset += 1;
        self.write(dlist_base + dlist_offset, 0x70); dlist_offset += 1;
        self.write(dlist_base + dlist_offset, 0x70); dlist_offset += 1;

        // Mode 2 (40-column text) with LMS (Load Memory Scan) - first line
        self.write(dlist_base + dlist_offset, 0x42); dlist_offset += 1;  // Mode 2 + LMS
        self.write(dlist_base + dlist_offset, (screen_base & 0xFF) as u8); dlist_offset += 1;
        self.write(dlist_base + dlist_offset, (screen_base >> 8) as u8); dlist_offset += 1;

        // 23 more lines of Mode 2 (no LMS needed, ANTIC auto-increments)
        for _ in 0..23 {
            self.write(dlist_base + dlist_offset, 0x02); dlist_offset += 1;
        }

        // JVB (Jump with Vertical Blank) - jump back to start of display list
        self.write(dlist_base + dlist_offset, 0x41); dlist_offset += 1;
        self.write(dlist_base + dlist_offset, (dlist_base & 0xFF) as u8); dlist_offset += 1;
        self.write(dlist_base + dlist_offset, (dlist_base >> 8) as u8);

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
    /// Processes all 240 visible NTSC scanlines
    pub fn render(&mut self) {
        // Clear framebuffer to background color
        self.gtia.clear_framebuffer();

        // Reset ANTIC display list state for new frame (simulates vertical blank)
        self.antic.start_frame();

        // Get temporary Mem for ANTIC processing (compatibility layer)
        let temp_mem = self.get_temp_mem();

        // Process all 240 visible NTSC scanlines through ANTIC display list
        // The display list itself controls which lines are blank (overscan) vs. content
        for scanline in 0..240 {
            self.antic.process_scanline(&temp_mem);
            self.gtia.render_scanline(scanline, &self.antic.scanline_buffer, self.antic.get_current_mode());
        }

        // Update memory if ANTIC modified anything (currently no-op)
        self.update_from_temp_mem(&temp_mem);
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

    /// Get current CPU program counter (for debugging)
    pub fn get_pc(&self) -> u16 {
        self.cpu.pc
    }

    /// Check if CPU is executing from RAM vs ROM
    pub fn is_executing_from_rom(&self) -> bool {
        let pc = self.cpu.pc;
        // Check if PC is in OS ROM range
        if self.config.machine_type == MachineType::Atari800XL {
            pc >= 0xC000  // XL OS ROM
        } else {
            pc >= 0xD800  // 800 OS-B ROM
        }
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

    /// Handle joystick direction keys (Option+arrows)
    pub fn handle_joystick_direction(&mut self, up: bool, down: bool, left: bool, right: bool) {
        if let Some(keyboard_joy) = self.joystick.as_any_mut().downcast_mut::<KeyboardJoystick>() {
            keyboard_joy.handle_direction(up, down, left, right);
        }
        self.update_joystick_state();
    }

    /// Handle joystick trigger key (Option+/)
    pub fn handle_joystick_trigger(&mut self, pressed: bool) {
        if let Some(keyboard_joy) = self.joystick.as_any_mut().downcast_mut::<KeyboardJoystick>() {
            keyboard_joy.handle_trigger(pressed);
        }
        self.update_joystick_state();
    }

    /// Set Option/Alt key state for joystick emulation
    pub fn set_joystick_option(&mut self, pressed: bool) {
        if let Some(keyboard_joy) = self.joystick.as_any_mut().downcast_mut::<KeyboardJoystick>() {
            keyboard_joy.set_option(pressed);
            if !pressed {
                keyboard_joy.release_all();
            }
        }
        self.update_joystick_state();
    }

    /// Update PIA and GTIA with current joystick state
    fn update_joystick_state(&mut self) {
        // Get state for all 4 joystick ports
        let joy1 = self.joystick.get_state(1);
        let joy2 = self.joystick.get_state(2);
        let joy3 = self.joystick.get_state(3);
        let joy4 = self.joystick.get_state(4);

        // Update PORTA (joysticks 1 & 2)
        // Bits 0-3: joy1, Bits 4-7: joy2
        let porta = (joy2.to_pia_bits() << 4) | joy1.to_pia_bits();
        self.pia.set_porta_input(porta);

        // Update PORTB (joysticks 3 & 4)
        // Bits 0-3: joy3, Bits 4-7: joy4
        let portb = (joy4.to_pia_bits() << 4) | joy3.to_pia_bits();
        self.pia.set_portb_input(portb);

        // Update GTIA triggers
        self.gtia.set_trigger(0, joy1.trigger_value());
        self.gtia.set_trigger(1, joy2.trigger_value());
        self.gtia.set_trigger(2, joy3.trigger_value());
        self.gtia.set_trigger(3, joy4.trigger_value());
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
        // Read through the memory map
        self.memory_map.read(addr).unwrap_or(0xFF)
    }

    /// Get temporary Mem wrapper for ANTIC scanline processing
    /// This is a compatibility layer until ANTIC is refactored to use Bus
    fn get_temp_mem(&mut self) -> Mem {
        // Create a temporary Mem struct by extracting RAM from memory map
        // This is inefficient but maintains compatibility with existing ANTIC code
        if let Some(region) = self.memory_map.find_region_mut(0x0000) {
            if let MemoryRegionType::Ram(ram) = region {
                // Create a Mem struct that wraps the RAM data
                // For now, just create one with the OS ROM start from config
                let mut mem = Mem::new(self.config.os_rom_start, false);
                // Copy RAM data (inefficient but works for now)
                mem.ram[..ram.data.len()].copy_from_slice(&ram.data);
                return mem;
            }
        }
        // Fallback: create empty Mem
        Mem::new(0xD800, false)
    }

    /// Update RAM from temporary Mem (after ANTIC processing)
    fn update_from_temp_mem(&mut self, _mem: &Mem) {
        // ANTIC doesn't write to memory during scanline processing, so this is a no-op
        // If ANTIC ever needs to write (e.g., for DMA), this would need implementation
    }
}

impl Bus for Atari800 {
    fn read(&mut self, addr: u16) -> u8 {
        // 1. Cartridge ROM (highest priority, not in memory map)
        if let Some(cart_rom) = &self.cart_rom {
            if addr >= self.cart_base && addr < self.cart_base + (cart_rom.len() as u16) {
                return cart_rom[(addr - self.cart_base) as usize];
            }
        }

        // 2. Check memory map for region type
        if let Some(region) = self.memory_map.find_region(addr) {
            match region {
                MemoryRegionType::Io(io_region) => {
                    // Route to appropriate chip based on region config
                    let val = match io_region.chip_type {
                        ChipType::Gtia => self.gtia.read_register(addr),
                        ChipType::Pokey => self.pokey.read_register(addr),
                        ChipType::Pia => self.pia.read_register(addr),
                        ChipType::Antic => self.antic.read_register(addr),
                    };
                    return val;
                }
                _ => {
                    // For RAM/ROM/Banked, continue to check banking first
                }
            }
        }

        // 3. Banked regions (XL OS/BASIC ROM banking)
        if let Some(value) = self.bank_controller.read(addr) {
            return value;
        }

        // 4. Memory map (RAM/ROM)
        self.memory_map.read(addr).unwrap_or(0xFF)
    }

    fn write(&mut self, addr: u16, val: u8) {
        // 1. Cartridge ROM (ignore writes, not in memory map)
        if let Some(cart_rom) = &self.cart_rom {
            if addr >= self.cart_base && addr < self.cart_base + (cart_rom.len() as u16) {
                return;
            }
        }

        // 2. Check memory map for region type
        if let Some(region) = self.memory_map.find_region(addr) {
            match region {
                MemoryRegionType::Io(io_region) => {
                    // Route to appropriate chip based on region config
                    match io_region.chip_type {
                        ChipType::Gtia => {
                            self.gtia.write_register(addr, val);
                        }
                        ChipType::Pokey => {
                            // Special handling for IRQEN register ($D20E)
                            if (addr & 0x0F) == 0x0E {
                                self.irq_line = self.pokey.write_irqen(val);
                            } else {
                                self.pokey.write_register(addr, val);
                            }
                        }
                        ChipType::Pia => {
                            self.pia.write_register(addr, val);
                            // PORTB ($D301) controls banking on XL
                            if (addr & 0x03) == 0x01 {
                                let portb = self.pia.read_register(0xD301);
                                self.bank_controller.update_portb(portb);
                            }
                        }
                        ChipType::Antic => {
                            self.antic.write_register(addr, val);
                        }
                    }
                    return;
                }
                _ => {
                    // For RAM/ROM/Banked, continue to check banking first
                }
            }
        }

        // 3. Banked regions
        if self.bank_controller.write(addr, val) {
            return;
        }

        // 4. Memory map (RAM/ROM)
        self.memory_map.write(addr, val);
    }

    fn nmi_asserted(&self) -> bool {
        self.nmi_line
    }

    fn irq_asserted(&self) -> bool {
        self.irq_line
    }
}

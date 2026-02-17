use crate::bus::{Bus, ReadBus};
use crate::cpu::Cpu;
use crate::debugger::Debugger;
use crate::antic::Antic;
use crate::gtia::Gtia;
use crate::pokey::Pokey;
use crate::pia::Pia;
use crate::joystick::{JoystickInput, KeyboardJoystick};
use crate::memory_map::MemoryMap;
use crate::memory_region::{MemoryRegionType, RamRegion, RomRegion, IoRegion, ChipType};
use crate::banking::{BankController, BankingScheme};
use crate::rom_overlay::RomOverlayController;
use crate::machine_config::{MachineConfig, MachineType};
use crate::rom_scanner::RomDatabase;
use crate::sio::{SioController, SioDevice, SioTiming};
use crate::sio_bus::SioBus;
use crate::patch::PatchManager;

// ============================================================================
// MemorySystem - Extracted memory subsystem for clean borrow separation
// ============================================================================

/// Memory subsystem containing all memory-related components.
/// Implements ReadBus for immutable reads (ANTIC DMA) and Bus for mutable access.
pub struct MemorySystem {
    pub memory_map: MemoryMap,
    pub cart_rom: Option<Vec<u8>>,
    pub cart_base: u16,
    pub rom_overlay: RomOverlayController,
    pub bank_controller: BankController,
}

impl MemorySystem {
    pub fn new() -> Self {
        MemorySystem {
            memory_map: MemoryMap::new(),
            cart_rom: None,
            cart_base: 0xA000,
            rom_overlay: RomOverlayController::new(),
            bank_controller: BankController::new(BankingScheme::None),
        }
    }
}

/// ReadBus implementation for immutable memory reads (used by ANTIC for DMA)
impl ReadBus for MemorySystem {
    fn read(&self, addr: u16) -> u8 {
        // 1. Cartridge ROM (highest priority)
        if let Some(cart_rom) = &self.cart_rom {
            if addr >= self.cart_base && addr < self.cart_base + (cart_rom.len() as u16) {
                return cart_rom[(addr - self.cart_base) as usize];
            }
        }

        // 2. ROM overlays (800XL/130XE PORTB-controlled)
        if let Some(value) = self.rom_overlay.read(addr) {
            return value;
        }

        // 3. Banked RAM (130XE extended memory)
        if let Some(value) = self.bank_controller.read(addr) {
            return value;
        }

        // 4. Memory map (base RAM/ROM)
        self.memory_map.read(addr).unwrap_or(0xFF)
    }
}

// ============================================================================
// SystemBus - Wrapper combining memory and chip I/O for CPU execution
// ============================================================================

/// Combines MemorySystem with chip I/O references for full Bus implementation.
/// This allows CPU to access both memory and I/O chips without ownership conflicts.
struct SystemBus<'a> {
    memory: &'a mut MemorySystem,
    gtia: &'a mut Gtia,
    pokey: &'a mut Pokey,
    pia: &'a mut Pia,
    antic: &'a mut Antic,
    nmi_line: bool,
    sio_controller: &'a mut SioController,
}

impl Bus for SystemBus<'_> {
    fn read(&mut self, addr: u16) -> u8 {
        // Check for I/O regions first
        if let Some(region) = self.memory.memory_map.find_region(addr) {
            if let MemoryRegionType::Io(io_region) = region {
                return match io_region.chip_type {
                    ChipType::Gtia => self.gtia.read_register(addr),
                    ChipType::Pokey => self.pokey.read_register(addr),
                    ChipType::Pia => self.pia.read_register(addr),
                    ChipType::Antic => self.antic.read_register(addr),
                };
            }
        }

        // Use ReadBus implementation for memory access
        ReadBus::read(self.memory, addr)
    }

    fn write(&mut self, addr: u16, val: u8) {
        // 1. Cartridge ROM (ignore writes)
        if let Some(cart_rom) = &self.memory.cart_rom {
            if addr >= self.memory.cart_base && addr < self.memory.cart_base + (cart_rom.len() as u16) {
                return;
            }
        }

        // 2. Check for I/O regions
        if let Some(region) = self.memory.memory_map.find_region(addr) {
            if let MemoryRegionType::Io(io_region) = region {
                match io_region.chip_type {
                    ChipType::Gtia => {
                        self.gtia.write_register(addr, val);
                    }
                    ChipType::Pokey => {
                        if (addr & 0x0F) == 0x0E {
                            self.pokey.write_irqen(val);
                        } else {
                            self.pokey.write_register(addr, val);
                            // Forward SEROUT writes to SIO controller
                            if (addr & 0x0F) == 0x0D {
                                self.sio_controller.receive_byte(val);
                            }
                        }
                    }
                    ChipType::Pia => {
                        self.pia.write_register(addr, val);
                        // PORTB controls ROM overlays on 800XL/130XE
                        if (addr & 0x03) == 0x01 {
                            let portb = self.pia.get_portb();
                            self.memory.rom_overlay.update_portb(portb);
                        }
                    }
                    ChipType::Antic => {
                        self.antic.write_register(addr, val);
                    }
                }
                return;
            }
        }

        // 3. ROM overlay write blocking
        if self.memory.rom_overlay.is_write_blocked(addr) {
            return;
        }

        // 4. Banked RAM (130XE)
        if self.memory.bank_controller.write(addr, val) {
            return;
        }

        // 5. Memory map (base RAM)
        self.memory.memory_map.write(addr, val);
    }

    fn nmi_asserted(&self) -> bool {
        self.nmi_line
    }

    fn irq_asserted(&self) -> bool {
        self.pokey.irq_active() || self.pia.irq_active()
    }
}

pub struct Atari800 {
    // Core components
    cpu: Cpu,

    // Memory subsystem (contains memory_map, cart_rom, rom_overlay, bank_controller)
    pub memory: MemorySystem,
    config: MachineConfig,

    // Custom chips
    antic: Antic,
    pub gtia: Gtia,  // Public for SDL access to framebuffer
    pokey: Pokey,
    pia: Pia,

    // Input devices
    joystick: Box<dyn JoystickInput>,

    // Serial I/O
    sio_controller: SioController,
    sio_devices: Vec<Box<dyn SioDevice>>,
    sio_timing: SioTiming,

    // OS Patching (SIO patch, etc.)
    patch_manager: PatchManager,

    // Debugger
    debugger: Debugger,

    // Cycle tracking
    master_cycle: u64,
    cpu_halted: bool,

    // NMI line (6502 hardware interrupt line)
    nmi_line: bool,
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
        // Build memory system
        let mut memory = MemorySystem::new();

        // Base RAM
        memory.memory_map.add_region(MemoryRegionType::Ram(RamRegion::new(
            0x0000,
            0xFFFF,
            0,
        )));

        // I/O regions (overlay RAM)
        memory.memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD000,
            0xD0FF,
            ChipType::Gtia,
            100,
        )));
        memory.memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD200,
            0xD2FF,
            ChipType::Pokey,
            100,
        )));
        memory.memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD300,
            0xD3FF,
            ChipType::Pia,
            100,
        )));
        memory.memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD400,
            0xD4FF,
            ChipType::Antic,
            100,
        )));

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
            memory,
            config,
            antic: Antic::new(),
            gtia: Gtia::new(),
            pokey: Pokey::new(),
            pia: Pia::new(),
            joystick: Box::new(KeyboardJoystick::new()),
            sio_controller: SioController::new(),
            sio_devices: Vec::new(),
            sio_timing: SioTiming::default(),
            patch_manager: PatchManager::new(),  // No patches for render tests
            debugger: Debugger::new(),
            master_cycle: 0,
            cpu_halted: false,
            nmi_line: false,
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

        // Build memory system
        let mut memory = MemorySystem::new();
        memory.cart_rom = cart_rom;
        memory.cart_base = cart_base;
        memory.bank_controller = BankController::new(config.banking_scheme);

        // 1. Add RAM (entire address space up to ram_size, lowest priority)
        // Handle 64KB case: ram_size can be 65536, but u16 max is 65535
        let ram_end = if config.ram_size >= 65536 {
            0xFFFF
        } else {
            (config.ram_size - 1) as u16
        };
        memory.memory_map.add_region(MemoryRegionType::Ram(RamRegion::new(
            0x0000,
            ram_end,
            0,  // Lowest priority
        )));

        // 2. Add I/O regions (high priority, overlays RAM)
        memory.memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD000,
            0xD0FF,
            ChipType::Gtia,
            100,
        )));
        memory.memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD200,
            0xD2FF,
            ChipType::Pokey,
            100,
        )));
        memory.memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
            0xD300,
            0xD3FF,
            ChipType::Pia,
            100,
        )));
        memory.memory_map.add_region(MemoryRegionType::Io(IoRegion::new(
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

        // 4. Set up ROM overlays
        // Atari 800: Fixed OS ROM in memory map (no overlays)
        // Atari 800XL/130XE: ROM overlays controlled by PORTB
        match config.machine_type {
            MachineType::Atari800 => {
                // Add OS ROM as fixed ROM region (not an overlay)
                memory.memory_map.add_region(MemoryRegionType::Rom(RomRegion::new(
                    config.os_rom_start,
                    os_rom_data.clone(),
                    50,  // Higher priority than RAM, lower than I/O
                )));
            }
            MachineType::Atari800XL => {
                // 800XL OS ROM is 16KB but split into 3 sections:
                // - $0000-$0FFF (4KB) in file → $C000-$CFFF in memory (OS code)
                // - $1000-$1FFF (4KB) in file → $5000-$57FF in memory (self-test, via PORTB bit 7)
                // - $2000-$2FFF (8KB) in file → $D800-$FFFF in memory (OS code)
                // The middle section ($D000-$D7FF) is masked by I/O registers

                if os_rom_data.len() >= 0x4000 {
                    // 800XL OS ROM is 16KB with this layout in the file:
                    // $0000-$0FFF (4KB): OS code for $C000-$CFFF
                    // $1000-$17FF (2KB): Self-test code for $5000-$57FF (normally at $D000-$D7FF)
                    // $1800-$3FFF (10KB): OS code for $D800-$FFFF
                    let os_low = os_rom_data[0x0000..0x1000].to_vec();   // 4KB → $C000-$CFFF
                    let selftest = os_rom_data[0x1000..0x1800].to_vec(); // 2KB → $5000-$57FF
                    let os_high = os_rom_data[0x1800..0x4000].to_vec();  // 10KB → $D800-$FFFF

                    // Combine low and high for OS ROM overlay
                    let mut os_combined = os_low.clone();
                    os_combined.extend_from_slice(&vec![0xFF; 0x800]); // Pad $D000-$D7FF (I/O hole)
                    os_combined.extend_from_slice(&os_high);

                    memory.rom_overlay.set_os_rom(os_combined);
                    memory.rom_overlay.set_selftest_rom(selftest);
                } else {
                    eprintln!("Warning: 800XL OS ROM too small ({} bytes, expected 16KB)", os_rom_data.len());
                    memory.rom_overlay.set_os_rom(os_rom_data.clone());
                }

                if let Some(ref basic_path) = config.basic_rom_path {
                    let basic_data = std::fs::read(basic_path)
                        .unwrap_or_else(|e| {
                            eprintln!("Warning: Could not load BASIC ROM '{}': {}", basic_path, e);
                            eprintln!("Using empty ROM (BASIC will not work)");
                            vec![0xFF; config.basic_rom_size]
                        });
                    memory.rom_overlay.set_basic_rom(basic_data);
                }
            }
        }

        // Create system
        let mut atari800 = Atari800 {
            cpu: Cpu::new(),
            memory,
            config,
            antic: Antic::new(),
            gtia: Gtia::new(),
            pokey: Pokey::new(),
            pia: Pia::new(),
            joystick: Box::new(KeyboardJoystick::new()),
            sio_controller: SioController::new(),
            sio_devices: Vec::new(),
            sio_timing: SioTiming::default(),
            patch_manager: {
                let mut pm = PatchManager::new();
                crate::patch::register_standard_patches(&mut pm);
                pm
            },
            debugger: Debugger::new(),
            master_cycle: 0,
            cpu_halted: false,
            nmi_line: false,
        };

        // Set PORTB based on machine type
        // 800: PORTB controls joysticks (should be $FF for no input)
        // 800XL: PORTB controls ROM overlays (should be $FD for OS=on, BASIC=off, self-test=off)
        match atari800.config.machine_type {
            MachineType::Atari800 => {
                atari800.pia.set_portb(0xFF);  // No joystick input (active low)
            }
            MachineType::Atari800XL => {
                // Simulate OPTION held down during boot (BASIC disabled)
                atari800.pia.set_portb(0xFF);  // OS ROM on, BASIC disabled
                atari800.memory.rom_overlay.update_portb(0xFF);  // Initialize ROM overlay state
            }
        }

        // Reset CPU after construction to load PC from reset vector
        // Use SystemBus wrapper to avoid ownership conflicts
        {
            let mut bus = SystemBus {
                memory: &mut atari800.memory,
                gtia: &mut atari800.gtia,
                pokey: &mut atari800.pokey,
                pia: &mut atari800.pia,
                antic: &mut atari800.antic,
                nmi_line: atari800.nmi_line,
                sio_controller: &mut atari800.sio_controller,
            };
            atari800.cpu.reset(&mut bus);
        }

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

    // ========================================================================
    // SIO Methods
    // ========================================================================

    /// Add an SIO device (disk drive, printer, modem, etc.)
    pub fn add_sio_device(&mut self, device: Box<dyn SioDevice>) {
        self.sio_devices.push(device);
    }

    /// Enable SIO debug logging
    pub fn enable_sio_debug(&mut self) {
        self.sio_controller.enable_debug();
    }

    /// Tick the SIO controller for one machine cycle
    /// This uses the sequential borrowing pattern like tick() for the CPU
    /// SIO controller handles byte pacing internally - sends one byte per timing interval
    pub fn tick_sio(&mut self) {
        // Take temporary ownership of SioController (like we do with CPU)
        let mut sio = std::mem::take(&mut self.sio_controller);

        // Pass self as SioBus (like CPU uses self as Bus)
        // SIO tick() now handles byte pacing and sends bytes to POKEY internally
        sio.tick(self);

        // Return ownership
        self.sio_controller = sio;
    }

    // ========================================================================
    // Debug Methods
    // ========================================================================

    /// Enable GDB-style debug mode
    pub fn enable_debug_mode(&mut self) {
        let initial_pc = self.cpu.pc;
        self.debugger.enable_debug_mode(initial_pc);
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

        let mut cycles_this_scanline = 0;

        // Execute CPU for one scanline worth of cycles
        while cycles_this_scanline < CYCLES_PER_SCANLINE {
            // Check for OS patches before executing CPU instruction
            if self.patch_manager.has_patch(self.cpu.pc) {
                if self.check_and_execute_patch() {
                    cycles_this_scanline += 1;
                    continue;
                }
            }

            // Execute one CPU cycle using SystemBus wrapper
            {
                let mut bus = SystemBus {
                    memory: &mut self.memory,
                    gtia: &mut self.gtia,
                    pokey: &mut self.pokey,
                    pia: &mut self.pia,
                    antic: &mut self.antic,
                    nmi_line: self.nmi_line,
                    sio_controller: &mut self.sio_controller,
                };
                self.cpu.tick(&mut bus);
            }
            cycles_this_scanline += 1;

            // Tick POKEY for this cycle
            self.pokey.tick();

            // Tick SIO for this cycle
            self.tick_sio();
        }

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

        // Process all 240 visible NTSC scanlines through ANTIC display list
        // ANTIC reads directly from MemorySystem via ReadBus (no temp copy needed!)
        for scanline in 0..240 {
            self.antic.process_scanline(&self.memory);
            // Fetch PM graphics from memory via DMA (pass current scanline for correct offset)
            self.antic.fetch_pm_data(&self.memory, scanline);
            // Render the scanline with PM DMA data
            self.gtia.render_scanline(scanline, &self.antic.scanline_buffer, self.antic.get_current_mode(), Some(&self.antic.pm_data));
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
            // Trigger NMI on CPU using SystemBus wrapper
            let mut bus = SystemBus {
                memory: &mut self.memory,
                gtia: &mut self.gtia,
                pokey: &mut self.pokey,
                pia: &mut self.pia,
                antic: &mut self.antic,
                nmi_line: self.nmi_line,
                sio_controller: &mut self.sio_controller,
            };
            self.cpu.nmi(&mut bus);
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
        self.pokey.key_press(atari_key_code, shift, ctrl);
        // IRQ state will be checked directly via irq_asserted()
    }

    /// Handle keyboard key release event
    /// Clears the keyboard code register
    pub fn handle_key_release(&mut self) {
        self.pokey.key_release();
        // IRQ state will be checked directly via irq_asserted()
    }

    /// Handle Break key press (triggers POKEY IRQ bit 7)
    pub fn handle_break_key(&mut self) {
        self.pokey.break_key_press();
        // IRQ state will be checked directly via irq_asserted()
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

        // IMPORTANT: On 800XL, TRIG3 is connected to cartridge RD5 line (not joystick 4)
        // Only update TRIG3 for joystick 4 on Atari 800
        if matches!(self.config.machine_type, MachineType::Atari800) {
            self.gtia.set_trigger(3, joy4.trigger_value());
        }
        // On 800XL, TRIG3 remains 0 (no cartridge) or 1 (cartridge present) from init
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
        // Use ReadBus implementation on memory system
        ReadBus::read(&self.memory, addr)
    }
}

impl Bus for Atari800 {
    fn read(&mut self, addr: u16) -> u8 {
        // 1. Cartridge ROM (highest priority)
        if let Some(cart_rom) = &self.memory.cart_rom {
            if addr >= self.memory.cart_base && addr < self.memory.cart_base + (cart_rom.len() as u16) {
                return cart_rom[(addr - self.memory.cart_base) as usize];
            }
        }

        // 2. Check memory map for region type
        if let Some(region) = self.memory.memory_map.find_region(addr) {
            match region {
                MemoryRegionType::Io(io_region) => {
                    // Route to appropriate chip based on region config
                    let val = match io_region.chip_type {
                        ChipType::Gtia => self.gtia.read_register(addr),
                        ChipType::Pokey => self.pokey.read_register(addr),
                        ChipType::Pia => {
                            self.pia.read_register(addr)
                        }
                        ChipType::Antic => self.antic.read_register(addr),
                    };
                    return val;
                }
                _ => {
                    // For RAM/ROM/Banked, continue to check banking first
                }
            }
        }

        // 3. ROM overlays (800XL/130XE PORTB-controlled ROM overlays)
        if let Some(value) = self.memory.rom_overlay.read(addr) {
            return value;
        }

        // 4. Banked RAM (130XE extended memory)
        if let Some(value) = self.memory.bank_controller.read(addr) {
            return value;
        }

        // 5. Memory map (base RAM)
        self.memory.memory_map.read(addr).unwrap_or(0xFF)
    }

    fn write(&mut self, addr: u16, val: u8) {
        // 1. Cartridge ROM (ignore writes)
        if let Some(cart_rom) = &self.memory.cart_rom {
            if addr >= self.memory.cart_base && addr < self.memory.cart_base + (cart_rom.len() as u16) {
                return;
            }
        }

        // 2. Check memory map for region type
        if let Some(region) = self.memory.memory_map.find_region(addr) {
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
                                self.pokey.write_irqen(val);
                            } else {
                                self.pokey.write_register(addr, val);

                                // Forward SEROUT ($D20D) writes to SIO controller
                                if (addr & 0x0F) == 0x0D {
                                    let mut sio = std::mem::take(&mut self.sio_controller);
                                    sio.receive_byte(val);
                                    self.sio_controller = sio;
                                }
                            }
                        }
                        ChipType::Pia => {
                            self.pia.write_register(addr, val);
                            // PORTB controls ROM overlays on 800XL/130XE
                            if (addr & 0x03) == 0x01 {
                                let portb = self.pia.get_portb();
                                self.memory.rom_overlay.update_portb(portb);
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

        // 3. ROM overlay write blocking
        if self.memory.rom_overlay.is_write_blocked(addr) {
            return;
        }

        // 4. Banked RAM (130XE)
        if self.memory.bank_controller.write(addr, val) {
            return;
        }

        // 5. Memory map (base RAM)
        self.memory.memory_map.write(addr, val);
    }

    fn nmi_asserted(&self) -> bool {
        self.nmi_line
    }

    fn irq_asserted(&self) -> bool {
        self.pokey.irq_active() || self.pia.irq_active()
    }
}

// ============================================================================
// SIO Bus Implementation
// ============================================================================

impl SioBus for Atari800 {
    fn get_command_line(&self) -> bool {
        // Command line is controlled by PIA CB2 (Port B Control bit 2)
        // CB2 output state directly maps to SIO command line
        self.pia.get_cb2_output()
    }

    fn route_sio_command(&mut self, device_id: u8, cmd: u8, aux1: u8, aux2: u8) -> crate::sio::SioResponse {
        // Find matching device and route command
        for device in &mut self.sio_devices {
            if device.accepts_device_id(device_id) {
                return device.handle_command(cmd, aux1, aux2);
            }
        }

        // No device found - return NAK
        crate::sio::SioResponse::Nak
    }

    fn has_sio_device(&self, device_id: u8) -> bool {
        self.sio_devices.iter().any(|d| d.accepts_device_id(device_id))
    }

    fn send_byte_to_pokey(&mut self, byte: u8) {
        // Queue byte in POKEY SERIN register
        // This triggers serial input ready IRQ if enabled
        self.pokey.queue_serin_byte(byte);
    }

    fn get_sio_timing(&self) -> &SioTiming {
        &self.sio_timing
    }
}

// ============================================================================
// OS Patching Support
// ============================================================================

impl Atari800 {
    /// Check if there's a patch at the current PC and execute it
    /// Returns true if a patch was executed and handled the operation
    pub fn check_and_execute_patch(&mut self) -> bool {
        let pc = self.cpu.pc;

        if !self.patch_manager.has_patch(pc) {
            return false;
        }

        // Execute the appropriate patch based on PC
        match pc {
            0xE459 => {
                // SIOV patch - returns true if handled
                self.execute_siov_patch()
            }
            _ => false,
        }
    }

    /// Execute the SIOV patch directly
    /// This reads the DCB, performs disk I/O, and sets CPU state
    /// Returns true if the patch handled the operation, false if it should fall through
    fn execute_siov_patch(&mut self) -> bool {
        use crate::patch::{dcb, sio_status};

        // Read Device Control Block from memory
        let device = self.read(dcb::DDEVIC);
        let _unit = self.read(dcb::DUNIT);
        let command = self.read(dcb::DCOMND);
        let buffer = self.read_word(dcb::DBUFLO);
        let length = self.read_word(dcb::DBYTLO);
        let aux1 = self.read(dcb::DAUX1);
        let aux2 = self.read(dcb::DAUX2);
        let _sector = (aux2 as u16) << 8 | (aux1 as u16);

        // Only handle disk devices ($31-$38 = D1:-D8:)
        if device < 0x31 || device > 0x38 {
            // Not a disk device - let original code handle it
            return false;
        }

        // Find the disk device and perform operation
        let device_id = device;
        let mut status = 0u8;
        let mut data: Option<Vec<u8>> = None;

        for dev in &mut self.sio_devices {
            if dev.accepts_device_id(device_id) {
                let response = dev.handle_command(command, aux1, aux2);

                match response {
                    crate::sio::SioResponse::Complete => {
                        status = b'C';
                    }
                    crate::sio::SioResponse::CompleteWithData(d) => {
                        status = b'C';
                        data = Some(d);
                    }
                    crate::sio::SioResponse::Error => {
                        status = b'E';
                    }
                    crate::sio::SioResponse::ErrorWithData(d) => {
                        status = b'E';
                        data = Some(d);
                    }
                    crate::sio::SioResponse::Nak => {
                        status = b'N';
                    }
                    crate::sio::SioResponse::Ack => {
                        status = b'C';
                    }
                }
                break;
            }
        }

        // Copy data to buffer if operation returned data
        if let Some(bytes) = data {
            for (i, &byte) in bytes.iter().enumerate() {
                if i < length as usize {
                    self.write(buffer.wrapping_add(i as u16), byte);
                }
            }
        }

        // Set CPU state based on result
        match status {
            b'C' => {
                self.cpu.y = sio_status::SUCCESS;
                self.cpu.n = false;
            }
            b'E' => {
                self.cpu.y = sio_status::DERROR;
                self.cpu.n = true;
            }
            b'N' => {
                self.cpu.y = sio_status::DNACK;
                self.cpu.n = true;
            }
            _ => {
                self.cpu.y = sio_status::TIMEOUT;
                self.cpu.n = true;
            }
        }

        // Store status in DSTATS
        self.write(dcb::DSTATS, self.cpu.y);

        // Clear A, set carry (like reference)
        self.cpu.a = 0;
        self.cpu.c = true;

        // Return from subroutine (pull return address from stack, add 1)
        let s = self.cpu.s;
        let lo = self.read(0x0100 | (s.wrapping_add(1) as u16));
        let hi = self.read(0x0100 | (s.wrapping_add(2) as u16));
        self.cpu.s = s.wrapping_add(2);
        let ret_addr = ((hi as u16) << 8) | (lo as u16);
        self.cpu.pc = ret_addr.wrapping_add(1);

        true
    }

    /// Enable or disable the SIO patch
    pub fn set_sio_patch_enabled(&mut self, enabled: bool) {
        self.patch_manager.set_patch_enabled(0xE459, enabled);
    }

    /// Enable or disable all patches
    pub fn set_all_patches_enabled(&mut self, enabled: bool) {
        self.patch_manager.set_all_enabled(enabled);
    }
}

// ============================================================================
// Command API Support (power control, disk/cart management, etc.)
// ============================================================================

impl Atari800 {
    /// Power on the emulator (reset CPU)
    pub fn power_on(&mut self) {
        self.reset();
    }

    /// Power off the emulator (stop CPU execution)
    pub fn power_off(&mut self) {
        // Nothing to do - main loop handles the power state
    }

    /// Reset the CPU
    pub fn reset(&mut self) {
        let mut bus = SystemBus {
            memory: &mut self.memory,
            gtia: &mut self.gtia,
            pokey: &mut self.pokey,
            pia: &mut self.pia,
            antic: &mut self.antic,
            nmi_line: self.nmi_line,
            sio_controller: &mut self.sio_controller,
        };
        self.cpu.reset(&mut bus);
    }

    /// Get the current machine type
    pub fn get_machine_type(&self) -> MachineType {
        self.config.machine_type
    }

    /// Get paths of all attached disk drives
    /// Returns a Vec of (drive_number, path) tuples
    pub fn get_disk_paths(&self) -> Vec<(u8, String)> {
        let mut paths = Vec::new();
        for device in &self.sio_devices {
            let id = device.device_id();
            // Disk drives are $31-$38 (D1:-D8:)
            if id >= 0x31 && id <= 0x38 {
                let drive = id - 0x30;
                paths.push((drive, device.name()));
            }
        }
        paths
    }

    /// Get the path of a specific disk drive
    pub fn get_disk_path(&self, drive: u8) -> Option<String> {
        let device_id = 0x30 + drive;
        for device in &self.sio_devices {
            if device.device_id() == device_id {
                return Some(device.name());
            }
        }
        None
    }

    /// Eject a disk from a drive
    /// Returns true if a disk was ejected
    pub fn eject_disk(&mut self, drive: u8) -> bool {
        let device_id = 0x30 + drive;
        let initial_len = self.sio_devices.len();
        self.sio_devices.retain(|d| d.device_id() != device_id);
        self.sio_devices.len() < initial_len
    }

    /// Get the cartridge path (if any)
    pub fn get_cart_path(&self) -> Option<String> {
        if self.memory.cart_rom.is_some() {
            // We don't store the path, so just indicate a cart is loaded
            Some("(cartridge loaded)".to_string())
        } else {
            None
        }
    }

    /// Load a cartridge from file
    pub fn load_cart(&mut self, path: &str) -> Result<(), String> {
        let data = std::fs::read(path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let (rom, base) = if data.len() >= 16 && &data[0..4] == b"CART" {
            // CART format
            let cart_type = (data[4] as u32) << 24
                | (data[5] as u32) << 16
                | (data[6] as u32) << 8
                | data[7] as u32;
            let rom = data[16..].to_vec();
            let base = match cart_type {
                1 => {
                    if rom.len() != 0x2000 {
                        return Err(format!("CART type 1 expects 8KB ROM, got {}", rom.len()));
                    }
                    0xA000u16
                }
                2 => {
                    if rom.len() != 0x4000 {
                        return Err(format!("CART type 2 expects 16KB ROM, got {}", rom.len()));
                    }
                    0x8000u16
                }
                _ => {
                    return Err(format!(
                        "Unsupported CART type {} - only types 1 (8KB) and 2 (16KB) supported",
                        cart_type
                    ))
                }
            };
            (rom, base)
        } else {
            // Raw ROM - determine mapping from size
            let base = match data.len() {
                0x2000 => 0xA000u16,
                0x4000 => 0x8000u16,
                other => {
                    return Err(format!(
                        "Unsupported raw cartridge size: {} bytes (expected 8192 or 16384)",
                        other
                    ))
                }
            };
            (data, base)
        };

        self.memory.cart_rom = Some(rom);
        self.memory.cart_base = base;

        // Reset after loading cartridge
        self.reset();

        Ok(())
    }

    /// Eject the cartridge
    pub fn eject_cart(&mut self) {
        self.memory.cart_rom = None;
        self.memory.cart_base = 0xA000;
        self.reset();
    }
}

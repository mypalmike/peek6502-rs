/// OS Patching Facility
///
/// This module provides a general-purpose mechanism for intercepting CPU execution
/// at specific memory addresses and running custom handler code. This is commonly
/// used to accelerate or replace OS routines like SIO (Serial I/O).
///
/// ## How It Works
///
/// 1. Patches are registered with a target PC address
/// 2. Before each CPU instruction, the patch manager checks if PC matches any patch
/// 3. If matched, the patch handler runs and can modify CPU state
/// 4. The handler returns whether to skip the original instruction
///
/// ## Example: SIO Patch
///
/// The Atari OS calls SIOV at $E459 to perform serial I/O. By patching this address,
/// we can perform disk operations instantly without emulating the serial protocol.

use std::collections::HashMap;

/// Result of executing a patch handler
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PatchResult {
    /// Patch handled the operation, skip original instruction and continue at new PC
    Handled,

    /// Patch did not handle, execute original instruction normally
    NotHandled,

    /// Patch handled but wants to execute original instruction too (for logging/tracing)
    Continue,
}

/// Trait for accessing system state needed by patches
/// This abstracts the Bus so patches can read/write memory and access devices
pub trait PatchContext {
    // Memory access
    fn read_byte(&self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, val: u8);

    fn read_word(&self, addr: u16) -> u16 {
        let lo = self.read_byte(addr) as u16;
        let hi = self.read_byte(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    fn write_word(&mut self, addr: u16, val: u16) {
        self.write_byte(addr, val as u8);
        self.write_byte(addr.wrapping_add(1), (val >> 8) as u8);
    }

    // CPU state access
    fn get_pc(&self) -> u16;
    fn set_pc(&mut self, pc: u16);
    fn get_a(&self) -> u8;
    fn set_a(&mut self, a: u8);
    fn get_x(&self) -> u8;
    fn set_x(&mut self, x: u8);
    fn get_y(&self) -> u8;
    fn set_y(&mut self, y: u8);
    fn get_s(&self) -> u8;
    fn set_s(&mut self, s: u8);

    // Flag access
    fn get_n(&self) -> bool;
    fn set_n(&mut self, n: bool);
    fn get_z(&self) -> bool;
    fn set_z(&mut self, z: bool);
    fn get_c(&self) -> bool;
    fn set_c(&mut self, c: bool);
    fn get_v(&self) -> bool;
    fn set_v(&mut self, v: bool);

    // Return from subroutine (pull PC from stack, add 1)
    fn rts(&mut self) {
        let s = self.get_s();
        let lo = self.read_byte(0x0100 | (s.wrapping_add(1) as u16));
        let hi = self.read_byte(0x0100 | (s.wrapping_add(2) as u16));
        self.set_s(s.wrapping_add(2));
        let ret_addr = ((hi as u16) << 8) | (lo as u16);
        self.set_pc(ret_addr.wrapping_add(1));
    }
}

/// A patch handler function
/// Takes the patch context and returns whether the patch handled the call
pub type PatchHandler = fn(&mut dyn PatchContext, &mut dyn PatchEnv) -> PatchResult;

/// Environment providing access to emulator subsystems (disk, etc.)
/// Patches use this to perform actual operations
pub trait PatchEnv {
    /// Perform SIO disk operation
    /// Returns (status, data) where status is 'C' for complete, 'E' for error, etc.
    fn sio_disk_operation(&mut self, unit: u8, command: u8, sector: u16,
                          buffer_addr: u16, length: u16, aux1: u8, aux2: u8) -> (u8, Option<Vec<u8>>);
}

/// Information about a registered patch
#[derive(Clone)]
pub struct PatchInfo {
    /// Human-readable name for debugging
    pub name: String,

    /// The handler function
    pub handler: PatchHandler,

    /// Whether this patch is currently enabled
    pub enabled: bool,
}

/// Manages all registered patches
pub struct PatchManager {
    /// Patches indexed by target PC address
    patches: HashMap<u16, PatchInfo>,

    /// Global enable/disable for all patches
    enabled: bool,
}

impl Default for PatchManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PatchManager {
    pub fn new() -> Self {
        PatchManager {
            patches: HashMap::new(),
            enabled: true,
        }
    }

    /// Register a patch at a specific address
    pub fn register(&mut self, addr: u16, name: &str, handler: PatchHandler) {
        self.patches.insert(addr, PatchInfo {
            name: name.to_string(),
            handler,
            enabled: true,
        });
    }

    /// Unregister a patch
    pub fn unregister(&mut self, addr: u16) {
        self.patches.remove(&addr);
    }

    /// Enable or disable a specific patch
    pub fn set_patch_enabled(&mut self, addr: u16, enabled: bool) {
        if let Some(patch) = self.patches.get_mut(&addr) {
            patch.enabled = enabled;
        }
    }

    /// Enable or disable all patches globally
    pub fn set_all_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if there's an enabled patch at the given address
    pub fn has_patch(&self, addr: u16) -> bool {
        self.enabled && self.patches.get(&addr).map_or(false, |p| p.enabled)
    }

    /// Get patch info for an address (if any)
    pub fn get_patch(&self, addr: u16) -> Option<&PatchInfo> {
        if self.enabled {
            self.patches.get(&addr).filter(|p| p.enabled)
        } else {
            None
        }
    }

    /// Execute patch at address if one exists
    /// Returns Some(result) if patch was found and executed, None otherwise
    pub fn execute(&self, addr: u16, ctx: &mut dyn PatchContext, env: &mut dyn PatchEnv) -> Option<PatchResult> {
        if !self.enabled {
            return None;
        }

        if let Some(patch) = self.patches.get(&addr) {
            if patch.enabled {
                return Some((patch.handler)(ctx, env));
            }
        }

        None
    }

    /// List all registered patches (for debugging)
    pub fn list_patches(&self) -> Vec<(u16, String, bool)> {
        self.patches.iter()
            .map(|(addr, info)| (*addr, info.name.clone(), info.enabled))
            .collect()
    }
}

// ============================================================================
// Standard Atari OS Patches
// ============================================================================

/// Atari OS memory locations (Device Control Block)
pub mod dcb {
    pub const DDEVIC: u16 = 0x0300;  // Device ID
    pub const DUNIT: u16 = 0x0301;   // Unit number
    pub const DCOMND: u16 = 0x0302;  // Command
    pub const DSTATS: u16 = 0x0303;  // Status (direction before, result after)
    pub const DBUFLO: u16 = 0x0304;  // Buffer address low
    pub const DBUFHI: u16 = 0x0305;  // Buffer address high
    pub const DTIMLO: u16 = 0x0306;  // Timeout (low)
    pub const DBYTLO: u16 = 0x0308;  // Byte count low
    pub const DBYTHI: u16 = 0x0309;  // Byte count high
    pub const DAUX1: u16 = 0x030A;   // Aux byte 1 (sector low)
    pub const DAUX2: u16 = 0x030B;   // Aux byte 2 (sector high)
}

/// SIO status codes
pub mod sio_status {
    pub const SUCCESS: u8 = 0x01;    // Successful operation
    pub const TIMEOUT: u8 = 0x8A;    // Device timeout
    pub const DNACK: u8 = 0x8B;      // Device NAK
    pub const DERROR: u8 = 0x90;     // Device error
}

/// SIO command codes
pub mod sio_cmd {
    pub const STATUS: u8 = 0x53;     // 'S' - Get status
    pub const READ: u8 = 0x52;       // 'R' - Read sector
    pub const WRITE: u8 = 0x57;      // 'W' - Write sector (with verify)
    pub const PUT: u8 = 0x50;        // 'P' - Put sector (no verify)
    pub const FORMAT: u8 = 0x21;     // '!' - Format disk
    pub const FORMAT_ED: u8 = 0x22;  // '"' - Format enhanced density
}

/// SIOV patch handler - intercepts calls to the OS SIO routine
///
/// This reads the DCB from memory, performs the disk operation directly,
/// and sets CPU registers with the result, completely bypassing the
/// serial protocol.
pub fn siov_patch_handler(ctx: &mut dyn PatchContext, env: &mut dyn PatchEnv) -> PatchResult {
    // Read Device Control Block from memory
    let device = ctx.read_byte(dcb::DDEVIC);
    let unit = ctx.read_byte(dcb::DUNIT);
    let command = ctx.read_byte(dcb::DCOMND);
    let buffer = ctx.read_word(dcb::DBUFLO);
    let length = ctx.read_word(dcb::DBYTLO);
    let aux1 = ctx.read_byte(dcb::DAUX1);
    let aux2 = ctx.read_byte(dcb::DAUX2);
    let sector = (aux2 as u16) << 8 | (aux1 as u16);

    // Calculate unit number like the reference implementation
    // XL OS: LDA $0300 ADC $0301 ADC #$FF STA 023A
    let _unit_calc = device.wrapping_add(unit).wrapping_add(0xFF);

    // Only handle disk devices ($31-$38 = D1:-D8:)
    if device < 0x31 || device > 0x38 {
        // Not a disk device - let original code handle it
        return PatchResult::NotHandled;
    }

    // The disk unit is 0-based internally
    let disk_unit = device - 0x31;

    // Perform the operation via the environment
    let (status, data) = env.sio_disk_operation(disk_unit, command, sector, buffer, length, aux1, aux2);

    // Copy data to buffer if operation returned data
    if let Some(bytes) = data {
        for (i, &byte) in bytes.iter().enumerate() {
            if i < length as usize {
                ctx.write_byte(buffer.wrapping_add(i as u16), byte);
            }
        }
    }

    // Set CPU state based on result
    match status {
        b'C' => {
            // Complete - success
            ctx.set_y(sio_status::SUCCESS);
            ctx.set_n(false);
        }
        b'E' => {
            // Error
            ctx.set_y(sio_status::DERROR);
            ctx.set_n(true);
        }
        b'N' => {
            // NAK - device didn't acknowledge
            ctx.set_y(sio_status::DNACK);
            ctx.set_n(true);
        }
        0 => {
            // No device / timeout
            ctx.set_y(sio_status::TIMEOUT);
            ctx.set_n(true);
        }
        _ => {
            // Unknown status
            ctx.set_y(sio_status::DERROR);
            ctx.set_n(true);
        }
    }

    // Store status in DSTATS
    ctx.write_byte(dcb::DSTATS, ctx.get_y());

    // Clear A, set carry (like reference)
    ctx.set_a(0);
    ctx.set_c(true);

    // Return from subroutine
    ctx.rts();

    PatchResult::Handled
}

/// Register standard Atari OS patches
pub fn register_standard_patches(manager: &mut PatchManager) {
    // SIOV entry point is $E459 on all OS versions
    // Note: $E456 is CIOV (Central I/O), NOT SIOV - don't patch it!
    // CIOV uses IOCB at $0340, SIOV uses DCB at $0300
    manager.register(0xE459, "SIOV", siov_patch_handler);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockContext {
        memory: [u8; 65536],
        pc: u16,
        a: u8, x: u8, y: u8, s: u8,
        n: bool, z: bool, c: bool, v: bool,
    }

    impl MockContext {
        fn new() -> Self {
            MockContext {
                memory: [0; 65536],
                pc: 0, a: 0, x: 0, y: 0, s: 0xFF,
                n: false, z: false, c: false, v: false,
            }
        }
    }

    impl PatchContext for MockContext {
        fn read_byte(&self, addr: u16) -> u8 { self.memory[addr as usize] }
        fn write_byte(&mut self, addr: u16, val: u8) { self.memory[addr as usize] = val; }
        fn get_pc(&self) -> u16 { self.pc }
        fn set_pc(&mut self, pc: u16) { self.pc = pc; }
        fn get_a(&self) -> u8 { self.a }
        fn set_a(&mut self, a: u8) { self.a = a; }
        fn get_x(&self) -> u8 { self.x }
        fn set_x(&mut self, x: u8) { self.x = x; }
        fn get_y(&self) -> u8 { self.y }
        fn set_y(&mut self, y: u8) { self.y = y; }
        fn get_s(&self) -> u8 { self.s }
        fn set_s(&mut self, s: u8) { self.s = s; }
        fn get_n(&self) -> bool { self.n }
        fn set_n(&mut self, n: bool) { self.n = n; }
        fn get_z(&self) -> bool { self.z }
        fn set_z(&mut self, z: bool) { self.z = z; }
        fn get_c(&self) -> bool { self.c }
        fn set_c(&mut self, c: bool) { self.c = c; }
        fn get_v(&self) -> bool { self.v }
        fn set_v(&mut self, v: bool) { self.v = v; }
    }

    struct MockEnv;

    impl PatchEnv for MockEnv {
        fn sio_disk_operation(&mut self, _unit: u8, command: u8, _sector: u16,
                              _buffer: u16, _length: u16, _aux1: u8, _aux2: u8) -> (u8, Option<Vec<u8>>) {
            match command {
                0x53 => (b'C', Some(vec![0x00, 0xFF, 0xE0, 0x00])), // Status
                0x52 => (b'C', Some(vec![0; 128])), // Read
                _ => (b'N', None),
            }
        }
    }

    #[test]
    fn test_patch_registration() {
        let mut manager = PatchManager::new();

        fn dummy_handler(_: &mut dyn PatchContext, _: &mut dyn PatchEnv) -> PatchResult {
            PatchResult::Handled
        }

        manager.register(0x1234, "Test Patch", dummy_handler);

        assert!(manager.has_patch(0x1234));
        assert!(!manager.has_patch(0x5678));
    }

    #[test]
    fn test_patch_enable_disable() {
        let mut manager = PatchManager::new();

        fn dummy_handler(_: &mut dyn PatchContext, _: &mut dyn PatchEnv) -> PatchResult {
            PatchResult::Handled
        }

        manager.register(0x1234, "Test Patch", dummy_handler);
        assert!(manager.has_patch(0x1234));

        manager.set_patch_enabled(0x1234, false);
        assert!(!manager.has_patch(0x1234));

        manager.set_patch_enabled(0x1234, true);
        assert!(manager.has_patch(0x1234));

        manager.set_all_enabled(false);
        assert!(!manager.has_patch(0x1234));
    }

    #[test]
    fn test_siov_patch_status_command() {
        let mut ctx = MockContext::new();
        let mut env = MockEnv;

        // Set up DCB for Status command to D1:
        ctx.memory[0x0300] = 0x31;  // DDEVIC = $31 (D1:)
        ctx.memory[0x0301] = 0x01;  // DUNIT
        ctx.memory[0x0302] = 0x53;  // DCOMND = Status
        ctx.memory[0x0304] = 0x00;  // DBUFLO
        ctx.memory[0x0305] = 0x04;  // DBUFHI = $0400
        ctx.memory[0x0308] = 0x04;  // DBYTLO = 4
        ctx.memory[0x0309] = 0x00;  // DBYTHI

        // Set up return address on stack
        ctx.memory[0x01FF] = 0x00;  // Return address high
        ctx.memory[0x01FE] = 0x00;  // Return address low
        ctx.s = 0xFD;

        let result = siov_patch_handler(&mut ctx, &mut env);

        assert_eq!(result, PatchResult::Handled);
        assert_eq!(ctx.y, sio_status::SUCCESS);
        assert!(!ctx.n);
        assert!(ctx.c);

        // Check data was written to buffer
        assert_eq!(ctx.memory[0x0400], 0x00);
        assert_eq!(ctx.memory[0x0401], 0xFF);
        assert_eq!(ctx.memory[0x0402], 0xE0);
        assert_eq!(ctx.memory[0x0403], 0x00);
    }
}

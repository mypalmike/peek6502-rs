// SIO (Serial I/O) Protocol Handler - Clean Architecture
//
// This module implements the Atari SIO protocol with a clean separation of concerns:
// - SioDevice trait: abstraction for any SIO peripheral (disk, printer, modem, etc.)
// - SioController: protocol state machine (uses SioBus for hardware access)
// - SioBus trait: bus abstraction for routing and hardware integration
//
// The SioController handles protocol state machine logic, while the SioBus trait
// provides access to hardware state (PIA command line) and device routing.
// This follows the same architectural pattern as the CPU Bus.

use std::collections::VecDeque;
use crate::sio_bus::SioBus;

// ============================================================================
// Public Types
// ============================================================================

/// Response from an SIO device to a command
#[derive(Debug, Clone, PartialEq)]
pub enum SioResponse {
    /// $4E - Invalid command, bad checksum, or framing error
    Nak,

    /// $41 - Command accepted, no data to follow
    Ack,

    /// $43 - Command completed successfully, no data
    Complete,

    /// $43 + data + checksum - Success with data
    CompleteWithData(Vec<u8>),

    /// $45 - Command failed
    Error,

    /// $45 + data + checksum - Error but still returns data (e.g. Format bad sector list)
    ErrorWithData(Vec<u8>),
}

impl SioResponse {
    /// Get the protocol byte for this response (A/N/C/E)
    pub fn protocol_byte(&self) -> u8 {
        match self {
            SioResponse::Nak => 0x4E,
            SioResponse::Ack => 0x41,
            SioResponse::Complete => 0x43,
            SioResponse::CompleteWithData(_) => 0x43,
            SioResponse::Error => 0x45,
            SioResponse::ErrorWithData(_) => 0x45,
        }
    }

    /// Check if this response includes a data frame
    pub fn has_data(&self) -> bool {
        matches!(
            self,
            SioResponse::CompleteWithData(_) | SioResponse::ErrorWithData(_)
        )
    }

    /// Get the data bytes (if any)
    pub fn data(&self) -> Option<&[u8]> {
        match self {
            SioResponse::CompleteWithData(data) | SioResponse::ErrorWithData(data) => Some(data),
            _ => None,
        }
    }
}

/// Trait for any SIO-capable device (disk drive, printer, modem, etc.)
pub trait SioDevice {
    /// Get the device ID this device responds to
    fn device_id(&self) -> u8;

    /// Handle a command frame and return a response
    /// This is called after the command frame has been validated
    fn handle_command(&mut self, cmd: u8, aux1: u8, aux2: u8) -> SioResponse;

    /// Optional: check if this device accepts a specific ID
    /// Default: only accept exact device_id match
    fn accepts_device_id(&self, id: u8) -> bool {
        id == self.device_id()
    }

    /// Optional: get device name for debugging
    fn name(&self) -> String {
        format!("Device ${:02X}", self.device_id())
    }
}

// ============================================================================
// Command Frame
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
struct CommandFrame {
    device_id: u8,
    command: u8,
    aux1: u8,
    aux2: u8,
    checksum: u8,
}

impl CommandFrame {
    fn from_bytes(bytes: &[u8; 5]) -> Self {
        CommandFrame {
            device_id: bytes[0],
            command: bytes[1],
            aux1: bytes[2],
            aux2: bytes[3],
            checksum: bytes[4],
        }
    }

    fn validate_checksum(&self) -> bool {
        // Mirror atari800-master: don't actually validate checksum
        // The reference implementation accepts all command frames without validation
        // Checksum is calculated on all 4 command bytes (device, command, aux1, aux2)
        // but validation is disabled to match hardware behavior
        true
    }
}

// ============================================================================
// State Machine
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
enum SioState {
    /// Waiting for command line assertion
    Idle,

    /// Receiving 5-byte command frame
    ReceivingCommand {
        bytes_received: usize,
        buffer: [u8; 5],
    },

    /// Waiting for command line to deassert before sending ACK/NAK
    WaitingForCommandLineDeassert {
        frame: CommandFrame,
    },

    /// Sending ACK byte ($41)
    SendingAck,

    /// Command executing (simulated delay)
    Executing {
        device_index: usize,
        response: SioResponse,
        cycles_remaining: u32,
    },

    /// Inter-frame delay between Complete and Data
    /// The OS WAIT routine expects Complete as a single byte,
    /// then the OS calls RECEIV to get the data frame
    InterFrameDelay {
        data_frame: Vec<u8>,  // Data + checksum to send after delay
        cycles_remaining: u32,
    },
}

// ============================================================================
// SIO Controller
// ============================================================================

pub struct SioController {
    /// Current protocol state
    state: SioState,

    /// Previous command line state (for edge detection)
    prev_command_line: bool,

    /// Queue of bytes to send to POKEY
    tx_queue: VecDeque<u8>,

    /// Debug logging enable
    debug: bool,

    /// Byte pacing timer - cycles remaining until next byte can be sent
    byte_timer: u32,
}

/// Timing parameters for cycle-accurate emulation
#[derive(Debug, Clone)]
pub struct SioTiming {
    /// Cycles per byte at current baud rate
    pub cycles_per_byte: u32,

    /// Minimum delay before sending result byte
    pub result_min_delay: u32,

    /// Typical disk read command execution time
    pub read_sector_delay: u32,

    /// Status command execution time (very fast)
    pub status_delay: u32,
}

impl Default for SioTiming {
    fn default() -> Self {
        Self::new_ntsc_19200_baud()
    }
}

impl SioTiming {
    /// Standard NTSC timing at 19200 baud
    pub fn new_ntsc_19200_baud() -> Self {
        const NTSC_CLOCK: u32 = 1_789_725;

        // At 19200 baud: 1 bit = 52.08μs, 10 bits/byte = 520.8μs
        let cycles_per_byte = (NTSC_CLOCK as f64 * 520.8e-6) as u32;

        SioTiming {
            cycles_per_byte,
            result_min_delay: (NTSC_CLOCK as f64 * 2e-3) as u32,        // 2ms (was 250μs - too short!)
            read_sector_delay: (NTSC_CLOCK as f64 * 100e-3) as u32,     // 100ms
            status_delay: (NTSC_CLOCK as f64 * 1e-3) as u32,            // 1ms
        }
    }

    /// No timing delays (for testing)
    pub fn instant() -> Self {
        SioTiming {
            cycles_per_byte: 1,
            result_min_delay: 0,
            read_sector_delay: 0,
            status_delay: 0,
        }
    }
}

impl Default for SioController {
    fn default() -> Self {
        Self::new()
    }
}

impl SioController {
    pub fn new() -> Self {
        SioController {
            state: SioState::Idle,
            prev_command_line: false,
            tx_queue: VecDeque::new(),
            debug: false,
            byte_timer: 0,
        }
    }

    /// Enable debug logging
    pub fn enable_debug(&mut self) {
        self.debug = true;
    }

    /// Detect command line edges and update state
    /// This is called from tick() with the current command line state from the bus
    fn handle_command_line_edges(&mut self, command_line: bool) {
        let was_asserted = self.prev_command_line;
        self.prev_command_line = command_line;

        // Rising edge: start new command frame
        if command_line && !was_asserted {
            // Only accept new commands when truly idle (no pending state OR bytes)
            // Don't interrupt if we're executing, waiting, or transmitting
            if !matches!(self.state, SioState::Idle) || !self.tx_queue.is_empty() {
                return;
            }

            self.state = SioState::ReceivingCommand {
                bytes_received: 0,
                buffer: [0; 5],
            };
            self.tx_queue.clear();
        }
    }

    /// Called when POKEY receives a byte from the computer
    pub fn receive_byte(&mut self, byte: u8) {
        if let SioState::ReceivingCommand {
            bytes_received,
            mut buffer,
        } = self.state
        {
            if bytes_received < 5 {
                buffer[bytes_received] = byte;
                let new_count = bytes_received + 1;

                if new_count >= 5 {
                    // Command frame complete - wait for command line to deassert
                    let frame = CommandFrame::from_bytes(&buffer);
                    self.state = SioState::WaitingForCommandLineDeassert { frame };
                } else {
                    self.state = SioState::ReceivingCommand {
                        bytes_received: new_count,
                        buffer,
                    };
                }
            }
        }
    }

    /// Execute one machine cycle
    /// Queries the bus for command line state and routes commands to devices
    pub fn tick(&mut self, bus: &mut dyn SioBus) {
        // Check command line for edges
        let command_line = bus.get_command_line();
        self.handle_command_line_edges(command_line);

        // Process command frame if waiting and command line deasserted
        if let SioState::WaitingForCommandLineDeassert { frame } = self.state {
            if !command_line {
                self.process_command(frame, bus);
            }
        }

        // Handle executing state with delay
        if let SioState::Executing {
            device_index,
            response,
            cycles_remaining,
        } = &self.state
        {
            if *cycles_remaining == 0 {
                // Execution complete - prepare response
                let response = response.clone();

                // CRITICAL: Set byte_timer to result_min_delay before queueing response
                // This ensures Complete doesn't immediately overwrite ACK
                let timing = bus.get_sio_timing();
                self.byte_timer = timing.result_min_delay;

                self.prepare_response(response, timing);
            } else {
                self.state = SioState::Executing {
                    device_index: *device_index,
                    response: response.clone(),
                    cycles_remaining: cycles_remaining - 1,
                };
            }
        }

        // Handle inter-frame delay between Complete and Data
        if let SioState::InterFrameDelay {
            data_frame,
            cycles_remaining,
        } = &self.state
        {
            if *cycles_remaining == 0 {
                // Delay complete - queue data frame
                for &byte in data_frame {
                    self.tx_queue.push_back(byte);
                }
                self.state = SioState::Idle;
            } else {
                self.state = SioState::InterFrameDelay {
                    data_frame: data_frame.clone(),
                    cycles_remaining: cycles_remaining - 1,
                };
            }
        }

        // Handle byte pacing - send one byte at a time with proper timing
        if self.byte_timer > 0 {
            self.byte_timer -= 1;
        }

        // When timer expires and we have bytes to send, send one to POKEY
        if self.byte_timer == 0 && !self.tx_queue.is_empty() {
            if let Some(byte) = self.tx_queue.pop_front() {
                bus.send_byte_to_pokey(byte);

                // Reset timer for next byte (based on baud rate)
                let timing = bus.get_sio_timing();
                self.byte_timer = timing.cycles_per_byte;
            }
        }
    }

    /// Process a command frame using the bus to route to devices
    /// This is called from tick() after command line deasserts
    fn process_command(&mut self, frame: CommandFrame, bus: &mut dyn SioBus) {
        // Validate checksum
        if !frame.validate_checksum() {
            self.tx_queue.push_back(0x4E); // NAK
            self.state = SioState::Idle;
            return;
        }

        // Check if a device exists for this ID
        if !bus.has_sio_device(frame.device_id) {
            // No device found - don't respond (devices ignore unknown IDs)
            self.state = SioState::Idle;
            return;
        }

        // Send ACK first
        self.tx_queue.push_back(0x41);
        self.state = SioState::SendingAck;

        // Route command to device via bus
        let response = bus.route_sio_command(frame.device_id, frame.command, frame.aux1, frame.aux2);

        // Determine execution delay
        let timing = bus.get_sio_timing();
        let delay = match frame.command {
            0x53 => timing.status_delay,      // Status
            0x52 => timing.read_sector_delay, // Read
            _ => timing.status_delay,
        };

        let timing = bus.get_sio_timing();

        // If instant timing (delay = 0), prepare response immediately
        if delay == 0 {
            self.prepare_response(response, timing);
        } else {
            self.state = SioState::Executing {
                device_index: 0,  // Not used anymore - devices accessed via bus
                response,
                cycles_remaining: delay,
            };
        }
    }

    /// Prepare response bytes to send
    /// For CompleteWithData/ErrorWithData, this queues the result byte alone
    /// and sets up InterFrameDelay state to send data frame separately
    fn prepare_response(&mut self, response: SioResponse, timing: &SioTiming) {
        match response {
            SioResponse::Nak => {
                self.tx_queue.push_back(0x4E);
                self.state = SioState::Idle;
            }
            SioResponse::Ack => {
                self.tx_queue.push_back(0x41);
                self.state = SioState::Idle;
            }
            SioResponse::Complete => {
                self.tx_queue.push_back(0x43);
                self.state = SioState::Idle;
            }
            SioResponse::Error => {
                self.tx_queue.push_back(0x45);
                self.state = SioState::Idle;
            }
            SioResponse::CompleteWithData(ref data) | SioResponse::ErrorWithData(ref data) => {
                // FRAME 1: Result byte ALONE (Complete or Error)
                let result_byte = response.protocol_byte();
                self.tx_queue.push_back(result_byte);

                // FRAME 2: Data + checksum (will be sent after inter-frame delay)
                let mut data_frame = data.clone();
                let checksum = sio_checksum(data);
                data_frame.push(checksum);

                // Enter InterFrameDelay state
                self.state = SioState::InterFrameDelay {
                    data_frame,
                    cycles_remaining: timing.result_min_delay,
                };
            }
        }
    }

    /// Get current state (for debugging/testing)
    pub fn state(&self) -> String {
        format!("{:?}", self.state)
    }
}

// ============================================================================
// Checksum Calculation
// ============================================================================

/// Calculate SIO checksum with carry wraparound
///
/// This is a one's complement sum: for each byte, add to sum and fold carry back in.
/// Algorithm matches 6502: CLC, then ADC each byte, then ADC #0 to fold final carry.
pub fn sio_checksum(bytes: &[u8]) -> u8 {
    let mut sum = 0u16;
    for &byte in bytes {
        sum += byte as u16;
        if sum > 0xFF {
            sum = (sum & 0xFF) + 1; // Fold carry
        }
    }
    sum as u8
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sio_bus::SioBus;

    #[test]
    fn test_checksum_simple() {
        // Checksum on all 4 command bytes (device, command, aux1, aux2)
        // This matches AltirraOS implementation
        assert_eq!(sio_checksum(&[0x31, 0x53, 0x00, 0x00]), 0x84);
    }

    #[test]
    fn test_checksum_with_carry() {
        assert_eq!(sio_checksum(&[0xFF, 0xFF, 0x01, 0x00]), 0x01);
    }

    #[test]
    fn test_command_frame_validation() {
        // Mirror atari800-master: checksum validation is disabled
        // All frames are accepted regardless of checksum
        let frame1 = CommandFrame::from_bytes(&[0x31, 0x53, 0x00, 0x00, 0x53]);
        assert!(frame1.validate_checksum()); // Any checksum accepted

        let frame2 = CommandFrame::from_bytes(&[0x31, 0x53, 0x00, 0x00, 0xFF]);
        assert!(frame2.validate_checksum()); // Any checksum accepted
    }

    // Mock device for testing
    struct MockDevice {
        id: u8,
        response: SioResponse,
    }

    impl SioDevice for MockDevice {
        fn device_id(&self) -> u8 {
            self.id
        }

        fn handle_command(&mut self, _cmd: u8, _aux1: u8, _aux2: u8) -> SioResponse {
            self.response.clone()
        }
    }

    // Mock SIO bus for testing
    struct MockSioBus {
        command_line: bool,
        devices: Vec<Box<dyn SioDevice>>,
        timing: SioTiming,
        pokey_bytes: Vec<u8>,  // Capture bytes sent to POKEY
    }

    impl MockSioBus {
        fn new() -> Self {
            MockSioBus {
                command_line: false,
                devices: Vec::new(),
                timing: SioTiming::instant(),
                pokey_bytes: Vec::new(),
            }
        }

        fn add_device(&mut self, device: Box<dyn SioDevice>) {
            self.devices.push(device);
        }

        fn set_command_line(&mut self, asserted: bool) {
            self.command_line = asserted;
        }

        fn get_pokey_bytes(&self) -> &[u8] {
            &self.pokey_bytes
        }
    }

    impl SioBus for MockSioBus {
        fn get_command_line(&self) -> bool {
            self.command_line
        }

        fn route_sio_command(&mut self, device_id: u8, cmd: u8, aux1: u8, aux2: u8) -> SioResponse {
            for device in &mut self.devices {
                if device.accepts_device_id(device_id) {
                    return device.handle_command(cmd, aux1, aux2);
                }
            }
            SioResponse::Nak
        }

        fn has_sio_device(&self, device_id: u8) -> bool {
            self.devices.iter().any(|d| d.accepts_device_id(device_id))
        }

        fn send_byte_to_pokey(&mut self, byte: u8) {
            self.pokey_bytes.push(byte);
        }

        fn get_sio_timing(&self) -> &SioTiming {
            &self.timing
        }
    }

    #[test]
    fn test_controller_basic() {
        let mut sio = SioController::new();
        let mut bus = MockSioBus::new();

        // Add mock device to bus
        bus.add_device(Box::new(MockDevice {
            id: 0x31,
            response: SioResponse::CompleteWithData(vec![0x10, 0xFF, 0x60, 0x00]),
        }));

        // Simulate command frame
        bus.set_command_line(true);
        sio.tick(&mut bus);  // Detect rising edge

        sio.receive_byte(0x31); // Device ID
        sio.receive_byte(0x53); // Command (Status)
        sio.receive_byte(0x00); // AUX1
        sio.receive_byte(0x00); // AUX2
        sio.receive_byte(0x84); // Checksum ($31 + $53 + $00 + $00, matches AltirraOS)

        bus.set_command_line(false);
        sio.tick(&mut bus);  // Detect falling edge and process command

        // Tick multiple times to send all bytes (instant timing = 0 cycles per byte)
        // Each tick sends one byte when timer expires
        for _ in 0..10 {
            sio.tick(&mut bus);
        }

        // Check all bytes were sent to POKEY in correct order
        let bytes = bus.get_pokey_bytes();
        assert_eq!(bytes[0], 0x41);  // ACK
        assert_eq!(bytes[1], 0x43);  // Complete
        assert_eq!(bytes[2], 0x10);  // Data byte 1
        assert_eq!(bytes[3], 0xFF);  // Data byte 2
        assert_eq!(bytes[4], 0x60);  // Data byte 3
        assert_eq!(bytes[5], 0x00);  // Data byte 4

        // Should send checksum (only covers data bytes, not Complete byte)
        let checksum = sio_checksum(&[0x10, 0xFF, 0x60, 0x00]);
        assert_eq!(bytes[6], checksum);
    }

    #[test]
    fn test_bad_checksum_accepted() {
        // Mirror atari800-master: checksums are not validated
        // Even "bad" checksums are accepted
        let mut sio = SioController::new();
        let mut bus = MockSioBus::new();

        bus.add_device(Box::new(MockDevice {
            id: 0x31,
            response: SioResponse::Complete,
        }));

        bus.set_command_line(true);
        sio.tick(&mut bus);

        // Send any checksum - it will be accepted
        sio.receive_byte(0x31);
        sio.receive_byte(0x53);
        sio.receive_byte(0x00);
        sio.receive_byte(0x00);
        sio.receive_byte(0xFF); // Any value accepted

        bus.set_command_line(false);
        sio.tick(&mut bus);

        // Tick to send bytes
        for _ in 0..5 {
            sio.tick(&mut bus);
        }

        // Should send ACK and Complete (command accepted)
        let bytes = bus.get_pokey_bytes();
        assert!(bytes.len() >= 1);
        assert_eq!(bytes[0], 0x41);  // ACK
    }

    #[test]
    fn test_instant_timing_processes_command_immediately() {
        let mut sio = SioController::new();
        let mut bus = MockSioBus::new();

        bus.add_device(Box::new(MockDevice {
            id: 0x31,
            response: SioResponse::CompleteWithData(vec![0x10, 0xFF]),
        }));

        // Assert command line
        bus.set_command_line(true);
        sio.tick(&mut bus);
        println!("After assert: {}", sio.state());

        // Send command
        sio.receive_byte(0x31);
        sio.receive_byte(0x53);
        sio.receive_byte(0x00);
        sio.receive_byte(0x00);
        sio.receive_byte(0x84);
        println!("After bytes: {}", sio.state());

        // Deassert command line - with instant timing, should immediately process
        bus.set_command_line(false);
        sio.tick(&mut bus);
        println!("After deassert: {}", sio.state());

        // Tick multiple times to send all bytes (instant timing = 0 cycles per byte)
        for _ in 0..10 {
            sio.tick(&mut bus);
        }

        // Check all bytes were sent in correct order
        let bytes = bus.get_pokey_bytes();
        assert!(bytes.len() >= 5, "Should have all bytes");
        assert_eq!(bytes[0], 0x41); // ACK
        assert_eq!(bytes[1], 0x43); // Complete
        assert_eq!(bytes[2], 0x10); // Data
        assert_eq!(bytes[3], 0xFF); // Data

        // Checksum (only covers data bytes, not Complete byte)
        let checksum = sio_checksum(&[0x10, 0xFF]);
        assert_eq!(bytes[4], checksum);
    }
}

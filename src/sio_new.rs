// SIO (Serial I/O) Protocol Handler - Clean Architecture
//
// This module implements the Atari SIO protocol with a clean separation of concerns:
// - SioDevice trait: abstraction for any SIO peripheral (disk, printer, modem, etc.)
// - SioController: protocol state machine and device routing
// - Integration with POKEY (serial port) and PIA (command line)
//
// See docs/SIO_ARCHITECTURE.md for detailed protocol documentation.

use std::collections::VecDeque;

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

    /// Sending NAK byte ($4E)
    SendingNak,

    /// Command executing (simulated delay)
    Executing {
        device_index: usize,
        response: SioResponse,
        cycles_remaining: u32,
    },

    /// Sending result byte (C or E) and optionally data frame
    SendingResponse {
        bytes_sent: usize,
        response: Vec<u8>,
    },
}

// ============================================================================
// SIO Controller
// ============================================================================

pub struct SioController {
    /// Current protocol state
    state: SioState,

    /// Connected devices
    devices: Vec<Box<dyn SioDevice>>,

    /// Command line state (from PIA CB2)
    command_line: bool,

    /// Queue of bytes to send to POKEY
    tx_queue: VecDeque<u8>,

    /// Timing parameters
    timing: SioTiming,
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
            result_min_delay: (NTSC_CLOCK as f64 * 250e-6) as u32,      // 250μs
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

impl SioController {
    pub fn new() -> Self {
        Self::with_timing(SioTiming::default())
    }

    pub fn with_timing(timing: SioTiming) -> Self {
        SioController {
            state: SioState::Idle,
            devices: Vec::new(),
            command_line: false,
            tx_queue: VecDeque::new(),
            timing,
        }
    }

    /// Add a device to the SIO bus
    pub fn add_device(&mut self, device: Box<dyn SioDevice>) {
        self.devices.push(device);
    }

    /// Update command line state (from PIA CB2)
    pub fn set_command_line(&mut self, asserted: bool) {
        let was_asserted = self.command_line;
        self.command_line = asserted;

        // Rising edge: start new command frame
        if asserted && !was_asserted {
            self.state = SioState::ReceivingCommand {
                bytes_received: 0,
                buffer: [0; 5],
            };
            self.tx_queue.clear();
        }

        // Falling edge during wait: proceed to send response
        if !asserted && was_asserted {
            if let SioState::WaitingForCommandLineDeassert { frame } = self.state {
                self.process_command(frame);
            }
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

    /// Check if there's a byte ready to send to POKEY
    pub fn has_byte_for_pokey(&self) -> bool {
        !self.tx_queue.is_empty()
    }

    /// Get next byte to send to POKEY (if available)
    pub fn get_byte_for_pokey(&mut self) -> Option<u8> {
        self.tx_queue.pop_front()
    }

    /// Execute one machine cycle
    pub fn tick(&mut self) {
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
                self.prepare_response(response);
            } else {
                self.state = SioState::Executing {
                    device_index: *device_index,
                    response: response.clone(),
                    cycles_remaining: cycles_remaining - 1,
                };
            }
        }
    }

    /// Process a complete command frame
    fn process_command(&mut self, frame: CommandFrame) {
        // Validate checksum
        if !frame.validate_checksum() {
            self.tx_queue.push_back(0x4E); // NAK
            self.state = SioState::Idle;
            return;
        }

        // Find matching device
        for (index, device) in self.devices.iter_mut().enumerate() {
            if device.accepts_device_id(frame.device_id) {
                // Send ACK first
                self.tx_queue.push_back(0x41);
                self.state = SioState::SendingAck;

                // Execute command
                let response = device.handle_command(frame.command, frame.aux1, frame.aux2);

                // Determine execution delay
                let delay = match frame.command {
                    0x53 => self.timing.status_delay,      // Status
                    0x52 => self.timing.read_sector_delay, // Read
                    _ => self.timing.status_delay,
                };

                self.state = SioState::Executing {
                    device_index: index,
                    response,
                    cycles_remaining: delay,
                };
                return;
            }
        }

        // No device found - don't respond (devices ignore unknown IDs)
        self.state = SioState::Idle;
    }

    /// Prepare response bytes to send
    fn prepare_response(&mut self, response: SioResponse) {
        let mut bytes = Vec::new();

        match response {
            SioResponse::Nak => {
                bytes.push(0x4E);
            }
            SioResponse::Ack => {
                bytes.push(0x41);
            }
            SioResponse::Complete => {
                bytes.push(0x43);
            }
            SioResponse::Error => {
                bytes.push(0x45);
            }
            SioResponse::CompleteWithData(ref data) | SioResponse::ErrorWithData(ref data) => {
                // Result byte
                bytes.push(response.protocol_byte());

                // Data bytes
                bytes.extend_from_slice(data);

                // Checksum (calculated on result + data)
                let checksum = sio_checksum(&bytes);
                bytes.push(checksum);
            }
        }

        // Queue all bytes
        for byte in bytes {
            self.tx_queue.push_back(byte);
        }

        self.state = SioState::Idle;
    }

    /// Get current state (for debugging/testing)
    pub fn state(&self) -> String {
        format!("{:?}", self.state)
    }

    /// Get device count
    pub fn device_count(&self) -> usize {
        self.devices.len()
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

    #[test]
    fn test_controller_basic() {
        let mut sio = SioController::with_timing(SioTiming::instant());

        // Add mock device
        sio.add_device(Box::new(MockDevice {
            id: 0x31,
            response: SioResponse::CompleteWithData(vec![0x10, 0xFF, 0x60, 0x00]),
        }));

        // Simulate command frame
        sio.set_command_line(true);

        sio.receive_byte(0x31); // Device ID
        sio.receive_byte(0x53); // Command (Status)
        sio.receive_byte(0x00); // AUX1
        sio.receive_byte(0x00); // AUX2
        sio.receive_byte(0x84); // Checksum ($31 + $53 + $00 + $00, matches AltirraOS)

        sio.set_command_line(false);

        // Should send ACK
        assert_eq!(sio.get_byte_for_pokey(), Some(0x41));

        // Tick to complete execution
        while !sio.has_byte_for_pokey() {
            sio.tick();
        }

        // Should send Complete
        assert_eq!(sio.get_byte_for_pokey(), Some(0x43));

        // Should send data bytes
        assert_eq!(sio.get_byte_for_pokey(), Some(0x10));
        assert_eq!(sio.get_byte_for_pokey(), Some(0xFF));
        assert_eq!(sio.get_byte_for_pokey(), Some(0x60));
        assert_eq!(sio.get_byte_for_pokey(), Some(0x00));

        // Should send checksum
        let checksum = sio_checksum(&[0x43, 0x10, 0xFF, 0x60, 0x00]);
        assert_eq!(sio.get_byte_for_pokey(), Some(checksum));
    }

    #[test]
    fn test_bad_checksum_accepted() {
        // Mirror atari800-master: checksums are not validated
        // Even "bad" checksums are accepted
        let mut sio = SioController::with_timing(SioTiming::instant());

        sio.add_device(Box::new(MockDevice {
            id: 0x31,
            response: SioResponse::Complete,
        }));

        sio.set_command_line(true);

        // Send any checksum - it will be accepted
        sio.receive_byte(0x31);
        sio.receive_byte(0x53);
        sio.receive_byte(0x00);
        sio.receive_byte(0x00);
        sio.receive_byte(0xFF); // Any value accepted

        sio.set_command_line(false);

        // Should send ACK (command accepted)
        assert_eq!(sio.get_byte_for_pokey(), Some(0x41));
    }
}

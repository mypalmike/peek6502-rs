// Unit tests for SIO (Serial I/O) Protocol
//
// These tests verify the SIO protocol implementation without requiring
// a full emulator. They test:
// - Checksum calculation
// - Command frame parsing
// - State machine transitions
// - Device routing
// - Protocol timing

// ============================================================================
// Test 1: Checksum Calculation
// ============================================================================

/// SIO uses a carry wrap-around checksum (also called one's complement sum).
/// Algorithm: sum = 0; for each byte: sum += byte; if carry: sum = (sum & 0xFF) + 1
fn sio_checksum(bytes: &[u8]) -> u8 {
    let mut sum = 0u16;
    for &byte in bytes {
        sum += byte as u16;
        if sum > 0xFF {
            sum = (sum & 0xFF) + 1;
        }
    }
    sum as u8
}

#[test]
fn test_checksum_simple() {
    // D1: Status command: $31 $53 $00 $00
    // Expected: $31 + $53 + $00 + $00 = $84
    assert_eq!(sio_checksum(&[0x31, 0x53, 0x00, 0x00]), 0x84);
}

#[test]
fn test_checksum_with_single_carry() {
    // Test with one carry
    // $80 + $90 = $110 -> $10 + 1 = $11
    assert_eq!(sio_checksum(&[0x80, 0x90]), 0x11);
}

#[test]
fn test_checksum_with_multiple_carries() {
    // $FF + $FF = $1FE -> $FE + 1 = $FF
    // $FF + $01 = $100 -> $00 + 1 = $01
    // $01 + $00 = $01
    assert_eq!(sio_checksum(&[0xFF, 0xFF, 0x01, 0x00]), 0x01);
}

#[test]
fn test_checksum_all_zeros() {
    assert_eq!(sio_checksum(&[0x00, 0x00, 0x00, 0x00]), 0x00);
}

#[test]
fn test_checksum_all_ff() {
    // $FF + $FF = $1FE -> $FE + 1 = $FF
    // $FF + $FF = $1FE -> $FE + 1 = $FF
    // Each addition results in $FF, so final result is $FF
    assert_eq!(sio_checksum(&[0xFF, 0xFF, 0xFF, 0xFF]), 0xFF);
}

#[test]
fn test_checksum_known_read_sector() {
    // Read sector 1 from D1:
    // Device: $31, Command: $52, AUX1: $01, AUX2: $00
    // $31 + $52 + $01 + $00 = $84
    assert_eq!(sio_checksum(&[0x31, 0x52, 0x01, 0x00]), 0x84);
}

#[test]
fn test_checksum_status_d2() {
    // Status from D2:
    // Device: $32, Command: $53, AUX1: $00, AUX2: $00
    // $32 + $53 + $00 + $00 = $85
    assert_eq!(sio_checksum(&[0x32, 0x53, 0x00, 0x00]), 0x85);
}

// ============================================================================
// Test 2: Command Frame Structure
// ============================================================================

#[derive(Debug, PartialEq)]
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
        let calculated = sio_checksum(&[self.device_id, self.command, self.aux1, self.aux2]);
        calculated == self.checksum
    }

    fn sector_number(&self) -> u16 {
        // AUX1 = low byte, AUX2 = high byte (little-endian)
        (self.aux1 as u16) | ((self.aux2 as u16) << 8)
    }
}

#[test]
fn test_command_frame_parsing() {
    let frame = CommandFrame::from_bytes(&[0x31, 0x52, 0x01, 0x00, 0x84]);

    assert_eq!(frame.device_id, 0x31);  // D1:
    assert_eq!(frame.command, 0x52);     // Read sector
    assert_eq!(frame.aux1, 0x01);
    assert_eq!(frame.aux2, 0x00);
    assert_eq!(frame.checksum, 0x84);
    assert_eq!(frame.sector_number(), 1);
}

#[test]
fn test_command_frame_valid_checksum() {
    let frame = CommandFrame::from_bytes(&[0x31, 0x53, 0x00, 0x00, 0x84]);
    assert!(frame.validate_checksum());
}

#[test]
fn test_command_frame_invalid_checksum() {
    let frame = CommandFrame::from_bytes(&[0x31, 0x53, 0x00, 0x00, 0xFF]);
    assert!(!frame.validate_checksum());
}

#[test]
fn test_sector_number_parsing() {
    // Sector 1
    let frame1 = CommandFrame::from_bytes(&[0x31, 0x52, 0x01, 0x00, 0x84]);
    assert_eq!(frame1.sector_number(), 1);

    // Sector 256 (0x100)
    let frame2 = CommandFrame::from_bytes(&[0x31, 0x52, 0x00, 0x01, 0x84]);
    assert_eq!(frame2.sector_number(), 256);

    // Sector 720 (single density disk)
    let frame3 = CommandFrame::from_bytes(&[0x31, 0x52, 0xD0, 0x02, 0x56]);
    assert_eq!(frame3.sector_number(), 720);
}

// ============================================================================
// Test 3: Device ID Routing
// ============================================================================

fn device_id_to_unit(device_id: u8) -> Option<(&'static str, u8)> {
    match device_id {
        0x31..=0x3F => Some(("D", device_id - 0x30)),  // D1: through D15:
        0x40 => Some(("P", 1)),                         // P1: (printer)
        0x41 => Some(("P", 2)),                         // P2:
        0x50..=0x53 => Some(("R", device_id - 0x4F)),  // R1: through R4: (850 serial)
        0x58 => Some(("T", 1)),                         // T1: (1030 modem)
        _ => None,
    }
}

#[test]
fn test_disk_device_ids() {
    assert_eq!(device_id_to_unit(0x31), Some(("D", 1)));
    assert_eq!(device_id_to_unit(0x32), Some(("D", 2)));
    assert_eq!(device_id_to_unit(0x33), Some(("D", 3)));
    assert_eq!(device_id_to_unit(0x34), Some(("D", 4)));
    assert_eq!(device_id_to_unit(0x3F), Some(("D", 15)));
}

#[test]
fn test_printer_device_ids() {
    assert_eq!(device_id_to_unit(0x40), Some(("P", 1)));
    assert_eq!(device_id_to_unit(0x41), Some(("P", 2)));
}

#[test]
fn test_unknown_device_ids() {
    assert_eq!(device_id_to_unit(0x00), None);
    assert_eq!(device_id_to_unit(0x20), None);
    assert_eq!(device_id_to_unit(0x5F), None);  // Cassette (virtual)
    assert_eq!(device_id_to_unit(0xFF), None);
}

// ============================================================================
// Test 4: SIO Response Types
// ============================================================================

#[derive(Debug, PartialEq, Clone)]
enum SioResponse {
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

    /// $45 + data + checksum - Error but still returns data (e.g. Format)
    ErrorWithData(Vec<u8>),
}

impl SioResponse {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            SioResponse::Nak => vec![0x4E],
            SioResponse::Ack => vec![0x41],
            SioResponse::Complete => vec![0x43],
            SioResponse::Error => vec![0x45],
            SioResponse::CompleteWithData(data) => {
                let mut response = vec![0x43];
                response.extend_from_slice(data);
                let checksum = sio_checksum(&response);
                response.push(checksum);
                response
            }
            SioResponse::ErrorWithData(data) => {
                let mut response = vec![0x45];
                response.extend_from_slice(data);
                let checksum = sio_checksum(&response);
                response.push(checksum);
                response
            }
        }
    }

    fn result_byte(&self) -> u8 {
        match self {
            SioResponse::Nak => 0x4E,
            SioResponse::Ack => 0x41,
            SioResponse::Complete => 0x43,
            SioResponse::CompleteWithData(_) => 0x43,
            SioResponse::Error => 0x45,
            SioResponse::ErrorWithData(_) => 0x45,
        }
    }
}

#[test]
fn test_nak_response() {
    let response = SioResponse::Nak;
    assert_eq!(response.to_bytes(), vec![0x4E]);
    assert_eq!(response.result_byte(), 0x4E);
}

#[test]
fn test_ack_response() {
    let response = SioResponse::Ack;
    assert_eq!(response.to_bytes(), vec![0x41]);
    assert_eq!(response.result_byte(), 0x41);
}

#[test]
fn test_complete_with_status_data() {
    // Status response: 4 status bytes
    let status_data = vec![0x10, 0xFF, 0x60, 0x00];
    let response = SioResponse::CompleteWithData(status_data.clone());

    let bytes = response.to_bytes();

    // Should be: Complete byte + 4 status bytes + checksum
    assert_eq!(bytes.len(), 6);
    assert_eq!(bytes[0], 0x43);  // Complete
    assert_eq!(&bytes[1..5], status_data.as_slice());

    // Verify checksum
    let expected_checksum = sio_checksum(&bytes[0..5]);
    assert_eq!(bytes[5], expected_checksum);
}

#[test]
fn test_complete_with_sector_data() {
    // Sector data: 128 bytes
    let sector_data = vec![0x42; 128];  // Fill with 'B'
    let response = SioResponse::CompleteWithData(sector_data.clone());

    let bytes = response.to_bytes();

    // Should be: Complete byte + 128 bytes + checksum
    assert_eq!(bytes.len(), 130);
    assert_eq!(bytes[0], 0x43);
    assert_eq!(&bytes[1..129], sector_data.as_slice());

    // Verify checksum
    let expected_checksum = sio_checksum(&bytes[0..129]);
    assert_eq!(bytes[129], expected_checksum);
}

// ============================================================================
// Test 5: SIO State Machine
// ============================================================================

#[derive(Debug, PartialEq, Clone)]
enum SioState {
    Idle,
    ReceivingCommand {
        bytes_received: usize,
        buffer: [u8; 5],
    },
    SendingResponse {
        bytes_sent: usize,
        response: Vec<u8>,
    },
}

struct SioStateMachine {
    state: SioState,
    command_line: bool,
}

impl SioStateMachine {
    fn new() -> Self {
        SioStateMachine {
            state: SioState::Idle,
            command_line: false,
        }
    }

    fn set_command_line(&mut self, asserted: bool) {
        // Rising edge: start receiving command
        if asserted && !self.command_line {
            self.state = SioState::ReceivingCommand {
                bytes_received: 0,
                buffer: [0; 5],
            };
        }
        self.command_line = asserted;
    }

    fn receive_byte(&mut self, byte: u8) {
        if let SioState::ReceivingCommand { bytes_received, mut buffer } = self.state {
            if bytes_received < 5 {
                buffer[bytes_received] = byte;
                let new_count = bytes_received + 1;

                if new_count >= 5 {
                    // Command complete - validate and prepare response
                    let frame = CommandFrame::from_bytes(&buffer);
                    let response = if frame.validate_checksum() {
                        vec![0x41]  // ACK
                    } else {
                        vec![0x4E]  // NAK
                    };

                    self.state = SioState::SendingResponse {
                        bytes_sent: 0,
                        response,
                    };
                } else {
                    self.state = SioState::ReceivingCommand {
                        bytes_received: new_count,
                        buffer,
                    };
                }
            }
        }
    }

    fn get_response_byte(&mut self) -> Option<u8> {
        if let SioState::SendingResponse { bytes_sent, ref response } = self.state {
            if bytes_sent < response.len() {
                let byte = response[bytes_sent];
                let new_count = bytes_sent + 1;

                if new_count >= response.len() {
                    self.state = SioState::Idle;
                } else {
                    self.state = SioState::SendingResponse {
                        bytes_sent: new_count,
                        response: response.clone(),
                    };
                }

                return Some(byte);
            }
        }
        None
    }
}

#[test]
fn test_state_machine_valid_command() {
    let mut sm = SioStateMachine::new();

    // Assert command line
    sm.set_command_line(true);
    assert!(matches!(sm.state, SioState::ReceivingCommand { .. }));

    // Send valid command frame
    let command = [0x31, 0x53, 0x00, 0x00, 0x84];
    for &byte in &command {
        sm.receive_byte(byte);
    }

    // Should transition to sending response
    assert!(matches!(sm.state, SioState::SendingResponse { .. }));

    // Should return ACK
    assert_eq!(sm.get_response_byte(), Some(0x41));

    // Should be idle after ACK
    assert_eq!(sm.state, SioState::Idle);
}

#[test]
fn test_state_machine_invalid_checksum() {
    let mut sm = SioStateMachine::new();

    sm.set_command_line(true);

    // Send command with bad checksum
    let command = [0x31, 0x53, 0x00, 0x00, 0xFF];
    for &byte in &command {
        sm.receive_byte(byte);
    }

    // Should send NAK
    assert_eq!(sm.get_response_byte(), Some(0x4E));
    assert_eq!(sm.state, SioState::Idle);
}

#[test]
fn test_state_machine_partial_command() {
    let mut sm = SioStateMachine::new();

    sm.set_command_line(true);

    // Send only 3 bytes
    sm.receive_byte(0x31);
    sm.receive_byte(0x53);
    sm.receive_byte(0x00);

    // Should still be receiving
    assert!(matches!(
        sm.state,
        SioState::ReceivingCommand { bytes_received: 3, .. }
    ));

    // No response available yet
    assert_eq!(sm.get_response_byte(), None);
}

#[test]
fn test_state_machine_restart_on_command_line() {
    let mut sm = SioStateMachine::new();

    sm.set_command_line(true);

    // Send 2 bytes
    sm.receive_byte(0x31);
    sm.receive_byte(0x53);

    // Deassert and reassert command line (simulates retry)
    sm.set_command_line(false);
    sm.set_command_line(true);

    // Should restart
    if let SioState::ReceivingCommand { bytes_received, .. } = sm.state {
        assert_eq!(bytes_received, 0);
    } else {
        panic!("Expected ReceivingCommand state");
    }
}

// ============================================================================
// Test 6: Mock Device Implementation
// ============================================================================

trait SioDevice {
    fn device_id(&self) -> u8;
    fn handle_command(&mut self, cmd: u8, aux1: u8, aux2: u8) -> SioResponse;
}

struct MockDiskDrive {
    device_id: u8,
    sector_count: u16,
}

impl MockDiskDrive {
    fn new(device_id: u8, sector_count: u16) -> Self {
        MockDiskDrive {
            device_id,
            sector_count,
        }
    }
}

impl SioDevice for MockDiskDrive {
    fn device_id(&self) -> u8 {
        self.device_id
    }

    fn handle_command(&mut self, cmd: u8, aux1: u8, aux2: u8) -> SioResponse {
        match cmd {
            0x53 => {
                // Status command
                let status = vec![
                    0x10,  // Drive ready
                    0xFF,  // Hardware OK
                    0x60,  // Timeout
                    0x00,  // Unused
                ];
                SioResponse::CompleteWithData(status)
            }
            0x52 => {
                // Read sector
                let sector = (aux1 as u16) | ((aux2 as u16) << 8);

                if sector == 0 || sector > self.sector_count {
                    SioResponse::Error
                } else {
                    // Return dummy sector data
                    let data = vec![0x42; 128];  // All 'B'
                    SioResponse::CompleteWithData(data)
                }
            }
            _ => SioResponse::Nak,
        }
    }
}

#[test]
fn test_mock_disk_status() {
    let mut drive = MockDiskDrive::new(0x31, 720);
    let response = drive.handle_command(0x53, 0x00, 0x00);

    match response {
        SioResponse::CompleteWithData(data) => {
            assert_eq!(data.len(), 4);
            assert_eq!(data[0], 0x10);  // Drive ready
        }
        _ => panic!("Expected CompleteWithData"),
    }
}

#[test]
fn test_mock_disk_read_valid_sector() {
    let mut drive = MockDiskDrive::new(0x31, 720);
    let response = drive.handle_command(0x52, 0x01, 0x00);  // Sector 1

    match response {
        SioResponse::CompleteWithData(data) => {
            assert_eq!(data.len(), 128);
        }
        _ => panic!("Expected CompleteWithData"),
    }
}

#[test]
fn test_mock_disk_read_invalid_sector() {
    let mut drive = MockDiskDrive::new(0x31, 720);

    // Sector 0 (invalid)
    assert!(matches!(
        drive.handle_command(0x52, 0x00, 0x00),
        SioResponse::Error
    ));

    // Sector 721 (beyond disk)
    assert!(matches!(
        drive.handle_command(0x52, 0xD1, 0x02),
        SioResponse::Error
    ));
}

#[test]
fn test_mock_disk_unknown_command() {
    let mut drive = MockDiskDrive::new(0x31, 720);
    let response = drive.handle_command(0x99, 0x00, 0x00);

    assert!(matches!(response, SioResponse::Nak));
}

// ============================================================================
// Test 7: Multi-Device Routing
// ============================================================================

struct SioController {
    devices: Vec<Box<dyn SioDevice>>,
}

impl SioController {
    fn new() -> Self {
        SioController {
            devices: Vec::new(),
        }
    }

    fn add_device(&mut self, device: Box<dyn SioDevice>) {
        self.devices.push(device);
    }

    fn route_command(&mut self, frame: &CommandFrame) -> SioResponse {
        if !frame.validate_checksum() {
            return SioResponse::Nak;
        }

        // Find device with matching ID
        for device in &mut self.devices {
            if device.device_id() == frame.device_id {
                return device.handle_command(frame.command, frame.aux1, frame.aux2);
            }
        }

        // No device found - ignore (don't send NAK)
        SioResponse::Nak
    }
}

#[test]
fn test_route_to_correct_device() {
    let mut controller = SioController::new();

    controller.add_device(Box::new(MockDiskDrive::new(0x31, 720)));  // D1:
    controller.add_device(Box::new(MockDiskDrive::new(0x32, 1040))); // D2:

    // Status to D1:
    let frame1 = CommandFrame::from_bytes(&[0x31, 0x53, 0x00, 0x00, 0x84]);
    let response1 = controller.route_command(&frame1);
    assert!(matches!(response1, SioResponse::CompleteWithData(_)));

    // Status to D2:
    let frame2 = CommandFrame::from_bytes(&[0x32, 0x53, 0x00, 0x00, 0x85]);
    let response2 = controller.route_command(&frame2);
    assert!(matches!(response2, SioResponse::CompleteWithData(_)));
}

#[test]
fn test_route_to_nonexistent_device() {
    let mut controller = SioController::new();

    controller.add_device(Box::new(MockDiskDrive::new(0x31, 720)));

    // Command to D3: (not present)
    let frame = CommandFrame::from_bytes(&[0x33, 0x53, 0x00, 0x00, 0x86]);
    let response = controller.route_command(&frame);

    assert!(matches!(response, SioResponse::Nak));
}

#[test]
fn test_reject_bad_checksum() {
    let mut controller = SioController::new();

    controller.add_device(Box::new(MockDiskDrive::new(0x31, 720)));

    // Valid command but bad checksum
    let frame = CommandFrame::from_bytes(&[0x31, 0x53, 0x00, 0x00, 0xFF]);
    let response = controller.route_command(&frame);

    assert!(matches!(response, SioResponse::Nak));
}

// ============================================================================
// Test 8: Timing Constants
// ============================================================================

const NTSC_CLOCK: u32 = 1_789_725;  // 1.79 MHz

#[derive(Debug)]
struct SioTiming {
    cycles_per_byte: u32,       // Time to send/receive one byte
    command_pre_delay: u32,     // 750μs - 1600μs
    command_post_delay: u32,    // 650μs - 950μs
    ack_timeout: u32,           // 0 - 16ms
    result_min_delay: u32,      // 250μs minimum
}

impl SioTiming {
    fn new_ntsc_19200_baud() -> Self {
        // At 19200 baud: 1 bit = 52.08μs, 10 bits = 520.8μs
        let cycles_per_byte = (NTSC_CLOCK as f64 * 520.8e-6) as u32;

        SioTiming {
            cycles_per_byte,
            command_pre_delay: (NTSC_CLOCK as f64 * 1000e-6) as u32,   // 1ms
            command_post_delay: (NTSC_CLOCK as f64 * 800e-6) as u32,   // 800μs
            ack_timeout: (NTSC_CLOCK as f64 * 16e-3) as u32,           // 16ms
            result_min_delay: (NTSC_CLOCK as f64 * 250e-6) as u32,     // 250μs
        }
    }
}

#[test]
fn test_timing_calculations() {
    let timing = SioTiming::new_ntsc_19200_baud();

    // ~932 cycles per byte at 1.79MHz, 19200 baud
    assert!(timing.cycles_per_byte > 900 && timing.cycles_per_byte < 950);

    // Command delays
    assert!(timing.command_pre_delay > 1700);   // > 750μs
    assert!(timing.command_post_delay > 1400);  // > 650μs

    // ACK timeout
    assert!(timing.ack_timeout > 28000);  // > 16ms

    // Result delay
    assert!(timing.result_min_delay > 400);  // > 250μs
}

#[test]
fn test_full_command_timing() {
    let timing = SioTiming::new_ntsc_19200_baud();

    // Full command frame time:
    // Pre-delay + 5 bytes + post-delay
    let command_time = timing.command_pre_delay
        + (5 * timing.cycles_per_byte)
        + timing.command_post_delay;

    // Should be around 3-4ms total
    // 1000μs + (5 * 520.8μs) + 800μs = ~4.4ms
    assert!(command_time > 6000 && command_time < 8500);
}

// ============================================================================
// Integration Tests: PIA-SIO Interaction
// ============================================================================
//
// These tests verify the integration between PIA (which controls the
// COMMAND line via CB2) and SioController (which observes the COMMAND
// line to detect command frames).

#[test]
fn test_pia_cb2_controls_sio_command_line() {
    use atari800_rs::pia::Pia;
    use atari800_rs::sio::SioController;

    let mut pia = Pia::new();
    let mut sio = SioController::new();

    // Initially, CB2 should be low (PBCTL bit 3 = 0)
    assert_eq!(pia.get_cb2_output(), false);

    // CPU writes to PBCTL ($D303) to assert command line (set bit 3)
    pia.write_register(0xD303, 0x08);

    // Read CB2 state and pass to SIO controller
    let command_line = pia.get_cb2_output();
    assert_eq!(command_line, true);
    sio.set_command_line(command_line);

    // SIO should now be ready to receive command bytes
    // (State should transition from Idle to ReceivingCommand)
    assert!(sio.state().contains("ReceivingCommand"));
}

#[test]
fn test_full_command_sequence_via_pia() {
    use atari800_rs::pia::Pia;
    use atari800_rs::sio::{SioController, SioDevice, SioResponse, SioTiming};

    // Mock device for testing
    struct MockDisk {
        id: u8,
    }

    impl SioDevice for MockDisk {
        fn device_id(&self) -> u8 {
            self.id
        }

        fn handle_command(&mut self, cmd: u8, _aux1: u8, _aux2: u8) -> SioResponse {
            match cmd {
                0x53 => SioResponse::CompleteWithData(vec![0x10, 0xFF, 0x60, 0x00]), // Status
                _ => SioResponse::Nak,
            }
        }
    }

    let mut pia = Pia::new();
    let mut sio = SioController::with_timing(SioTiming::instant());

    // Add a mock disk drive (D1:)
    sio.add_device(Box::new(MockDisk { id: 0x31 }));

    // Step 1: CPU asserts COMMAND line via PIA
    pia.write_register(0xD303, 0x08); // Set PBCTL bit 3
    sio.set_command_line(pia.get_cb2_output());

    // Step 2: CPU sends 5-byte command frame via POKEY serial port
    // (simulated here by directly calling receive_byte)
    // Checksum = $31 + $53 + $00 + $00 = $84 (matches AltirraOS)
    let command_bytes = [0x31, 0x53, 0x00, 0x00, 0x84]; // D1: Status command
    for &byte in &command_bytes {
        sio.receive_byte(byte);
    }

    // Step 3: CPU deasserts COMMAND line
    pia.write_register(0xD303, 0x00); // Clear PBCTL bit 3
    sio.set_command_line(pia.get_cb2_output());

    // Step 4: SIO should send ACK
    assert!(sio.has_byte_for_pokey());
    assert_eq!(sio.get_byte_for_pokey(), Some(0x41)); // ACK

    // Step 5: Tick to execute command
    while !sio.has_byte_for_pokey() {
        sio.tick();
    }

    // Step 6: SIO should send Complete byte
    assert_eq!(sio.get_byte_for_pokey(), Some(0x43)); // Complete

    // Step 7: SIO should send data bytes
    assert_eq!(sio.get_byte_for_pokey(), Some(0x10));
    assert_eq!(sio.get_byte_for_pokey(), Some(0xFF));
    assert_eq!(sio.get_byte_for_pokey(), Some(0x60));
    assert_eq!(sio.get_byte_for_pokey(), Some(0x00));

    // Step 8: SIO should send checksum
    assert!(sio.has_byte_for_pokey());
    let _checksum = sio.get_byte_for_pokey();
}

#[test]
fn test_pia_command_line_must_deassert_before_response() {
    use atari800_rs::pia::Pia;
    use atari800_rs::sio::{SioController, SioDevice, SioResponse, SioTiming};

    struct MockDevice;
    impl SioDevice for MockDevice {
        fn device_id(&self) -> u8 { 0x31 }
        fn handle_command(&mut self, _: u8, _: u8, _: u8) -> SioResponse {
            SioResponse::Complete
        }
    }

    let mut pia = Pia::new();
    let mut sio = SioController::with_timing(SioTiming::instant());
    sio.add_device(Box::new(MockDevice));

    // Assert command line
    pia.write_register(0xD303, 0x08);
    sio.set_command_line(pia.get_cb2_output());

    // Send command frame
    // Checksum = $31 + $53 + $00 + $00 = $84 (matches AltirraOS)
    for &byte in &[0x31, 0x53, 0x00, 0x00, 0x84] {
        sio.receive_byte(byte);
    }

    // Command frame complete, but COMMAND line still asserted
    // SIO should be waiting for command line to deassert
    assert!(sio.state().contains("WaitingForCommandLineDeassert"));
    assert!(!sio.has_byte_for_pokey()); // No response yet

    // Now deassert command line
    pia.write_register(0xD303, 0x00);
    sio.set_command_line(pia.get_cb2_output());

    // Now SIO should send ACK
    assert!(sio.has_byte_for_pokey());
    assert_eq!(sio.get_byte_for_pokey(), Some(0x41));
}

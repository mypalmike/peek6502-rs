// SIO (Serial I/O) Protocol Handler
//
// Handles the Atari SIO protocol for communication with peripheral devices.
// This module manages command frames, checksums, and data transfer, routing
// commands to the appropriate device (disk drives, printers, etc.).
//
// NOTE: This is the old implementation. The new architecture is in sio_new.rs

use crate::atrdisk::AtrDisk;
use crate::sio_new::{SioDevice, SioResponse};

#[derive(Debug, Clone, Copy, PartialEq)]
enum SioState {
    Idle,
    ReceivingCommand,    // Accumulating 5-byte command frame
    SendingAck,          // Sending 'A'
    SendingNak,          // Sending 'N'
    SendingData,         // Streaming data (includes result byte, data, checksum)
    SendingError,        // Sending 'E'
}

pub struct Sio {
    state: SioState,
    command_buffer: [u8; 5],  // Device ID, Command, Aux1, Aux2, Checksum
    command_index: usize,
    data_buffer: Vec<u8>,     // Response data (sector contents, status)
    data_index: usize,
    drives: [Option<AtrDisk>; 4],  // D1: through D4: (only D1 used initially)
    command_line: bool,       // SIO command line state (true = command frame in progress)
}

impl Sio {
    pub fn new() -> Self {
        Sio {
            state: SioState::Idle,
            command_buffer: [0; 5],
            command_index: 0,
            data_buffer: Vec::new(),
            data_index: 0,
            drives: [None, None, None, None],
            command_line: false,
        }
    }

    /// Set the SIO command line state (controlled by PIA CB2 via PBCTL register)
    pub fn set_command_line(&mut self, asserted: bool) {
        if asserted && !self.command_line {
            // Command line rising edge - start new command frame
            self.state = SioState::Idle;
            self.command_index = 0;
        }
        self.command_line = asserted;
    }

    /// Called per cycle (no-op for simplified timing)
    pub fn tick(&mut self) {
        // Future: implement cycle-accurate timing here
    }

    /// Called by POKEY when CPU writes to SEROUT ($D20D)
    pub fn send_byte(&mut self, byte: u8) {
        // Only accept command bytes when command line is asserted
        if !self.command_line {
            return;
        }

        match self.state {
            SioState::Idle => {
                // First byte starts a new command frame
                self.command_buffer[0] = byte;
                self.command_index = 1;
                self.state = SioState::ReceivingCommand;
            }
            SioState::ReceivingCommand => {
                self.command_buffer[self.command_index] = byte;
                self.command_index += 1;

                if self.command_index >= 5 {
                    // Complete command frame received
                    self.process_command();
                }
            }
            _ => {
                // Ignore bytes received while we're sending a response
                // The OS should wait for our response before sending next command
            }
        }
    }

    /// Called by POKEY when CPU reads from SERIN ($D20D)
    pub fn receive_byte(&mut self) -> u8 {
        let byte = match self.state {
            SioState::SendingAck => {
                self.state = if !self.data_buffer.is_empty() {
                    SioState::SendingData
                } else {
                    // No data to send - shouldn't happen with current commands
                    SioState::Idle
                };
                b'A'
            }
            SioState::SendingNak => {
                self.state = SioState::Idle;
                self.command_index = 0;  // Reset for next command
                b'N'
            }
            SioState::SendingData => {
                if self.data_index < self.data_buffer.len() {
                    let byte = self.data_buffer[self.data_index];
                    self.data_index += 1;

                    // After sending all data bytes, go back to Idle
                    if self.data_index >= self.data_buffer.len() {
                        self.state = SioState::Idle;
                        self.command_index = 0;  // Reset for next command
                        self.data_buffer.clear();
                        self.data_index = 0;
                    }

                    byte
                } else {
                    // Shouldn't reach here, but handle gracefully
                    self.state = SioState::Idle;
                    self.command_index = 0;
                    self.data_buffer.clear();
                    self.data_index = 0;
                    0x00
                }
            }
            SioState::SendingError => {
                self.state = SioState::Idle;
                self.command_index = 0;  // Reset for next command
                b'E'
            }
            SioState::Idle | SioState::ReceivingCommand => {
                0x00  // No data available
            }
        };
        eprintln!("SIO RX: ${:02X} (state: {:?})", byte, self.state);
        byte
    }

    /// Check if byte available for CPU to read
    pub fn data_available(&self) -> bool {
        match self.state {
            SioState::SendingAck | SioState::SendingNak | SioState::SendingData |
            SioState::SendingError => true,
            _ => false,
        }
    }

    /// Check if ready for next command byte
    pub fn ready_for_byte(&self) -> bool {
        matches!(self.state, SioState::Idle | SioState::ReceivingCommand)
    }

    /// Load disk image into drive slot (0-3 for D1:-D4:)
    pub fn load_disk(&mut self, drive: usize, path: &str) -> Result<(), String> {
        if drive >= 4 {
            return Err(format!("Invalid drive number: {} (must be 0-3)", drive));
        }

        // Device ID: 0x31 = D1:, 0x32 = D2:, etc.
        let device_id = 0x31 + (drive as u8);
        let disk = AtrDisk::new(path, device_id)?;
        self.drives[drive] = Some(disk);
        Ok(())
    }

    /// Process a complete 5-byte command frame
    fn process_command(&mut self) {
        let device_id = self.command_buffer[0];
        let command = self.command_buffer[1];
        let aux1 = self.command_buffer[2];
        let aux2 = self.command_buffer[3];
        let checksum = self.command_buffer[4];

        eprintln!("SIO CMD: Device ${:02X}, Cmd ${:02X}, Aux ${:02X} ${:02X}, Chk ${:02X}",
                  device_id, command, aux1, aux2, checksum);

        // Validate checksum
        if !self.validate_checksum() {
            eprintln!("SIO: Checksum FAILED - sending NAK");
            self.state = SioState::SendingNak;
            return;
        }

        // Route to appropriate drive (D1: = $31, D2: = $32, etc.)
        let drive_index = match device_id {
            0x31 => 0,  // D1:
            0x32 => 1,  // D2:
            0x33 => 2,  // D3:
            0x34 => 3,  // D4:
            _ => {
                // Unknown device
                self.state = SioState::SendingNak;
                return;
            }
        };

        // Check if drive exists
        if let Some(ref mut drive) = self.drives[drive_index] {
            let response = drive.handle_command(command, aux1, aux2);
            self.handle_response(response);
        } else {
            // No disk in drive
            self.state = SioState::SendingNak;
        }
    }

    /// Validate the checksum of the command frame
    /// Checksum is calculated on bytes 1-3 (command, aux1, aux2), NOT the device ID
    fn validate_checksum(&self) -> bool {
        let calculated = calculate_checksum(&self.command_buffer[1..4]);
        let received = self.command_buffer[4];
        calculated == received
    }

    /// Handle the response from a disk drive
    fn handle_response(&mut self, response: SioResponse) {
        match response {
            SioResponse::Ack => {
                self.data_buffer.clear();
                self.data_index = 0;
                self.state = SioState::SendingAck;
            }
            SioResponse::Complete => {
                self.data_buffer.clear();
                self.data_index = 0;
                self.state = SioState::SendingAck;
            }
            SioResponse::CompleteWithData(data) => {
                // Build response with 'C' + data + checksum
                let mut response_buf = vec![b'C'];
                response_buf.extend_from_slice(&data);
                let checksum = calculate_checksum(&response_buf);
                response_buf.push(checksum);

                self.data_buffer = response_buf;
                self.data_index = 0;
                self.state = SioState::SendingAck;
            }
            SioResponse::ErrorWithData(data) => {
                // Build response with 'E' + data + checksum
                let mut response_buf = vec![b'E'];
                response_buf.extend_from_slice(&data);
                let checksum = calculate_checksum(&response_buf);
                response_buf.push(checksum);

                self.data_buffer = response_buf;
                self.data_index = 0;
                self.state = SioState::SendingAck;
            }
            SioResponse::Nak => {
                self.state = SioState::SendingNak;
            }
            SioResponse::Error => {
                self.state = SioState::SendingError;
            }
        }
    }
}

/// Calculate SIO checksum with carry wraparound
fn calculate_checksum(bytes: &[u8]) -> u8 {
    let mut sum = 0u16;
    for &byte in bytes {
        sum += byte as u16;
        if sum > 0xFF {
            sum = (sum & 0xFF) + 1;  // Add carry
        }
    }
    sum as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sio_checksum() {
        // D1: Status command: $31 $53 $01 $00
        let bytes = [0x31, 0x53, 0x01, 0x00];
        let checksum = calculate_checksum(&bytes);
        // $31 + $53 + $01 + $00 = $85
        assert_eq!(checksum, 0x85);
    }

    #[test]
    fn test_sio_checksum_with_carry() {
        // Test checksum with carry
        let bytes = [0xFF, 0xFF, 0x01, 0x00];
        let checksum = calculate_checksum(&bytes);
        // $FF + $FF = $1FE -> $FE + 1 = $FF
        // $FF + $01 = $100 -> $00 + 1 = $01
        // $01 + $00 = $01
        assert_eq!(checksum, 0x01);
    }
}

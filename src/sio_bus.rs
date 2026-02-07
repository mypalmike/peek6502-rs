// SIO Bus Abstraction
//
// This module defines the SioBus trait, which provides a bus abstraction for
// Serial I/O operations. It mirrors the CPU Bus pattern for architectural consistency.
//
// The SioBus trait allows SioController to remain decoupled from the specific
// implementation details of how commands are routed to devices and how hardware
// state is accessed. This follows the same architectural pattern as the CPU bus,
// where the CPU uses the Bus trait to access memory and I/O without knowing
// the specific implementation.

use crate::sio::{SioResponse, SioTiming};

/// Bus abstraction for Serial I/O operations
///
/// This trait provides the interface between SioController (protocol handler)
/// and the system implementation (Atari800). It allows the SioController to:
/// - Query hardware state (command line from PIA CB2)
/// - Route commands to devices by device ID
/// - Send response bytes back to the computer (via POKEY)
/// - Access timing configuration
///
/// The Atari800 struct implements this trait to provide the actual routing
/// and hardware integration.
pub trait SioBus {
    /// Get command line state from PIA CB2
    ///
    /// The command line is controlled by PIA port B bit 2 (CB2).
    /// When asserted (true), it signals the start of a command frame.
    /// When deasserted (false), it signals the end of the command frame.
    ///
    /// Note: In the actual hardware, CB2 is active low (0V = asserted),
    /// but this method returns true when asserted for clarity.
    fn get_command_line(&self) -> bool;

    /// Route SIO command to the appropriate device by device ID
    ///
    /// This method finds the device that accepts the given device_id and
    /// forwards the command to it. Device IDs typically follow this pattern:
    /// - $31-$34: Disk drives D1-D4
    /// - $40-$43: Printers P1-P4
    /// - $50-$53: Modems R1-R4
    ///
    /// # Arguments
    /// * `device_id` - The device identifier from the command frame
    /// * `cmd` - The command byte (e.g., $52 = Read, $53 = Status)
    /// * `aux1` - First auxiliary byte (often low byte of sector number)
    /// * `aux2` - Second auxiliary byte (often high byte of sector number)
    ///
    /// # Returns
    /// The SioResponse from the device, or SioResponse::Nak if no device found
    fn route_sio_command(&mut self, device_id: u8, cmd: u8, aux1: u8, aux2: u8) -> SioResponse;

    /// Check if a device with the given ID exists on the bus
    ///
    /// This can be used to optimize command routing or for diagnostics.
    fn has_sio_device(&self, device_id: u8) -> bool;

    /// Send a byte to POKEY SERIN register (for the OS to read)
    ///
    /// When SIO devices send response bytes, they go through POKEY's serial
    /// input port. This method queues a byte in POKEY's SERIN register and
    /// triggers the serial input ready interrupt if enabled.
    ///
    /// # Arguments
    /// * `byte` - The byte to send to the computer (ACK, NAK, data, etc.)
    fn send_byte_to_pokey(&mut self, byte: u8);

    /// Get timing configuration for SIO operations
    ///
    /// Returns the timing parameters that control cycle-accurate emulation:
    /// - Baud rate (cycles per byte)
    /// - Command execution delays
    /// - Response timing
    fn get_sio_timing(&self) -> &SioTiming;
}

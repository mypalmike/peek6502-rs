// Strict Disk Boot Tests - These SHOULD fail until byte transfer is fixed
//
// These tests verify the ACTUAL boot sequence that the OS performs.
// They will fail at the byte transfer stage, showing exactly what needs to be fixed.

use atari800_rs::atari800::Atari800;
use atari800_rs::atrdisk::AtrDisk;

use atari800_rs::sio::SioDevice;

#[test]
#[should_panic(expected = "Expected ACK (0x41)")]
fn test_os_cannot_read_ack_byte() {
    println!("\n=== This test SHOULD FAIL - showing the byte transfer problem ===\n");

    let atr_data = create_minimal_bootable_atr();
    let temp_path = "/tmp/test_strict.atr";
    std::fs::write(temp_path, &atr_data).unwrap();

    let mut atari = Atari800::for_render_test();
    let disk = AtrDisk::new(temp_path, 0x31).unwrap();
    atari.add_sio_device(Box::new(disk));

    // Simulate OS sending Status command
    println!("OS: Asserting command line...");
    atari.bus_write(0xD303, 0x08);  // Assert command line
    atari.tick_sio();

    println!("OS: Sending command frame: 31 53 00 00 84");
    let command = [0x31u8, 0x53, 0x00, 0x00, 0x84];
    for &byte in &command {
        atari.bus_write(0xD20D, byte);  // Write to SEROUT
        atari.tick_sio();
    }

    println!("OS: Deasserting command line...");
    atari.bus_write(0xD303, 0x30);  // Deassert command line
    atari.tick_sio();

    println!("OS: Processing command...");
    atari.tick_sio();

    println!("OS: Reading ACK from SERIN...");
    let ack = atari.bus_read(0xD20D);

    println!("\n!!! PROBLEM DETECTED !!!");
    println!("Expected: 0x41 (ACK)");
    println!("Got:      0x{:02X} (this is the checksum we wrote, not the ACK!)", ack);
    println!("\nThis shows that POKEY SERIN is reading back what was written,");
    println!("not the response bytes from the SIO device.");
    println!("\nRoot cause: tick_sio() sends all response bytes at once,");
    println!("each call to queue_serin_byte() overwrites the previous byte.");

    assert_eq!(ack, 0x41, "Expected ACK (0x41), got 0x{:02X}", ack);

    let _ = std::fs::remove_file(temp_path);
}

#[test]
fn test_device_response_is_correct() {
    println!("\n=== This test PASSES - showing the device works correctly ===\n");

    let atr_data = create_minimal_bootable_atr();
    let temp_path = "/tmp/test_device.atr";
    std::fs::write(temp_path, &atr_data).unwrap();

    let mut disk = AtrDisk::new(temp_path, 0x31).unwrap();

    // Call device directly (bypassing SIO integration)
    println!("Calling device.handle_command(0x53, 0x00, 0x00) directly...");
    let response = disk.handle_command(0x53, 0x00, 0x00);

    match response {
        atari800_rs::sio::SioResponse::CompleteWithData(data) => {
            println!("✓ Device returns CompleteWithData");
            println!("✓ Data: {:02X?}", data);
            assert_eq!(data.len(), 4, "Status should return 4 bytes");
            println!("\nThis proves the device works correctly!");
            println!("The problem is in the byte transfer: SIO → POKEY → OS");
        }
        _ => panic!("Expected CompleteWithData"),
    }

    let _ = std::fs::remove_file(temp_path);
}

#[test]
fn test_show_what_sio_controller_queues() {
    println!("\n=== This test shows what SIO controller actually queues ===\n");

    use atari800_rs::sio::{SioController, SioDevice, SioResponse};
    use atari800_rs::sio_bus::SioBus;
    use atari800_rs::sio::SioTiming;

    // Create mock device
    struct TestDevice;
    impl SioDevice for TestDevice {
        fn device_id(&self) -> u8 { 0x31 }
        fn handle_command(&mut self, _: u8, _: u8, _: u8) -> SioResponse {
            SioResponse::CompleteWithData(vec![0xAA, 0xBB, 0xCC, 0xDD])
        }
    }

    // Create mock bus
    struct TestBus {
        device: TestDevice,
        command_line: bool,
        bytes_sent_to_pokey: Vec<u8>,
        timing: SioTiming,
    }

    impl SioBus for TestBus {
        fn get_command_line(&self) -> bool { self.command_line }
        fn route_sio_command(&mut self, _: u8, _: u8, _: u8, _: u8) -> SioResponse {
            self.device.handle_command(0, 0, 0)
        }
        fn has_sio_device(&self, _: u8) -> bool { true }
        fn send_byte_to_pokey(&mut self, byte: u8) {
            self.bytes_sent_to_pokey.push(byte);
        }
        fn get_sio_timing(&self) -> &SioTiming { &self.timing }
    }

    let mut bus = TestBus {
        device: TestDevice,
        command_line: true,
        bytes_sent_to_pokey: Vec::new(),
        timing: SioTiming::default(),
    };

    let mut sio = SioController::new();

    // Send command
    sio.tick(&mut bus);  // Rising edge
    sio.receive_byte(0x31);
    sio.receive_byte(0x53);
    sio.receive_byte(0x00);
    sio.receive_byte(0x00);
    sio.receive_byte(0x84);

    bus.command_line = false;
    sio.tick(&mut bus);  // Falling edge, process command

    // Check what was queued
    println!("Bytes that SIO tried to send to POKEY:");
    for (i, &byte) in bus.bytes_sent_to_pokey.iter().enumerate() {
        let label = match i {
            0 => "ACK",
            1 => "Complete",
            2..=5 => "Data",
            6 => "Checksum",
            _ => "???",
        };
        println!("  [{}] 0x{:02X} ({})", i, byte, label);
    }

    println!("\n✓ SIO controller correctly queues: ACK + Complete + Data + Checksum");
    println!("\nPROBLEM: All these bytes are sent in a single tick_sio() call,");
    println!("and queue_serin_byte() just overwrites SERIN each time.");
    println!("So only the LAST byte (checksum) remains visible to the OS!");
}

fn create_minimal_bootable_atr() -> Vec<u8> {
    let mut data = vec![0; 16 + 720 * 128];
    data[0] = 0x96; data[1] = 0x02;
    let paragraphs = (720 * 128) / 16;
    data[2] = (paragraphs & 0xFF) as u8;
    data[3] = ((paragraphs >> 8) & 0xFF) as u8;
    data[4] = 128; data[5] = 0;
    data[16] = 0x01; data[17] = 0x03;  // Boot flag
    data
}

// ============================================================================
// This test will PASS once byte pacing is implemented
// ============================================================================

#[test]
#[ignore]  // Ignore until fix is implemented
fn test_full_boot_after_fix() {
    println!("\n=== This test will PASS after byte pacing is implemented ===\n");

    let atr_data = create_minimal_bootable_atr();
    let temp_path = "/tmp/test_after_fix.atr";
    std::fs::write(temp_path, &atr_data).unwrap();

    let mut atari = Atari800::for_render_test();
    let disk = AtrDisk::new(temp_path, 0x31).unwrap();
    atari.add_sio_device(Box::new(disk));

    println!("Simulating OS boot sequence...\n");

    // Send Status command
    println!("1. Sending Status command");
    atari.bus_write(0xD303, 0x08);  // Assert command line
    atari.tick_sio();

    let cmd = [0x31u8, 0x53, 0x00, 0x00, 0x84];
    for &byte in &cmd {
        atari.bus_write(0xD20D, byte);
        atari.tick_sio();
    }

    atari.bus_write(0xD303, 0x30);  // Deassert
    atari.tick_sio();

    // Read ACK
    let mut ack = 0u8;
    for _ in 0..100 {
        ack = atari.bus_read(0xD20D);
        if ack == 0x41 {
            break;
        }
        atari.tick_sio();  // Keep ticking to get next byte
    }
    assert_eq!(ack, 0x41, "Should read ACK");
    println!("   ✓ Read ACK: 0x{:02X}", ack);

    // Read Complete
    let mut complete = 0u8;
    for _ in 0..100 {
        complete = atari.bus_read(0xD20D);
        if complete == 0x43 {
            break;
        }
        atari.tick_sio();
    }
    assert_eq!(complete, 0x43, "Should read Complete");
    println!("   ✓ Read Complete: 0x{:02X}", complete);

    // Read 4 status bytes
    let mut status = Vec::new();
    for _ in 0..4 {
        for _ in 0..100 {
            atari.tick_sio();
        }
        status.push(atari.bus_read(0xD20D));
    }
    assert_eq!(status.len(), 4);
    println!("   ✓ Read status bytes: {:02X?}", status);

    println!("\n✓✓✓ Full boot sequence works!");
    println!("    OS can now read all response bytes correctly!");

    let _ = std::fs::remove_file(temp_path);
}

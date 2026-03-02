// Debug test to trace SIO boot sequence with full logging
//
// This test enables all debug logging and simulates an OS boot sequence
// to see exactly what's happening with byte transfers.

use atari800_rs::atari800::Atari800;
use atari800_rs::atrdisk::AtrDisk;


#[test]
fn test_debug_sio_boot_sequence() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║ DEBUG TEST: Tracing SIO Boot Sequence                           ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");

    // Create minimal bootable ATR
    let atr_data = create_bootable_atr();
    let temp_path = "/tmp/test_debug_boot.atr";
    std::fs::write(temp_path, &atr_data).unwrap();

    // Create Atari800 system
    let mut atari = Atari800::for_render_test();

    // Add disk drive D1:
    println!("Adding disk drive D1: ({} sectors, {} bytes/sector)\n",
             720, 128);
    let disk = AtrDisk::new(temp_path, 0x31).expect("Failed to load ATR");
    atari.add_sio_device(Box::new(disk));

    // Enable SIO debug logging
    atari.enable_sio_debug();

    println!("═══════════════════════════════════════════════════════════════\n");
    println!("Simulating OS Boot Sequence\n");
    println!("═══════════════════════════════════════════════════════════════\n");

    // ========================================================================
    // STEP 1: OS sends Status command to check if disk is present
    // ========================================================================
    println!("╔═══ STEP 1: STATUS COMMAND ═══╗\n");

    // Assert command line
    println!("[OS] Writing to PIA PBCTL ($D303) = $08 (assert command line)");
    atari.bus_write(0xD303, 0x08);
    atari.tick_sio();

    // Send command frame: Device=$31, Command=$53 (Status), Aux1=$00, Aux2=$00
    let status_cmd = [0x31u8, 0x53, 0x00, 0x00, 0x84];  // Checksum = $84
    println!("[OS] Sending Status command frame:");
    for (i, &byte) in status_cmd.iter().enumerate() {
        let label = match i {
            0 => "Device ID",
            1 => "Command (Status)",
            2 => "AUX1",
            3 => "AUX2",
            4 => "Checksum",
            _ => "???",
        };
        println!("[OS]   Byte {}: ${:02X} ({})", i+1, byte, label);
        atari.bus_write(0xD20D, byte);  // Write to POKEY SEROUT
        atari.tick_sio();
    }

    // Deassert command line
    println!("[OS] Writing to PIA PBCTL ($D303) = $30 (deassert command line)");
    atari.bus_write(0xD303, 0x30);
    atari.tick_sio();

    println!("\n[OS] Processing command...");
    atari.tick_sio();

    println!("\n[OS] Attempting to read ACK from POKEY SERIN ($D20D)...");
    let ack = atari.bus_read(0xD20D);
    println!("[OS] Read from SERIN: ${:02X}", ack);

    if ack == 0x41 {
        println!("✓ SUCCESS: Got ACK!");
    } else {
        println!("✗ FAILURE: Expected ACK ($41), got ${:02X}", ack);
        println!("   This is likely the checksum we wrote, not the SIO response!");
    }

    println!("\n╚═══════════════════════════════╝\n");

    // ========================================================================
    // STEP 2: Attempt to read Complete byte
    // ========================================================================
    println!("╔═══ STEP 2: READ COMPLETE BYTE ═══╗\n");

    println!("[OS] Attempting to read Complete from SERIN...");
    let complete = atari.bus_read(0xD20D);
    println!("[OS] Read from SERIN: ${:02X}", complete);

    if complete == 0x43 {
        println!("✓ SUCCESS: Got Complete!");
    } else {
        println!("✗ FAILURE: Expected Complete ($43), got ${:02X}", complete);
    }

    println!("\n╚═══════════════════════════════╝\n");

    // ========================================================================
    // Analysis
    // ========================================================================
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║ ANALYSIS                                                      ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    println!("Expected sequence:");
    println!("  1. SIO queues ACK ($41)");
    println!("  2. SIO queues Complete ($43)");
    println!("  3. SIO queues 4 status data bytes");
    println!("  4. SIO queues checksum");
    println!("  5. tick_sio() sends ALL bytes to POKEY at once");
    println!("  6. Each queue_serin_byte() call OVERWRITES the previous byte");
    println!("  7. Only the LAST byte (checksum) remains in SERIN\n");

    println!("Look for the warning message:");
    println!("  '!!! WARNING: Sent N bytes to POKEY in one tick_sio() call !!!'\n");

    println!("This proves the byte transfer issue identified in DISK_BOOT_STATUS.md");

    let _ = std::fs::remove_file(temp_path);
}

fn create_bootable_atr() -> Vec<u8> {
    let mut data = vec![0; 16 + 720 * 128];

    // ATR header
    data[0] = 0x96; data[1] = 0x02;  // Magic number
    let paragraphs = (720 * 128) / 16;
    data[2] = (paragraphs & 0xFF) as u8;
    data[3] = ((paragraphs >> 8) & 0xFF) as u8;
    data[4] = 128; data[5] = 0;  // Sector size

    // Boot sector 1
    data[16] = 0x01;  // Boot flag
    data[17] = 0x03;  // 3 boot sectors
    data[18] = 0x00;  // Load address low
    data[19] = 0x07;  // Load address high ($0700)
    data[20] = 0x00;  // Init address low
    data[21] = 0x07;  // Init address high ($0700)

    data
}

/// Debug test to see what's actually in the PM scanline buffer

use atari800_rs::gtia::Gtia;

#[test]
fn debug_player_rendering() {
    let mut gtia = Gtia::new();

    // Enable players
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set up player 0 at position 100 with all bits set
    gtia.write_register(0xD000, 100);  // HPOSP0
    gtia.write_register(0xD008, 0x00); // SIZEP0 = 1x (8 pixels)
    gtia.write_register(0xD00D, 0xFF); // GRAFP0 = all bits set
    gtia.write_register(0xD012, 0xFE); // COLPM0 = bright color

    // Create an empty scanline
    let scanline = [0u8; 384];

    // Render it
    gtia.render_scanline(50, &scanline, 0x02, None);

    // Check the framebuffer to see what was actually rendered
    println!("\nPixels around position 100:");
    for x in 95..115 {
        let (r, g, b) = gtia.framebuffer.get_pixel(32 + x, 50);
        if r > 100 || g > 100 || b > 100 {
            println!("  x={}: RGB({}, {}, {}) - VISIBLE", x, r, g, b);
        } else {
            println!("  x={}: RGB({}, {}, {}) - dark", x, r, g, b);
        }
    }
}

#[test]
fn debug_player_2x_rendering() {
    let mut gtia = Gtia::new();

    // Enable players
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set up player 1 at position 100 with 2x width
    gtia.write_register(0xD001, 100);  // HPOSP1
    gtia.write_register(0xD009, 0x01); // SIZEP1 = 2x (16 pixels)
    gtia.write_register(0xD00E, 0xFF); // GRAFP1 = all bits set
    gtia.write_register(0xD013, 0xFE); // COLPM1 = bright color

    // Create an empty scanline
    let scanline = [0u8; 384];

    // Render it
    gtia.render_scanline(50, &scanline, 0x02, None);

    // Check the framebuffer to see what was actually rendered
    println!("\nPixels around position 100 (2x width, should be 16 pixels):");
    for x in 95..120 {
        let (r, g, b) = gtia.framebuffer.get_pixel(32 + x, 50);
        if r > 100 || g > 100 || b > 100 {
            println!("  x={}: RGB({}, {}, {}) - VISIBLE", x, r, g, b);
        } else {
            println!("  x={}: RGB({}, {}, {}) - dark", x, r, g, b);
        }
    }
}

#[test]
fn debug_player_pattern_rendering() {
    let mut gtia = Gtia::new();

    // Enable players
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set up player with specific pattern
    gtia.write_register(0xD000, 100);  // HPOSP0
    gtia.write_register(0xD008, 0x00); // SIZEP0 = 1x
    gtia.write_register(0xD00D, 0b10101010); // GRAFP0 = alternating bits
    gtia.write_register(0xD012, 0xFE); // COLPM0 = bright color

    // Create an empty scanline
    let scanline = [0u8; 384];

    // Render it
    gtia.render_scanline(50, &scanline, 0x02, None);

    // Check the framebuffer
    println!("\nPixels with pattern 0b10101010 (should alternate on/off):");
    for x in 100..108 {
        let (r, g, b) = gtia.framebuffer.get_pixel(32 + x, 50);
        if r > 100 || g > 100 || b > 100 {
            println!("  x={}: VISIBLE", x);
        } else {
            println!("  x={}: dark", x);
        }
    }
}

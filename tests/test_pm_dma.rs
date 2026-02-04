/// Unit tests for Player-Missile DMA functionality

use atari800_rs::antic::Antic;
use atari800_rs::gtia::Gtia;
use atari800_rs::mem::Mem;

#[test]
fn test_pm_dma_single_line_mode() {
    let mut mem = Mem::new(0, false);  // All RAM
    let mut antic = Antic::new();
    let mut gtia = Gtia::new();

    // Set up PM base address at $2000
    antic.write_register(0xD407, 0x20);  // PMBASE = 0x20 (base address $2000)

    // Enable PM DMA: missiles + players, single-line mode
    // Bit 2 = player DMA, Bit 3 = missile DMA, Bit 4 = 0 (single-line)
    antic.write_register(0xD400, 0x0C);  // DMACTL = 0b00001100

    // Set up player 0 data in memory
    // Single-line mode: P0 starts at PMBASE*256 + $400 = $2000 + $400 = $2400
    // We'll set scanline 100 to have pattern 0xFF
    mem.set_byte(0x2400 + 100, 0xFF);  // P0 scanline 100

    // Set up player positions and sizes
    gtia.write_register(0xD000, 50);   // HPOSP0 at x=50
    gtia.write_register(0xD008, 0x00); // SIZEP0 = 1x
    gtia.write_register(0xD012, 0xFE); // COLPM0 = bright color

    // Enable players
    gtia.write_register(0xD01D, 0b10);  // GRACTL

    // Set ANTIC scanline to 100
    antic.set_scanline_for_test(100);

    // Fetch PM data from memory
    antic.fetch_pm_data(&mem);

    // Verify PM data was fetched
    assert_eq!(antic.pm_data[1], 0xFF, "P0 data should be 0xFF at scanline 100");

    // Render the scanline with PM DMA data
    let scanline = [0u8; 384];
    gtia.render_scanline(100, &scanline, 0x02, Some(&antic.pm_data));

    // Check that player was rendered
    let (r, g, b) = gtia.framebuffer.get_pixel(32 + 50, 100);
    assert!(r > 200 || g > 200 || b > 200,
        "Player should be visible from PM DMA data: RGB({},{},{})", r, g, b);
}

#[test]
fn test_pm_dma_double_line_mode() {
    let mut mem = Mem::new(0, false);  // All RAM
    let mut antic = Antic::new();
    let mut gtia = Gtia::new();

    // Set up PM base address at $2000
    antic.write_register(0xD407, 0x20);  // PMBASE

    // Enable PM DMA: players, double-line mode
    // Bit 2 = player DMA, Bit 4 = 1 (double-line)
    antic.write_register(0xD400, 0x14);  // DMACTL = 0b00010100

    // Set up player 0 data in memory
    // Double-line mode: P0 starts at PMBASE*256 + $200 = $2000 + $200 = $2200
    // Scanline 100 / 2 = 50
    mem.set_byte(0x2200 + 50, 0xFF);  // P0 double-line 50 (scanlines 100-101)

    // Set up player
    gtia.write_register(0xD000, 50);   // HPOSP0
    gtia.write_register(0xD008, 0x00); // SIZEP0 = 1x
    gtia.write_register(0xD012, 0xFE); // COLPM0
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set ANTIC scanline to 100
    antic.set_scanline_for_test(100);

    // Fetch PM data
    antic.fetch_pm_data(&mem);

    // Verify PM data was fetched
    assert_eq!(antic.pm_data[1], 0xFF, "P0 data should be 0xFF in double-line mode");

    // Render scanline
    let scanline = [0u8; 384];
    gtia.render_scanline(100, &scanline, 0x02, Some(&antic.pm_data));

    // Check rendering
    let (r, g, b) = gtia.framebuffer.get_pixel(32 + 50, 100);
    assert!(r > 200 || g > 200 || b > 200,
        "Player should be visible in double-line mode: RGB({},{},{})", r, g, b);
}

#[test]
fn test_pm_dma_missiles() {
    let mut mem = Mem::new(0, false);
    let mut antic = Antic::new();
    let mut gtia = Gtia::new();

    // Set up PM base address
    antic.write_register(0xD407, 0x20);  // PMBASE = $2000

    // Enable missile DMA only, single-line mode
    antic.write_register(0xD400, 0x08);  // DMACTL = 0b00001000 (bit 3 = missiles)

    // Set up missile data in memory
    // Single-line mode: Missiles at PMBASE*256 + $300 = $2300
    mem.set_byte(0x2300 + 100, 0xFF);  // All missiles visible at scanline 100

    // Set up missiles
    gtia.write_register(0xD004, 60);   // HPOSM0
    gtia.write_register(0xD00C, 0x00); // SIZEM = 1x
    gtia.write_register(0xD012, 0xFE); // COLPM0
    gtia.write_register(0xD01D, 0b01);  // GRACTL - enable missiles

    // Set ANTIC scanline
    antic.set_scanline_for_test(100);

    // Fetch PM data
    antic.fetch_pm_data(&mem);

    // Verify missile data was fetched
    assert_eq!(antic.pm_data[0], 0xFF, "Missile data should be 0xFF");

    // Render scanline
    let scanline = [0u8; 384];
    gtia.render_scanline(100, &scanline, 0x02, Some(&antic.pm_data));

    // Check rendering
    let (r, g, b) = gtia.framebuffer.get_pixel(32 + 60, 100);
    assert!(r > 100 || g > 100 || b > 100,
        "Missile should be visible from DMA: RGB({},{},{})", r, g, b);
}

#[test]
fn test_pm_dma_disabled() {
    let mut mem = Mem::new(0, false);
    let mut antic = Antic::new();

    // Set up PM data in memory but don't enable DMA
    mem.set_byte(0x2400 + 100, 0xFF);

    // PMBASE set but DMA not enabled
    antic.write_register(0xD407, 0x20);
    antic.write_register(0xD400, 0x00);  // DMACTL = 0 (no PM DMA)

    antic.set_scanline_for_test(100);
    antic.fetch_pm_data(&mem);

    // PM data should be empty (not fetched)
    assert_eq!(antic.pm_data[1], 0x00, "P0 data should be 0 when DMA disabled");
}

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
    // Bit 2 = player DMA, Bit 3 = missile DMA, Bit 4 = 1 (single-line resolution)
    antic.write_register(0xD400, 0x1C);  // DMACTL = 0b00011100

    // Set up player 0 data in memory
    // Single-line mode: P0 starts at PMBASE*256 + $400 = $2000 + $400 = $2400
    // Visible scanline 100 maps to PM offset 108 (100 + 8 VBLANK lines)
    mem.set_byte(0x2400 + 108, 0xFF);  // P0 at visible scanline 100

    // Set up player positions and sizes
    gtia.write_register(0xD000, 50);   // HPOSP0 at x=50
    gtia.write_register(0xD008, 0x00); // SIZEP0 = 1x
    gtia.write_register(0xD012, 0xFE); // COLPM0 = bright color

    // Enable players
    gtia.write_register(0xD01D, 0b10);  // GRACTL

    // Set ANTIC scanline to 100
    antic.set_scanline_for_test(100);

    // Fetch PM data from memory (pass scanline for correct offset)
    antic.fetch_pm_data(&mem, 100);

    // Verify PM data was fetched
    assert_eq!(antic.pm_data[1], 0xFF, "P0 data should be 0xFF at scanline 100");

    // Apply PM DMA data to registers and render the scanline
    let scanline = [0u8; 384];
    gtia.apply_pm_dma(&antic.pm_data, 100);
    gtia.render_scanline(100, &scanline, 0x02, 0x22);

    // Check that player was rendered
    // HPOS=50 -> screen_x = (50-48)*2 = 4, framebuffer_x = 32 + 4 = 36
    let (r, g, b) = gtia.framebuffer.get_pixel(36, 100);
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
    // Bit 2 = player DMA, Bit 4 = 0 (double-line resolution)
    antic.write_register(0xD400, 0x04);  // DMACTL = 0b00000100

    // Set up player 0 data in memory
    // Double-line mode: P0 starts at PMBASE*256 + $200 = $2000 + $200 = $2200
    // Visible scanline 100 -> adjusted scanline 108, offset = 108/2 = 54
    mem.set_byte(0x2200 + 54, 0xFF);  // P0 double-line 54 (visible scanlines 100-101)

    // Set up player
    gtia.write_register(0xD000, 50);   // HPOSP0
    gtia.write_register(0xD008, 0x00); // SIZEP0 = 1x
    gtia.write_register(0xD012, 0xFE); // COLPM0
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set ANTIC scanline to 100
    antic.set_scanline_for_test(100);

    // Fetch PM data (pass scanline for correct offset)
    antic.fetch_pm_data(&mem, 100);

    // Verify PM data was fetched
    assert_eq!(antic.pm_data[1], 0xFF, "P0 data should be 0xFF in double-line mode");

    // Apply PM DMA data to registers and render scanline
    let scanline = [0u8; 384];
    gtia.apply_pm_dma(&antic.pm_data, 100);
    gtia.render_scanline(100, &scanline, 0x02, 0x22);

    // Check rendering
    // HPOS=50 -> screen_x = (50-48)*2 = 4, framebuffer_x = 32 + 4 = 36
    let (r, g, b) = gtia.framebuffer.get_pixel(36, 100);
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
    // Bit 3 = missile DMA, Bit 4 = 1 (single-line resolution)
    antic.write_register(0xD400, 0x18);  // DMACTL = 0b00011000

    // Set up missile data in memory
    // Single-line mode: Missiles at PMBASE*256 + $300 = $2300
    // Visible scanline 100 maps to PM offset 108 (100 + 8 VBLANK lines)
    mem.set_byte(0x2300 + 108, 0xFF);  // All missiles visible at scanline 100

    // Set up missiles
    gtia.write_register(0xD004, 60);   // HPOSM0
    gtia.write_register(0xD00C, 0x00); // SIZEM = 1x
    gtia.write_register(0xD012, 0xFE); // COLPM0
    gtia.write_register(0xD01D, 0b01);  // GRACTL - enable missiles

    // Set ANTIC scanline
    antic.set_scanline_for_test(100);

    // Fetch PM data (pass scanline for correct offset)
    antic.fetch_pm_data(&mem, 100);

    // Verify missile data was fetched
    assert_eq!(antic.pm_data[0], 0xFF, "Missile data should be 0xFF");

    // Apply PM DMA data to registers and render scanline
    let scanline = [0u8; 384];
    gtia.apply_pm_dma(&antic.pm_data, 100);
    gtia.render_scanline(100, &scanline, 0x02, 0x22);

    // Check rendering
    // HPOS=60 -> screen_x = (60-48)*2 = 24, framebuffer_x = 32 + 24 = 56
    let (r, g, b) = gtia.framebuffer.get_pixel(56, 100);
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
    antic.fetch_pm_data(&mem, 100);

    // PM data should be empty (not fetched)
    assert_eq!(antic.pm_data[1], 0x00, "P0 data should be 0 when DMA disabled");
}

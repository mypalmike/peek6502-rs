/// Unit tests for Player-Missile graphics functionality

use atari800_rs::gtia::Gtia;

#[test]
fn test_pm_register_writes() {
    let mut gtia = Gtia::new();

    // Test HPOSP0-3 writes ($D000-$D003)
    gtia.write_register(0xD000, 0x50);
    gtia.write_register(0xD001, 0x60);
    gtia.write_register(0xD002, 0x70);
    gtia.write_register(0xD003, 0x80);

    // Test HPOSM0-3 writes ($D004-$D007)
    gtia.write_register(0xD004, 0x90);
    gtia.write_register(0xD005, 0xA0);
    gtia.write_register(0xD006, 0xB0);
    gtia.write_register(0xD007, 0xC0);

    // Test SIZEP0-3 writes ($D008-$D00B)
    gtia.write_register(0xD008, 0x00);  // 1x
    gtia.write_register(0xD009, 0x01);  // 2x
    gtia.write_register(0xD00A, 0x02);  // 1x (same as 0)
    gtia.write_register(0xD00B, 0x03);  // 4x

    // Test SIZEM write ($D00C)
    gtia.write_register(0xD00C, 0b11100100);  // M0=1x, M1=2x, M2=4x, M3=4x

    // Test GRAFP0-3 writes ($D00D-$D010)
    gtia.write_register(0xD00D, 0xFF);
    gtia.write_register(0xD00E, 0xAA);
    gtia.write_register(0xD00F, 0x55);
    gtia.write_register(0xD010, 0x00);

    // Test GRAFM write ($D011)
    gtia.write_register(0xD011, 0b11100100);

    // Verify reads return collision data (not the written values)
    // Collision registers should be 0 initially
    assert_eq!(gtia.read_register(0xD000), 0x00, "M0PF should be 0 initially");
    assert_eq!(gtia.read_register(0xD001), 0x00, "M1PF should be 0 initially");
}

#[test]
fn test_hitclr_clears_collisions() {
    let mut gtia = Gtia::new();

    // Enable players
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set up a player and playfield
    gtia.write_register(0xD000, 100);  // HPOSP0 at position 100
    gtia.write_register(0xD00D, 0xFF); // GRAFP0 = all bits set
    gtia.write_register(0xD012, 0xFF); // COLPM0 = white
    gtia.write_register(0xD016, 0x44); // COLPF0 = some color

    // Create a scanline with playfield data
    let mut scanline = [0u8; 384];
    for i in 100..108 {
        scanline[i] = 1;  // Playfield 0
    }

    // Render the scanline to trigger collisions
    gtia.render_scanline(100, &scanline, 0x02, None);

    // Check that collision was detected
    let p0pf = gtia.read_register(0xD004);
    assert_ne!(p0pf, 0, "P0PF should have collision detected");

    // Clear collisions with HITCLR
    gtia.write_register(0xD01E, 0xFF);

    // Verify all collisions are cleared
    assert_eq!(gtia.read_register(0xD000), 0, "M0PF should be cleared");
    assert_eq!(gtia.read_register(0xD001), 0, "M1PF should be cleared");
    assert_eq!(gtia.read_register(0xD002), 0, "M2PF should be cleared");
    assert_eq!(gtia.read_register(0xD003), 0, "M3PF should be cleared");
    assert_eq!(gtia.read_register(0xD004), 0, "P0PF should be cleared");
    assert_eq!(gtia.read_register(0xD005), 0, "P1PF should be cleared");
    assert_eq!(gtia.read_register(0xD006), 0, "P2PF should be cleared");
    assert_eq!(gtia.read_register(0xD007), 0, "P3PF should be cleared");
    assert_eq!(gtia.read_register(0xD008), 0, "M0PL should be cleared");
    assert_eq!(gtia.read_register(0xD009), 0, "M1PL should be cleared");
    assert_eq!(gtia.read_register(0xD00A), 0, "M2PL should be cleared");
    assert_eq!(gtia.read_register(0xD00B), 0, "M3PL should be cleared");
    assert_eq!(gtia.read_register(0xD00C), 0, "P0PL should be cleared");
    assert_eq!(gtia.read_register(0xD00D), 0, "P1PL should be cleared");
    assert_eq!(gtia.read_register(0xD00E), 0, "P2PL should be cleared");
    assert_eq!(gtia.read_register(0xD00F), 0, "P3PL should be cleared");
}

#[test]
fn test_player_to_playfield_collision() {
    let mut gtia = Gtia::new();

    // Enable players
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set up player 0 at position 100
    gtia.write_register(0xD000, 100);  // HPOSP0
    gtia.write_register(0xD008, 0x00); // SIZEP0 = 1x (8 pixels wide)
    gtia.write_register(0xD00D, 0xFF); // GRAFP0 = all bits set

    // Create a scanline with playfield 1 overlapping the player
    let mut scanline = [0u8; 384];
    for i in 102..106 {
        scanline[i] = 2;  // Playfield 1 (PF1)
    }

    // Render the scanline
    gtia.render_scanline(50, &scanline, 0x02, None);

    // Check P0PF collision register
    let p0pf = gtia.read_register(0xD004);
    assert_ne!(p0pf, 0, "P0PF should detect collision");
    assert_eq!(p0pf & 0x02, 0x02, "P0PF bit 1 should be set for PF1 collision");
}

#[test]
fn test_player_to_player_collision() {
    let mut gtia = Gtia::new();

    // Enable players
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set up two overlapping players
    gtia.write_register(0xD000, 100);  // HPOSP0
    gtia.write_register(0xD001, 104);  // HPOSP1 overlaps P0
    gtia.write_register(0xD008, 0x00); // SIZEP0 = 1x
    gtia.write_register(0xD009, 0x00); // SIZEP1 = 1x
    gtia.write_register(0xD00D, 0xFF); // GRAFP0 = all bits set
    gtia.write_register(0xD00E, 0xFF); // GRAFP1 = all bits set

    // Render a scanline
    let scanline = [0u8; 384];
    gtia.render_scanline(50, &scanline, 0x02, None);

    // Check P0PL and P1PL collision registers
    let p0pl = gtia.read_register(0xD00C);
    let p1pl = gtia.read_register(0xD00D);

    assert_ne!(p0pl, 0, "P0PL should detect collision");
    assert_eq!(p0pl & 0x02, 0x02, "P0PL bit 1 should be set (collision with P1)");
    assert_ne!(p1pl, 0, "P1PL should detect collision");
    assert_eq!(p1pl & 0x01, 0x01, "P1PL bit 0 should be set (collision with P0)");
}

#[test]
fn test_missile_rendering() {
    let mut gtia = Gtia::new();

    // Enable missiles
    gtia.write_register(0xD01D, 0b01);  // GRACTL - enable missiles

    // Set up missile 0 at position 100
    gtia.write_register(0xD004, 100);  // HPOSM0
    gtia.write_register(0xD00C, 0x00); // SIZEM = all 1x
    gtia.write_register(0xD011, 0b11); // GRAFM - M0 both bits set

    // Set missile color
    gtia.write_register(0xD012, 0xFF); // COLPM0 = white

    // Render a scanline
    let scanline = [0u8; 384];
    gtia.render_scanline(50, &scanline, 0x02, None);

    // Verify framebuffer has missile pixels
    // HPOS=100 -> screen_x = (100-48)*2 = 104, framebuffer_x = 32 + 104 = 136
    let (r, g, b) = gtia.framebuffer.get_pixel(136, 50);  // 32 = OVERSCAN_LEFT

    // White should have high RGB values
    assert!(r > 200 || g > 200 || b > 200, "Missile should be visible with bright color");
}

#[test]
fn test_gractl_disable_players() {
    let mut gtia = Gtia::new();

    // Set up player 0
    gtia.write_register(0xD000, 100);  // HPOSP0
    gtia.write_register(0xD00D, 0xFF); // GRAFP0
    gtia.write_register(0xD012, 0xFF); // COLPM0 = white

    // Disable players (GRACTL bit 1 = 0)
    gtia.write_register(0xD01D, 0b00);

    // Render a scanline
    let scanline = [0u8; 384];
    gtia.render_scanline(50, &scanline, 0x02, None);

    // Get background color for comparison
    // HPOS=100 -> screen_x = (100-48)*2 = 104, framebuffer_x = 32 + 104 = 136
    let (bg_r, bg_g, bg_b) = gtia.framebuffer.get_pixel(136, 50);

    // Now enable players and render again
    gtia.write_register(0xD01D, 0b10);
    gtia.clear_framebuffer();
    gtia.render_scanline(50, &scanline, 0x02, None);

    let (p_r, p_g, p_b) = gtia.framebuffer.get_pixel(136, 50);

    // With players enabled, color should be different (brighter)
    assert!(
        p_r != bg_r || p_g != bg_g || p_b != bg_b,
        "Player should only be visible when GRACTL enables it"
    );
}

#[test]
fn test_priority_mode_0() {
    let mut gtia = Gtia::new();

    // Priority mode 0: All PM in front of all playfield
    gtia.write_register(0xD01B, 0x00);  // PRIOR = mode 0
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set up overlapping player and playfield
    gtia.write_register(0xD000, 100);  // HPOSP0
    gtia.write_register(0xD00D, 0xFF); // GRAFP0
    gtia.write_register(0xD012, 0xFE); // COLPM0 = hue 15, lum 7 (brightest yellow)
    gtia.write_register(0xD016, 0x44); // COLPF0 = darker color

    // HPOS=100 -> screen_x = (100-48)*2 = 104, player spans 104-119 (16 pixels at 1x)
    let mut scanline = [0u8; 384];
    for i in 100..120 {
        scanline[i] = 1;  // Playfield 0
    }

    gtia.render_scanline(50, &scanline, 0x02, None);

    // At position 104 (where both player and playfield exist),
    // player should be visible (mode 0 = PM in front)
    // framebuffer_x = 32 + 104 = 136
    let (r, g, b) = gtia.framebuffer.get_pixel(136, 50);

    // Debug output
    println!("Priority mode 0: pixel at (136, 50) = ({}, {}, {})", r, g, b);

    // Should see player color (bright yellow = high values)
    assert!(r > 200 || g > 200 || b > 200, "Player should be in front in priority mode 0 (got RGB {},{},{})", r, g, b);
}

#[test]
fn test_priority_mode_3() {
    let mut gtia = Gtia::new();

    // Priority mode 3: All playfield in front of all PM
    gtia.write_register(0xD01B, 0x03);  // PRIOR = mode 3
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set up overlapping player and playfield
    gtia.write_register(0xD000, 100);  // HPOSP0
    gtia.write_register(0xD00D, 0xFF); // GRAFP0
    gtia.write_register(0xD012, 0xFF); // COLPM0 = white (high values)
    gtia.write_register(0xD016, 0x22); // COLPF0 = darker color

    let mut scanline = [0u8; 384];
    for i in 95..110 {
        scanline[i] = 1;  // Playfield 0
    }

    gtia.render_scanline(50, &scanline, 0x02, None);

    // At position 100 (where both player and playfield exist),
    // playfield should be visible (mode 3 = PF in front)
    let (r, g, b) = gtia.framebuffer.get_pixel(32 + 100, 50);

    // Should see playfield color (darker, not white)
    assert!(r < 150 && g < 150 && b < 150, "Playfield should be in front in priority mode 3");
}

#[test]
fn test_vdelay_player() {
    let mut gtia = Gtia::new();

    // Enable players
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set up player 0 at position 100
    gtia.write_register(0xD000, 100);  // HPOSP0
    gtia.write_register(0xD008, 0x00); // SIZEP0 = 1x
    gtia.write_register(0xD00D, 0xFF); // GRAFP0 = all bits
    gtia.write_register(0xD012, 0xFE); // COLPM0 = bright color

    // Enable VDELAY for player 0
    gtia.write_register(0xD01C, 0x01); // VDELAY bit 0 = P0

    let scanline = [0u8; 384];

    // HPOS=100 -> screen_x = (100-48)*2 = 104, framebuffer_x = 32 + 104 = 136

    // Render even scanline (0) - should use delayed value (initially 0)
    gtia.render_scanline(0, &scanline, 0x02, None);
    let (r0, g0, b0) = gtia.framebuffer.get_pixel(136, 0);

    // Render odd scanline (1) - should use current value (0xFF) and store it
    gtia.render_scanline(1, &scanline, 0x02, None);
    let (r1, g1, b1) = gtia.framebuffer.get_pixel(136, 1);

    // Render even scanline (2) - should now use delayed value (0xFF from odd scanline)
    gtia.render_scanline(2, &scanline, 0x02, None);
    let (r2, g2, b2) = gtia.framebuffer.get_pixel(136, 2);

    // Even scanline 0: delayed value is 0 (not set yet), so should be dark
    assert!(r0 < 100 && g0 < 100 && b0 < 100,
        "Even scanline 0 should be dark (delayed value not set yet): RGB({},{},{})", r0, g0, b0);

    // Odd scanline 1: current value is 0xFF, so should be bright
    assert!(r1 > 200 || g1 > 200 || b1 > 200,
        "Odd scanline 1 should be bright (current value 0xFF): RGB({},{},{})", r1, g1, b1);

    // Even scanline 2: delayed value from scanline 1 (0xFF), so should be bright
    assert!(r2 > 200 || g2 > 200 || b2 > 200,
        "Even scanline 2 should be bright (delayed from scanline 1): RGB({},{},{})", r2, g2, b2);
}

#[test]
fn test_vdelay_disabled() {
    let mut gtia = Gtia::new();

    // Enable players
    gtia.write_register(0xD01D, 0b10);  // GRACTL - enable players

    // Set up player 0
    gtia.write_register(0xD000, 100);  // HPOSP0
    gtia.write_register(0xD008, 0x00); // SIZEP0 = 1x
    gtia.write_register(0xD00D, 0xFF); // GRAFP0 = all bits
    gtia.write_register(0xD012, 0xFE); // COLPM0 = bright color

    // VDELAY disabled (0x00)
    gtia.write_register(0xD01C, 0x00); // VDELAY = 0

    let scanline = [0u8; 384];

    // HPOS=100 -> screen_x = (100-48)*2 = 104, framebuffer_x = 32 + 104 = 136

    // Render even and odd scanlines
    gtia.render_scanline(0, &scanline, 0x02, None);
    let (r0, g0, b0) = gtia.framebuffer.get_pixel(136, 0);

    gtia.render_scanline(1, &scanline, 0x02, None);
    let (r1, g1, b1) = gtia.framebuffer.get_pixel(136, 1);

    // Both should be bright (no delay, always use current value)
    assert!(r0 > 200 || g0 > 200 || b0 > 200,
        "Even scanline should be bright when VDELAY disabled: RGB({},{},{})", r0, g0, b0);
    assert!(r1 > 200 || g1 > 200 || b1 > 200,
        "Odd scanline should be bright when VDELAY disabled: RGB({},{},{})", r1, g1, b1);
}

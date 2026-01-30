/// Input handling - SDL scancode to Atari key code mapping
///
/// Uses SDL scancodes (physical key position) rather than keycodes so that
/// modifier keys don't change which base key is detected. POKEY adds the
/// SHIFT (bit 6) and CTRL (bit 7) modifier bits to the base key code.

use sdl2::keyboard::Scancode;

// Modifier masks
pub const AKEY_SHFT: u8 = 0x40;
pub const AKEY_CTRL: u8 = 0x80;

// Letters (lowercase base codes)
pub const AKEY_A: u8 = 0x3f;
pub const AKEY_B: u8 = 0x15;
pub const AKEY_C: u8 = 0x12;
pub const AKEY_D: u8 = 0x3a;
pub const AKEY_E: u8 = 0x2a;
pub const AKEY_F: u8 = 0x38;
pub const AKEY_G: u8 = 0x3d;
pub const AKEY_H: u8 = 0x39;
pub const AKEY_I: u8 = 0x0d;
pub const AKEY_J: u8 = 0x01;
pub const AKEY_K: u8 = 0x05;
pub const AKEY_L: u8 = 0x00;
pub const AKEY_M: u8 = 0x25;
pub const AKEY_N: u8 = 0x23;
pub const AKEY_O: u8 = 0x08;
pub const AKEY_P: u8 = 0x0a;
pub const AKEY_Q: u8 = 0x2f;
pub const AKEY_R: u8 = 0x28;
pub const AKEY_S: u8 = 0x3e;
pub const AKEY_T: u8 = 0x2d;
pub const AKEY_U: u8 = 0x0b;
pub const AKEY_V: u8 = 0x10;
pub const AKEY_W: u8 = 0x2e;
pub const AKEY_X: u8 = 0x16;
pub const AKEY_Y: u8 = 0x2b;
pub const AKEY_Z: u8 = 0x17;

// Numbers
pub const AKEY_0: u8 = 0x32;
pub const AKEY_1: u8 = 0x1f;
pub const AKEY_2: u8 = 0x1e;
pub const AKEY_3: u8 = 0x1a;
pub const AKEY_4: u8 = 0x18;
pub const AKEY_5: u8 = 0x1d;
pub const AKEY_6: u8 = 0x1b;
pub const AKEY_7: u8 = 0x33;
pub const AKEY_8: u8 = 0x35;
pub const AKEY_9: u8 = 0x30;

// Special keys
pub const AKEY_SPACE: u8 = 0x21;
pub const AKEY_RETURN: u8 = 0x0c;
pub const AKEY_ESCAPE: u8 = 0x1c;
pub const AKEY_BACKSPACE: u8 = 0x34;
pub const AKEY_TAB: u8 = 0x2c;

// Punctuation / symbol keys
pub const AKEY_SEMICOLON: u8 = 0x02;
pub const AKEY_COMMA: u8 = 0x20;
pub const AKEY_PERIOD: u8 = 0x22;
pub const AKEY_SLASH: u8 = 0x26;
pub const AKEY_MINUS: u8 = 0x0e;
pub const AKEY_EQUALS: u8 = 0x0f;
pub const AKEY_ASTERISK: u8 = 0x07; // Shift+8 on Atari
pub const AKEY_PLUS: u8 = 0x06;     // Shift+= on Atari
pub const AKEY_LESSTHAN: u8 = 0x36; // Shift+, on Atari
pub const AKEY_GREATERTHAN: u8 = 0x37; // Shift+. on Atari
pub const AKEY_CAPS: u8 = 0x3c;
pub const AKEY_ATARI: u8 = 0x27;  // Inverse video key

// No key pressed
pub const AKEY_NONE: u8 = 0xFF;

/// Convert SDL scancode to Atari base key code (bits 0-5 only).
/// Modifier bits (SHIFT/CTRL) are added by POKEY based on modifier key state.
/// Returns None if the scancode has no Atari mapping.
pub fn scancode_to_atari(scancode: Scancode) -> Option<u8> {
    match scancode {
        // Letters
        Scancode::A => Some(AKEY_A),
        Scancode::B => Some(AKEY_B),
        Scancode::C => Some(AKEY_C),
        Scancode::D => Some(AKEY_D),
        Scancode::E => Some(AKEY_E),
        Scancode::F => Some(AKEY_F),
        Scancode::G => Some(AKEY_G),
        Scancode::H => Some(AKEY_H),
        Scancode::I => Some(AKEY_I),
        Scancode::J => Some(AKEY_J),
        Scancode::K => Some(AKEY_K),
        Scancode::L => Some(AKEY_L),
        Scancode::M => Some(AKEY_M),
        Scancode::N => Some(AKEY_N),
        Scancode::O => Some(AKEY_O),
        Scancode::P => Some(AKEY_P),
        Scancode::Q => Some(AKEY_Q),
        Scancode::R => Some(AKEY_R),
        Scancode::S => Some(AKEY_S),
        Scancode::T => Some(AKEY_T),
        Scancode::U => Some(AKEY_U),
        Scancode::V => Some(AKEY_V),
        Scancode::W => Some(AKEY_W),
        Scancode::X => Some(AKEY_X),
        Scancode::Y => Some(AKEY_Y),
        Scancode::Z => Some(AKEY_Z),

        // Numbers
        Scancode::Num0 => Some(AKEY_0),
        Scancode::Num1 => Some(AKEY_1),
        Scancode::Num2 => Some(AKEY_2),
        Scancode::Num3 => Some(AKEY_3),
        Scancode::Num4 => Some(AKEY_4),
        Scancode::Num5 => Some(AKEY_5),
        Scancode::Num6 => Some(AKEY_6),
        Scancode::Num7 => Some(AKEY_7),
        Scancode::Num8 => Some(AKEY_8),
        Scancode::Num9 => Some(AKEY_9),

        // Punctuation / symbols
        Scancode::Semicolon => Some(AKEY_SEMICOLON),
        Scancode::Comma => Some(AKEY_COMMA),
        Scancode::Period => Some(AKEY_PERIOD),
        Scancode::Slash => Some(AKEY_SLASH),
        Scancode::Minus => Some(AKEY_MINUS),
        Scancode::Equals => Some(AKEY_EQUALS),
        Scancode::CapsLock => Some(AKEY_CAPS),

        // Special keys
        Scancode::Space => Some(AKEY_SPACE),
        Scancode::Return => Some(AKEY_RETURN),
        Scancode::Escape => Some(AKEY_ESCAPE),
        Scancode::Backspace => Some(AKEY_BACKSPACE),
        Scancode::Tab => Some(AKEY_TAB),

        _ => None,
    }
}

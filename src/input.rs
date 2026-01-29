/// Input handling - SDL to Atari key code mapping
///
/// Maps SDL2 keyboard events to Atari 8-bit key codes.
/// Key codes are defined as in the Atari 800 hardware:
/// - Bits 0-5: Base key code
/// - Bit 6: SHIFT modifier (0x40)
/// - Bit 7: CTRL modifier (0x80)

use sdl2::keyboard::Keycode;

// Modifier masks
pub const AKEY_SHFT: u8 = 0x40;
pub const AKEY_CTRL: u8 = 0x80;

// Letters (lowercase)
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

// No key pressed
pub const AKEY_NONE: u8 = 0xFF;

/// Convert SDL keycode to Atari key code
/// Returns None if the key is not mapped
pub fn sdl_to_atari(keycode: Keycode) -> Option<u8> {
    match keycode {
        // Letters
        Keycode::A => Some(AKEY_A),
        Keycode::B => Some(AKEY_B),
        Keycode::C => Some(AKEY_C),
        Keycode::D => Some(AKEY_D),
        Keycode::E => Some(AKEY_E),
        Keycode::F => Some(AKEY_F),
        Keycode::G => Some(AKEY_G),
        Keycode::H => Some(AKEY_H),
        Keycode::I => Some(AKEY_I),
        Keycode::J => Some(AKEY_J),
        Keycode::K => Some(AKEY_K),
        Keycode::L => Some(AKEY_L),
        Keycode::M => Some(AKEY_M),
        Keycode::N => Some(AKEY_N),
        Keycode::O => Some(AKEY_O),
        Keycode::P => Some(AKEY_P),
        Keycode::Q => Some(AKEY_Q),
        Keycode::R => Some(AKEY_R),
        Keycode::S => Some(AKEY_S),
        Keycode::T => Some(AKEY_T),
        Keycode::U => Some(AKEY_U),
        Keycode::V => Some(AKEY_V),
        Keycode::W => Some(AKEY_W),
        Keycode::X => Some(AKEY_X),
        Keycode::Y => Some(AKEY_Y),
        Keycode::Z => Some(AKEY_Z),

        // Numbers
        Keycode::Num0 => Some(AKEY_0),
        Keycode::Num1 => Some(AKEY_1),
        Keycode::Num2 => Some(AKEY_2),
        Keycode::Num3 => Some(AKEY_3),
        Keycode::Num4 => Some(AKEY_4),
        Keycode::Num5 => Some(AKEY_5),
        Keycode::Num6 => Some(AKEY_6),
        Keycode::Num7 => Some(AKEY_7),
        Keycode::Num8 => Some(AKEY_8),
        Keycode::Num9 => Some(AKEY_9),

        // Special keys
        Keycode::Space => Some(AKEY_SPACE),
        Keycode::Return => Some(AKEY_RETURN),
        Keycode::Escape => Some(AKEY_ESCAPE),
        Keycode::Backspace => Some(AKEY_BACKSPACE),
        Keycode::Tab => Some(AKEY_TAB),

        // Unmapped keys
        _ => None,
    }
}

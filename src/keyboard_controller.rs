/// Keyboard-to-controller mapping for SDL frontend
///
/// Maps host keyboard keys to controller port state. Supports multiple binding
/// sets (e.g., arrows for player 1, WASD for player 2) and optional modifier
/// keys (e.g., LAlt required for Atari joystick mode).

use sdl2::keyboard::{KeyboardState, Scancode};
use crate::controller::{ControllerState, MAX_CONTROLLERS};

/// A single keyboard-to-controller binding set
pub struct KeyboardControllerBinding {
    pub port: usize,                  // which controller port (0-3)
    pub modifier: Option<Scancode>,   // if Some, keys only active when modifier is held
    pub up: Scancode,
    pub down: Scancode,
    pub left: Scancode,
    pub right: Scancode,
    pub button_a: Scancode,
    pub button_b: Option<Scancode>,
    pub select: Option<Scancode>,
    pub start: Option<Scancode>,
}

/// Manages keyboard-to-controller mappings
pub struct KeyboardController {
    bindings: Vec<KeyboardControllerBinding>,
}

impl KeyboardController {
    pub fn new(bindings: Vec<KeyboardControllerBinding>) -> Self {
        KeyboardController { bindings }
    }

    /// Default Atari binding: LAlt + arrows for d-pad, LAlt + slash for trigger, port 0
    pub fn atari_default() -> Self {
        KeyboardController::new(vec![
            KeyboardControllerBinding {
                port: 0,
                modifier: Some(Scancode::LAlt),
                up: Scancode::Up,
                down: Scancode::Down,
                left: Scancode::Left,
                right: Scancode::Right,
                button_a: Scancode::Slash,
                button_b: None,
                select: None,
                start: None,
            },
        ])
    }

    /// Read keyboard state and OR results into the port array.
    /// Uses OR semantics so keyboard and SDL gamepad merge naturally.
    pub fn read_states(&self, keyboard_state: &KeyboardState, ports: &mut [ControllerState; MAX_CONTROLLERS]) {
        for binding in &self.bindings {
            // Skip if modifier is required but not held
            if let Some(modifier) = binding.modifier {
                if !keyboard_state.is_scancode_pressed(modifier) {
                    continue;
                }
            }

            let port = &mut ports[binding.port];

            // OR digital directions
            port.up |= keyboard_state.is_scancode_pressed(binding.up);
            port.down |= keyboard_state.is_scancode_pressed(binding.down);
            port.left |= keyboard_state.is_scancode_pressed(binding.left);
            port.right |= keyboard_state.is_scancode_pressed(binding.right);

            // OR buttons
            port.button_a |= keyboard_state.is_scancode_pressed(binding.button_a);

            if let Some(b) = binding.button_b {
                port.button_b |= keyboard_state.is_scancode_pressed(b);
            }
            if let Some(s) = binding.select {
                port.select |= keyboard_state.is_scancode_pressed(s);
            }
            if let Some(s) = binding.start {
                port.start |= keyboard_state.is_scancode_pressed(s);
            }
        }
    }
}

/// Platform-agnostic controller input state
///
/// This module provides a shared abstraction for game controller input that works
/// across all emulated platforms (Atari 8-bit, C64, Apple II, NES). The emulator
/// core only deals with ControllerState; input source logic (keyboard bindings,
/// SDL gamepads) lives in the SDL frontend layer.

pub const MAX_CONTROLLERS: usize = 4;

/// State of a single controller port
#[derive(Clone, Copy, Debug, Default)]
pub struct ControllerState {
    // Digital d-pad
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,

    // Buttons (superset across platforms)
    // Atari: button_a only (trigger)
    // NES: button_a, button_b, select, start
    // C64: button_a only (fire)
    // Apple II: button_a, button_b
    pub button_a: bool,
    pub button_b: bool,
    pub select: bool,
    pub start: bool,

    // Analog axes (-1.0..1.0, for Apple II paddles / analog sticks)
    pub axis_x: f32,
    pub axis_y: f32,
}

/// Aggregated input state for all controller ports
pub struct ControllerInput {
    pub ports: [ControllerState; MAX_CONTROLLERS],
}

impl ControllerInput {
    pub fn new() -> Self {
        ControllerInput {
            ports: [ControllerState::default(); MAX_CONTROLLERS],
        }
    }

    /// Clear all controller state (call at start of each frame before merging inputs)
    pub fn clear(&mut self) {
        self.ports = [ControllerState::default(); MAX_CONTROLLERS];
    }
}

/// Joystick input abstraction for Atari 8-bit emulation
///
/// Atari joysticks connect through PIA and GTIA:
/// - PORTA ($D300): Joysticks 1 & 2 direction bits
/// - PORTB ($D302): Joysticks 3 & 4 direction bits
/// - TRIG0-3 ($D010-$D013): Trigger buttons (active low: 0=pressed, 1=released)
///
/// Each joystick uses 4 bits: up, down, left, right (active low: 0=pressed, 1=released)

/// State of a single joystick (1-4)
#[derive(Clone, Copy, Debug, Default)]
pub struct JoystickState {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub trigger: bool,
}

impl JoystickState {
    /// Convert joystick state to 4-bit value for PIA port (active low)
    /// Bit 0: right, Bit 1: left, Bit 2: down, Bit 3: up
    /// 0 = pressed, 1 = not pressed
    pub fn to_pia_bits(&self) -> u8 {
        let mut bits = 0xFF;  // All released by default
        if self.right { bits &= !0x01; }
        if self.left  { bits &= !0x02; }
        if self.down  { bits &= !0x04; }
        if self.up    { bits &= !0x08; }
        bits
    }

    /// Get trigger state (active low: 0=pressed, 1=released)
    pub fn trigger_value(&self) -> u8 {
        if self.trigger { 0 } else { 1 }
    }
}

/// Trait for joystick input sources
pub trait JoystickInput {
    /// Update joystick state (called once per frame or event loop)
    fn update(&mut self);

    /// Get current state of joystick port (1-4)
    fn get_state(&self, port: u8) -> JoystickState;

    /// Downcast helper for accessing concrete types
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Keyboard-based joystick emulation
/// Uses Option+arrow keys for direction, Option+/ for trigger
pub struct KeyboardJoystick {
    state: JoystickState,  // Port 1 joystick state
    option_pressed: bool,  // Host Option/Alt key state
}

impl KeyboardJoystick {
    pub fn new() -> Self {
        KeyboardJoystick {
            state: JoystickState::default(),
            option_pressed: false,
        }
    }

    /// Set host Option/Alt key state
    pub fn set_option(&mut self, pressed: bool) {
        self.option_pressed = pressed;
    }

    /// Handle directional key press
    pub fn handle_direction(&mut self, up: bool, down: bool, left: bool, right: bool) {
        if self.option_pressed {
            self.state.up = up;
            self.state.down = down;
            self.state.left = left;
            self.state.right = right;
        }
    }

    /// Handle trigger key
    pub fn handle_trigger(&mut self, pressed: bool) {
        if self.option_pressed {
            self.state.trigger = pressed;
        }
    }

    /// Release all directions (when Option is released)
    pub fn release_all(&mut self) {
        self.state = JoystickState::default();
    }
}

impl JoystickInput for KeyboardJoystick {
    fn update(&mut self) {
        // For keyboard input, state is updated via event handlers
        // This is a no-op, but required by trait
    }

    fn get_state(&self, port: u8) -> JoystickState {
        // Only port 1 is supported for now
        if port == 1 {
            self.state
        } else {
            JoystickState::default()
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

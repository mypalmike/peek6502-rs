/// SDL GameController manager
///
/// Handles hot-plug of SDL game controllers and reads their state into
/// the platform-agnostic ControllerState format each frame.

use sdl2::controller::GameController;
use sdl2::GameControllerSubsystem;
use crate::controller::{ControllerState, MAX_CONTROLLERS};

const ANALOG_DEADZONE: i16 = 8000; // ~0.25 of 32767

pub struct SdlControllerManager {
    subsystem: GameControllerSubsystem,
    controllers: [Option<GameController>; MAX_CONTROLLERS],
}

impl SdlControllerManager {
    pub fn new(subsystem: GameControllerSubsystem) -> Self {
        SdlControllerManager {
            subsystem,
            controllers: [None, None, None, None],
        }
    }

    /// Handle a ControllerDeviceAdded event. Opens the controller and assigns
    /// it to the first free port. `joystick_index` is the SDL joystick index
    /// from the event.
    pub fn handle_device_added(&mut self, joystick_index: u32) {
        // Find first free port
        let free_port = self.controllers.iter().position(|c| c.is_none());
        if let Some(port) = free_port {
            match self.subsystem.open(joystick_index) {
                Ok(controller) => {
                    println!("Controller '{}' connected -> port {}", controller.name(), port + 1);
                    self.controllers[port] = Some(controller);
                }
                Err(e) => {
                    eprintln!("Failed to open controller {}: {}", joystick_index, e);
                }
            }
        } else {
            eprintln!("Controller connected but all {} ports are full", MAX_CONTROLLERS);
        }
    }

    /// Handle a ControllerDeviceRemoved event. `instance_id` is the SDL
    /// instance ID from the event.
    pub fn handle_device_removed(&mut self, instance_id: u32) {
        for (port, slot) in self.controllers.iter_mut().enumerate() {
            if let Some(ref controller) = slot {
                if controller.instance_id() == instance_id {
                    println!("Controller '{}' disconnected from port {}", controller.name(), port + 1);
                    *slot = None;
                    return;
                }
            }
        }
    }

    /// Read all connected controllers into the state array.
    /// Uses OR semantics (writes into existing state).
    pub fn read_states(&self, ports: &mut [ControllerState; MAX_CONTROLLERS]) {
        for (i, slot) in self.controllers.iter().enumerate() {
            if let Some(ref controller) = slot {
                let port = &mut ports[i];

                // D-pad buttons
                use sdl2::controller::Button;
                port.up |= controller.button(Button::DPadUp);
                port.down |= controller.button(Button::DPadDown);
                port.left |= controller.button(Button::DPadLeft);
                port.right |= controller.button(Button::DPadRight);

                // Face buttons
                port.button_a |= controller.button(Button::A);
                port.button_b |= controller.button(Button::B);
                port.select |= controller.button(Button::Back);
                port.start |= controller.button(Button::Start);

                // Left analog stick -> digital directions with deadzone
                use sdl2::controller::Axis;
                let lx = controller.axis(Axis::LeftX);
                let ly = controller.axis(Axis::LeftY);

                if lx < -ANALOG_DEADZONE { port.left = true; }
                if lx > ANALOG_DEADZONE { port.right = true; }
                if ly < -ANALOG_DEADZONE { port.up = true; }
                if ly > ANALOG_DEADZONE { port.down = true; }

                // Raw analog values for platforms that need them (Apple II paddles)
                port.axis_x = lx as f32 / 32767.0;
                port.axis_y = ly as f32 / 32767.0;
            }
        }
    }
}

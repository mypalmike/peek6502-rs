// Command Executor - Executes commands on the Atari800 emulator
//
// This module provides the CommandExecutor which takes Commands and
// applies them to the running Atari800 instance.

use crate::atari800::Atari800;
use crate::atrdisk::AtrDisk;
use crate::command::{
    get_help_text, CartAction, Command, CommandResult, DiskAction, PowerAction, SpeedMode,
};
use crate::machine_config::MachineType;

/// Execution context passed to command executor
pub struct ExecutionContext<'a> {
    /// The Atari800 emulator instance
    pub atari: &'a mut Atari800,
    /// Whether the emulator is powered on
    pub powered_on: &'a mut bool,
    /// Current speed limiting state
    pub speed_limit: &'a mut bool,
}

/// Execute a command and return the result
pub fn execute(ctx: &mut ExecutionContext, cmd: Command) -> CommandResult {
    match cmd {
        Command::Power { action } => execute_power(ctx, action),
        Command::Reset => execute_reset(ctx),
        Command::SetMachine { machine_type } => execute_set_machine(ctx, &machine_type),
        Command::Disk { action, drive, path } => execute_disk(ctx, action, drive, path),
        Command::Cart { action, path } => execute_cart(ctx, action, path),
        Command::Status => execute_status(ctx),
        Command::Speed { mode } => execute_speed(ctx, mode),
        Command::Trace { count } => execute_trace(ctx, count),
        Command::Help => CommandResult::ok_with_message(get_help_text()),
    }
}

fn execute_power(ctx: &mut ExecutionContext, action: PowerAction) -> CommandResult {
    match action {
        PowerAction::On => {
            if *ctx.powered_on {
                CommandResult::ok_with_message("Already powered on")
            } else {
                *ctx.powered_on = true;
                ctx.atari.power_on();
                CommandResult::ok_with_message("Power on")
            }
        }
        PowerAction::Off => {
            if !*ctx.powered_on {
                CommandResult::ok_with_message("Already powered off")
            } else {
                *ctx.powered_on = false;
                ctx.atari.power_off();
                CommandResult::ok_with_message("Power off")
            }
        }
    }
}

fn execute_reset(ctx: &mut ExecutionContext) -> CommandResult {
    if !*ctx.powered_on {
        return CommandResult::error("Cannot reset: power is off");
    }
    ctx.atari.reset();
    CommandResult::ok_with_message("Reset complete")
}

fn execute_set_machine(ctx: &mut ExecutionContext, machine_type_str: &str) -> CommandResult {
    let machine_type = match Command::parse_machine_type(machine_type_str) {
        Ok(mt) => mt,
        Err(e) => return CommandResult::error(e),
    };

    let current_type = ctx.atari.get_machine_type();
    if machine_type == current_type {
        let name = machine_type_name(machine_type);
        return CommandResult::ok_with_message(format!("Already running {}", name));
    }

    // Collect current disk attachments before reconstruction
    let disk_paths = ctx.atari.get_disk_paths();
    let cart_path = ctx.atari.get_cart_path();

    // Reconstruct Atari800 with new machine type
    *ctx.atari = Atari800::with_config(machine_type, cart_path.as_deref());

    // Reattach disks
    for (drive, path) in disk_paths {
        if let Ok(disk) = AtrDisk::new(&path, 0x30 + drive) {
            ctx.atari.add_sio_device(Box::new(disk));
        }
    }

    let name = machine_type_name(machine_type);
    CommandResult::ok_with_message(format!("Switched to {}", name))
}

fn execute_disk(
    ctx: &mut ExecutionContext,
    action: DiskAction,
    drive: u8,
    path: Option<String>,
) -> CommandResult {
    match action {
        DiskAction::Load => {
            let path = match path {
                Some(p) => p,
                None => return CommandResult::error("No path specified"),
            };

            // Device ID is $31 for D1:, $32 for D2:, etc.
            let device_id = 0x30 + drive;

            // First eject any existing disk in this drive
            ctx.atari.eject_disk(drive);

            // Load new disk
            match AtrDisk::new(&path, device_id) {
                Ok(disk) => {
                    ctx.atari.add_sio_device(Box::new(disk));
                    CommandResult::ok_with_message(format!("Loaded D{}: {}", drive, path))
                }
                Err(e) => CommandResult::error(format!("Failed to load disk: {}", e)),
            }
        }
        DiskAction::Eject => {
            if ctx.atari.eject_disk(drive) {
                CommandResult::ok_with_message(format!("Ejected D{}:", drive))
            } else {
                CommandResult::ok_with_message(format!("D{}: was empty", drive))
            }
        }
        DiskAction::Status => {
            if let Some(path) = ctx.atari.get_disk_path(drive) {
                CommandResult::ok_with_message(format!("D{}: {}", drive, path))
            } else {
                CommandResult::ok_with_message(format!("D{}: empty", drive))
            }
        }
    }
}

fn execute_cart(
    ctx: &mut ExecutionContext,
    action: CartAction,
    path: Option<String>,
) -> CommandResult {
    match action {
        CartAction::Load => {
            let path = match path {
                Some(p) => p,
                None => return CommandResult::error("No path specified"),
            };

            match ctx.atari.load_cart(&path) {
                Ok(_) => CommandResult::ok_with_message(format!("Loaded cartridge: {}", path)),
                Err(e) => CommandResult::error(format!("Failed to load cartridge: {}", e)),
            }
        }
        CartAction::Eject => {
            ctx.atari.eject_cart();
            CommandResult::ok_with_message("Cartridge ejected")
        }
        CartAction::Status => {
            if let Some(path) = ctx.atari.get_cart_path() {
                CommandResult::ok_with_message(format!("Cartridge: {}", path))
            } else {
                CommandResult::ok_with_message("No cartridge loaded")
            }
        }
    }
}

fn execute_status(ctx: &mut ExecutionContext) -> CommandResult {
    let machine = machine_type_name(ctx.atari.get_machine_type());
    let power = if *ctx.powered_on { "on" } else { "off" };
    let speed = if *ctx.speed_limit { "normal" } else { "full" };
    let pc = ctx.atari.get_pc();
    let scanline = ctx.atari.get_scanline();

    let mut status = format!(
        "Machine: {}\nPower: {}\nSpeed: {}\nPC: ${:04X}\nScanline: {}",
        machine, power, speed, pc, scanline
    );

    // Add disk status
    for drive in 1..=4u8 {
        if let Some(path) = ctx.atari.get_disk_path(drive) {
            status.push_str(&format!("\nD{}: {}", drive, path));
        }
    }

    // Add cart status
    if let Some(path) = ctx.atari.get_cart_path() {
        status.push_str(&format!("\nCart: {}", path));
    }

    CommandResult::ok_with_message(status)
}

fn execute_speed(ctx: &mut ExecutionContext, mode: SpeedMode) -> CommandResult {
    match mode {
        SpeedMode::Full => {
            *ctx.speed_limit = false;
            CommandResult::ok_with_message("Speed: full (no limiting)")
        }
        SpeedMode::Normal => {
            *ctx.speed_limit = true;
            CommandResult::ok_with_message("Speed: normal (1.79 MHz)")
        }
    }
}

fn execute_trace(ctx: &mut ExecutionContext, count: u32) -> CommandResult {
    ctx.atari.enable_cpu_trace(count);
    CommandResult::ok_with_message(format!("Tracing next {} instructions", count))
}

fn machine_type_name(mt: MachineType) -> &'static str {
    match mt {
        MachineType::Atari800 => "Atari 800",
        MachineType::Atari800XL => "Atari 800XL",
    }
}

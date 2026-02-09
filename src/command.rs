// Command API for Quake-style console and HTTP JSON API
//
// This module provides a unified command representation that can be:
// - Parsed from text (console input)
// - Serialized/deserialized to/from JSON (HTTP API)
// - Executed by CommandExecutor

use crate::machine_config::MachineType;
use serde::{Deserialize, Serialize};

/// Commands that can be executed via console or HTTP API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    // Power control
    #[serde(rename = "power")]
    Power {
        #[serde(default)]
        action: PowerAction,
    },
    Reset,

    // Machine selection
    #[serde(rename = "machine")]
    SetMachine {
        #[serde(rename = "type")]
        machine_type: String,
    },

    // Disk management
    #[serde(rename = "disk")]
    Disk {
        action: DiskAction,
        #[serde(default = "default_drive")]
        drive: u8,
        #[serde(default)]
        path: Option<String>,
    },

    // Cartridge management
    #[serde(rename = "cart")]
    Cart {
        action: CartAction,
        #[serde(default)]
        path: Option<String>,
    },

    // State
    Status,

    // Speed control
    #[serde(rename = "speed")]
    Speed {
        mode: SpeedMode,
    },

    // Debug
    Trace {
        #[serde(default = "default_trace_count")]
        count: u32,
    },

    // Help
    Help,
}

fn default_drive() -> u8 {
    1
}

fn default_trace_count() -> u32 {
    100
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PowerAction {
    On,
    #[default]
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskAction {
    Load,
    Eject,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CartAction {
    Load,
    Eject,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeedMode {
    Full,
    Normal,
}

/// Result of command execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CommandResult {
    Ok {
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    Error {
        message: String,
    },
}

impl CommandResult {
    pub fn ok() -> Self {
        CommandResult::Ok { message: None }
    }

    pub fn ok_with_message(msg: impl Into<String>) -> Self {
        CommandResult::Ok {
            message: Some(msg.into()),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        CommandResult::Error {
            message: msg.into(),
        }
    }
}

impl Command {
    /// Parse a command from text input (console)
    ///
    /// Supported commands:
    /// - power on / power off
    /// - reset
    /// - machine 800 / machine 800xl
    /// - disk load 1 /path/to/file.atr
    /// - disk eject 1
    /// - disk status [1]
    /// - cart load /path/to/cart.rom
    /// - cart eject
    /// - cart status
    /// - status
    /// - speed full / speed normal
    /// - trace [count]
    /// - help
    pub fn parse(input: &str) -> Result<Command, String> {
        let input = input.trim();
        if input.is_empty() {
            return Err("Empty command".to_string());
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        match parts[0].to_lowercase().as_str() {
            "power" => {
                if parts.len() < 2 {
                    return Err("Usage: power on|off".to_string());
                }
                match parts[1].to_lowercase().as_str() {
                    "on" => Ok(Command::Power {
                        action: PowerAction::On,
                    }),
                    "off" => Ok(Command::Power {
                        action: PowerAction::Off,
                    }),
                    _ => Err(format!("Unknown power action: {}. Use 'on' or 'off'", parts[1])),
                }
            }

            "reset" => Ok(Command::Reset),

            "machine" => {
                if parts.len() < 2 {
                    return Err("Usage: machine 800|800xl".to_string());
                }
                Ok(Command::SetMachine {
                    machine_type: parts[1].to_string(),
                })
            }

            "disk" => {
                if parts.len() < 2 {
                    return Err("Usage: disk load|eject|status [drive] [path]".to_string());
                }
                match parts[1].to_lowercase().as_str() {
                    "load" => {
                        // disk load [drive] path
                        // Try to parse drive number, if it fails assume path
                        if parts.len() < 3 {
                            return Err("Usage: disk load [drive] <path>".to_string());
                        }

                        let (drive, path_idx) = if let Ok(d) = parts[2].parse::<u8>() {
                            if parts.len() < 4 {
                                return Err("Usage: disk load [drive] <path>".to_string());
                            }
                            (d, 3)
                        } else {
                            (1, 2)
                        };

                        if drive < 1 || drive > 8 {
                            return Err("Drive must be 1-8".to_string());
                        }

                        // Join remaining parts as path (handles spaces)
                        let path = parts[path_idx..].join(" ");
                        Ok(Command::Disk {
                            action: DiskAction::Load,
                            drive,
                            path: Some(path),
                        })
                    }
                    "eject" => {
                        let drive = if parts.len() >= 3 {
                            parts[2].parse::<u8>().unwrap_or(1)
                        } else {
                            1
                        };
                        if drive < 1 || drive > 8 {
                            return Err("Drive must be 1-8".to_string());
                        }
                        Ok(Command::Disk {
                            action: DiskAction::Eject,
                            drive,
                            path: None,
                        })
                    }
                    "status" => {
                        let drive = if parts.len() >= 3 {
                            parts[2].parse::<u8>().unwrap_or(1)
                        } else {
                            1
                        };
                        Ok(Command::Disk {
                            action: DiskAction::Status,
                            drive,
                            path: None,
                        })
                    }
                    _ => Err(format!(
                        "Unknown disk action: {}. Use 'load', 'eject', or 'status'",
                        parts[1]
                    )),
                }
            }

            "cart" => {
                if parts.len() < 2 {
                    return Err("Usage: cart load|eject|status [path]".to_string());
                }
                match parts[1].to_lowercase().as_str() {
                    "load" => {
                        if parts.len() < 3 {
                            return Err("Usage: cart load <path>".to_string());
                        }
                        let path = parts[2..].join(" ");
                        Ok(Command::Cart {
                            action: CartAction::Load,
                            path: Some(path),
                        })
                    }
                    "eject" => Ok(Command::Cart {
                        action: CartAction::Eject,
                        path: None,
                    }),
                    "status" => Ok(Command::Cart {
                        action: CartAction::Status,
                        path: None,
                    }),
                    _ => Err(format!(
                        "Unknown cart action: {}. Use 'load', 'eject', or 'status'",
                        parts[1]
                    )),
                }
            }

            "status" => Ok(Command::Status),

            "speed" => {
                if parts.len() < 2 {
                    return Err("Usage: speed full|normal".to_string());
                }
                match parts[1].to_lowercase().as_str() {
                    "full" => Ok(Command::Speed {
                        mode: SpeedMode::Full,
                    }),
                    "normal" => Ok(Command::Speed {
                        mode: SpeedMode::Normal,
                    }),
                    _ => Err(format!(
                        "Unknown speed mode: {}. Use 'full' or 'normal'",
                        parts[1]
                    )),
                }
            }

            "trace" => {
                let count = if parts.len() >= 2 {
                    parts[1].parse::<u32>().unwrap_or(100)
                } else {
                    100
                };
                Ok(Command::Trace { count })
            }

            "help" | "?" => Ok(Command::Help),

            _ => Err(format!("Unknown command: {}", parts[0])),
        }
    }

    /// Get list of available commands for tab completion
    pub fn command_names() -> &'static [&'static str] {
        &[
            "power", "reset", "machine", "disk", "cart", "status", "speed", "trace", "help",
        ]
    }

    /// Get completions for a partial command
    pub fn complete(partial: &str) -> Vec<String> {
        let partial = partial.trim().to_lowercase();
        let parts: Vec<&str> = partial.split_whitespace().collect();

        if parts.is_empty() {
            return Self::command_names()
                .iter()
                .map(|s| s.to_string())
                .collect();
        }

        // Complete first word (command name)
        if parts.len() == 1 && !partial.ends_with(' ') {
            return Self::command_names()
                .iter()
                .filter(|cmd| cmd.starts_with(&parts[0]))
                .map(|s| s.to_string())
                .collect();
        }

        // Complete subcommands
        match parts[0] {
            "power" => {
                if parts.len() == 1 || (parts.len() == 2 && !partial.ends_with(' ')) {
                    let prefix = parts.get(1).unwrap_or(&"");
                    return ["on", "off"]
                        .iter()
                        .filter(|s| s.starts_with(prefix))
                        .map(|s| format!("power {}", s))
                        .collect();
                }
            }
            "machine" => {
                if parts.len() == 1 || (parts.len() == 2 && !partial.ends_with(' ')) {
                    let prefix = parts.get(1).unwrap_or(&"");
                    return ["800", "800xl"]
                        .iter()
                        .filter(|s| s.starts_with(prefix))
                        .map(|s| format!("machine {}", s))
                        .collect();
                }
            }
            "disk" => {
                if parts.len() == 1 || (parts.len() == 2 && !partial.ends_with(' ')) {
                    let prefix = parts.get(1).unwrap_or(&"");
                    return ["load", "eject", "status"]
                        .iter()
                        .filter(|s| s.starts_with(prefix))
                        .map(|s| format!("disk {}", s))
                        .collect();
                }
            }
            "cart" => {
                if parts.len() == 1 || (parts.len() == 2 && !partial.ends_with(' ')) {
                    let prefix = parts.get(1).unwrap_or(&"");
                    return ["load", "eject", "status"]
                        .iter()
                        .filter(|s| s.starts_with(prefix))
                        .map(|s| format!("cart {}", s))
                        .collect();
                }
            }
            "speed" => {
                if parts.len() == 1 || (parts.len() == 2 && !partial.ends_with(' ')) {
                    let prefix = parts.get(1).unwrap_or(&"");
                    return ["full", "normal"]
                        .iter()
                        .filter(|s| s.starts_with(prefix))
                        .map(|s| format!("speed {}", s))
                        .collect();
                }
            }
            _ => {}
        }

        Vec::new()
    }

    /// Parse machine type string to MachineType enum
    pub fn parse_machine_type(s: &str) -> Result<MachineType, String> {
        match s.to_lowercase().as_str() {
            "800" => Ok(MachineType::Atari800),
            "800xl" | "xl" => Ok(MachineType::Atari800XL),
            _ => Err(format!("Unknown machine type: {}. Use '800' or '800xl'", s)),
        }
    }
}

/// Help text for all commands
pub fn get_help_text() -> &'static str {
    r#"Available commands:

  power on|off      - Power on/off the emulator
  reset             - Reset the CPU (warm reset)
  machine 800|800xl - Switch machine type (will reset)

  disk load [drive] <path>  - Load disk image to drive (1-8)
  disk eject [drive]        - Eject disk from drive
  disk status [drive]       - Show disk status

  cart load <path>  - Load cartridge ROM
  cart eject        - Eject cartridge
  cart status       - Show cartridge status

  status            - Show emulator status
  speed full|normal - Set emulation speed
  trace [count]     - Trace next N instructions (default: 100)

  help              - Show this help message
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_power() {
        assert_eq!(
            Command::parse("power on").unwrap(),
            Command::Power {
                action: PowerAction::On
            }
        );
        assert_eq!(
            Command::parse("power off").unwrap(),
            Command::Power {
                action: PowerAction::Off
            }
        );
        assert!(Command::parse("power").is_err());
    }

    #[test]
    fn test_parse_disk() {
        assert_eq!(
            Command::parse("disk load 1 /path/to/disk.atr").unwrap(),
            Command::Disk {
                action: DiskAction::Load,
                drive: 1,
                path: Some("/path/to/disk.atr".to_string()),
            }
        );
        assert_eq!(
            Command::parse("disk load /path/to/disk.atr").unwrap(),
            Command::Disk {
                action: DiskAction::Load,
                drive: 1,
                path: Some("/path/to/disk.atr".to_string()),
            }
        );
        assert_eq!(
            Command::parse("disk eject 2").unwrap(),
            Command::Disk {
                action: DiskAction::Eject,
                drive: 2,
                path: None,
            }
        );
    }

    #[test]
    fn test_json_serialization() {
        let cmd = Command::Status;
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("status"));

        let cmd2: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(cmd, cmd2);
    }

    #[test]
    fn test_completions() {
        let completions = Command::complete("po");
        assert!(completions.contains(&"power".to_string()));

        let completions = Command::complete("power o");
        assert!(completions.contains(&"power on".to_string()));
        assert!(completions.contains(&"power off".to_string()));
    }
}

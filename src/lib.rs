pub mod atari800;
pub mod bus;
pub mod cpu;
pub mod debugger;
pub mod framebuffer;
pub mod functional_test;
pub mod mem;
pub mod antic;
pub mod gtia;
pub mod pokey;
pub mod pia;
pub mod input;
pub mod joystick;
pub mod memory_region;
pub mod memory_map;
pub mod banking;  // RAM banking for 130XE extended memory
pub mod rom_overlay;  // ROM overlays for 800XL/130XE
pub mod machine_config;
pub mod rom_scanner;
pub mod sio;  // Serial I/O controller
pub mod sio_bus;  // Serial I/O bus abstraction
pub mod atrdisk;  // ATR disk image handler
pub mod patch;  // OS patching facility (SIO patch, etc.)
pub mod command;  // Command API for console and HTTP
pub mod command_executor;  // Command execution logic
pub mod console;  // In-emulator Quake-style console
pub mod http_api;  // HTTP JSON API server

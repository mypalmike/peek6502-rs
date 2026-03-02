pub mod atari800;
pub mod audio_buffer;
pub mod bus;
pub mod cpu;
pub mod debugger;
pub mod framebuffer;
pub mod functional_test;
pub mod mem;
pub mod antic;
pub mod gtia;
pub mod pokey;
pub mod pokey_audio;
pub mod pia;
pub mod input;
pub mod controller;
pub mod keyboard_controller;
pub mod sdl_controller;
pub mod page_table;
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

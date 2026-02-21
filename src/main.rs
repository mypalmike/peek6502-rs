use atari800_rs::atari800::Atari800;
use atari800_rs::command::Command;
use atari800_rs::command_executor::{execute, ExecutionContext};
use atari800_rs::console::{Console, ConsoleKey};
use atari800_rs::http_api::HttpApi;
use atari800_rs::machine_config::MachineType;
use atari800_rs::functional_test::FunctionalTest;
use atari800_rs::input;
use std::env;
use std::time::{Duration, Instant};
use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;

fn print_help() {
    println!("Atari 800 Emulator");
    println!();
    println!("USAGE:");
    println!("    atari800-rs [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help              Show this help message");
    println!("    -t, --test              Run 6502 functional test suite (full speed)");
    println!("    -r, --render            Render test pattern and save as image");
    println!("    -d, --debug             Run in debugger mode");
    println!("    -a, --animate           Run animated test pattern");
    println!("    -f, --fullspeed         Run at maximum speed (no speed limiting)");
    println!("    --machine <type>        Machine type: 800, 800xl (default: 800)");
    println!("    --cart1 <file>          Load cartridge ROM (8KB or 16KB, raw or .car)");
    println!("    --disk1 <file>          Attach disk image to D1: (.atr format)");
    println!("    --disk2 <file>          Attach disk image to D2: (.atr format)");
    println!("    --keyboard <mode>       Keyboard mapping mode: physical|modern (default: modern)");
    println!("    --video-scaling <num>   Video window scaling factor (default: 2.0)");
    println!("    --api [host:port]       HTTP API address (default: 127.0.0.1:8765)");
    println!("    --no-api                Disable HTTP API");
    println!();
    println!("KEYBOARD MODES:");
    println!("    --keyboard=modern (default)");
    println!("        Maps by character intent - typing produces expected characters.");
    println!("        Examples: Shift+2 = @, Shift+7 = &, apostrophe = '");
    println!("        Arrow keys map to Ctrl+cursor movement.");
    println!();
    println!("    --keyboard=physical");
    println!("        Maps by physical key position - host keys map to Atari key positions.");
    println!("        Use if you have an Atari-layout keyboard or want physical mapping.");
    println!("        CapsLock acts as Ctrl in this mode.");
    println!();
    println!("CONSOLE:");
    println!("    Press backtick (`) to toggle the in-emulator console.");
    println!("    Type 'help' for available commands.");
    println!();
    println!("SPEED LIMITING:");
    println!("    By default, the emulator runs at authentic Atari 800 speed:");
    println!("      - CPU: 1.79 MHz (NTSC)");
    println!("      - Frame rate: 59.92 FPS");
    println!("    Use --fullspeed to run as fast as possible (useful for development).");
    println!("    The --test mode always runs at full speed for faster testing.");
    println!();
    println!("EXAMPLES:");
    println!("    atari800-rs                            # Run at real-time 1.79 MHz (default)");
    println!("    atari800-rs --fullspeed                # Run at maximum speed");
    println!("    atari800-rs -f                         # Same as --fullspeed");
    println!("    atari800-rs --keyboard=physical        # Use physical key mapping");
    println!("    atari800-rs --cart1 basic.rom          # Load BASIC cartridge");
    println!("    atari800-rs --video-scaling 3.0        # Use 3x window scaling (960x576)");
    println!("    atari800-rs --test                     # Run functional tests (full speed)");
    println!("    atari800-rs --animate                  # Run animation demo");
    println!("    atari800-rs --api 0.0.0.0:8765         # Listen on all interfaces");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for help flag
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        print_help();
        return;
    }

    // Check for flags
    let run_functional_test = args.len() > 1 && (args[1] == "--test" || args[1] == "-t");
    let render_test = args.len() > 1 && (args[1] == "--render" || args[1] == "-r");
    let debugger_mode = args.len() > 1 && (args[1] == "--debug" || args[1] == "-d");
    let animate_mode = args.len() > 1 && (args[1] == "--animate" || args[1] == "-a");

    // Speed limiting flag (default: enabled, disable with --fullspeed)
    let full_speed = args.iter().any(|arg| arg == "--fullspeed" || arg == "-f");
    let speed_limit = !full_speed && !run_functional_test;  // Disable for test mode too

    // Machine type (default: 800)
    let machine_type = args.windows(2)
        .find(|w| w[0] == "--machine")
        .and_then(|w| match w[1].as_str() {
            "800" => Some(MachineType::Atari800),
            "800xl" => Some(MachineType::Atari800XL),
            _ => {
                eprintln!("Unknown machine type: {}", w[1]);
                eprintln!("Supported machine types: 800, 800xl");
                std::process::exit(1);
            }
        })
        .unwrap_or(MachineType::Atari800);  // Default to 800

    // Cartridge file
    let cart1_path = args.windows(2)
        .find(|w| w[0] == "--cart1")
        .map(|w| w[1].clone());

    // Disk images
    let disk1_path = args.windows(2)
        .find(|w| w[0] == "--disk1")
        .map(|w| w[1].clone());

    let disk2_path = args.windows(2)
        .find(|w| w[0] == "--disk2")
        .map(|w| w[1].clone());

    // Keyboard mapping mode (default: modern)
    let keyboard_mode = args.windows(2)
        .find(|w| w[0] == "--keyboard")
        .map(|w| w[1].as_str())
        .unwrap_or("modern");

    // Video scaling factor (default: 2.0)
    let video_scaling = args.windows(2)
        .find(|w| w[0] == "--video-scaling")
        .and_then(|w| w[1].parse::<f64>().ok())
        .unwrap_or(2.0);

    // HTTP API configuration
    let no_api = args.iter().any(|arg| arg == "--no-api");
    let api_addr = if no_api {
        None
    } else {
        Some(
            args.windows(2)
                .find(|w| w[0] == "--api")
                .map(|w| w[1].clone())
                .unwrap_or_else(|| "127.0.0.1:8765".to_string()),
        )
    };

    if run_functional_test {
        // Run the 6502 functional test suite
        let mut test = FunctionalTest::new();
        test.run();
    } else if render_test {
        // Render test pattern and save as image (no CPU, just ANTIC/GTIA test)
        println!("Rendering ANTIC/GTIA test pattern...");
        let mut atari800 = Atari800::for_render_test();

        // Render the screen
        atari800.render();

        // Save as PPM image
        match atari800.save_framebuffer("atari800_output.ppm") {
            Ok(_) => println!("Saved framebuffer to atari800_output.ppm"),
            Err(e) => println!("Error saving framebuffer: {}", e),
        }

        // Convert to PNG using ImageMagick if available
        println!("\nTo view the image:");
        println!("  convert atari800_output.ppm atari800_output.png");
        println!("  open atari800_output.png");
    } else if debugger_mode {
        // Run with SDL display and debugger enabled
        run_with_sdl(machine_type, speed_limit, cart1_path.as_deref(),
                     disk1_path.as_deref(), disk2_path.as_deref(),
                     keyboard_mode, video_scaling, true, api_addr.as_deref());
    } else if animate_mode {
        // Run color cycling animation test
        run_animated_test(machine_type, video_scaling);
    } else {
        // Run with SDL display and CPU execution (default)
        run_with_sdl(machine_type, speed_limit, cart1_path.as_deref(),
                     disk1_path.as_deref(), disk2_path.as_deref(),
                     keyboard_mode, video_scaling, false, api_addr.as_deref());
    }
}

fn run_with_sdl(machine_type: MachineType, speed_limit_initial: bool, cart_path: Option<&str>,
                disk1_path: Option<&str>, disk2_path: Option<&str>,
                keyboard_mode: &str, video_scaling: f64, debug_mode: bool,
                api_addr: Option<&str>) {
    // Create keyboard mapper based on mode
    let mapper: Box<dyn input::KeyboardMapper> = match keyboard_mode {
        "physical" => Box::new(input::PhysicalMapper),
        "modern" => Box::new(input::ModernMapper),
        _ => {
            eprintln!("Invalid keyboard mode '{}', using 'modern'", keyboard_mode);
            Box::new(input::ModernMapper)
        }
    };

    // Initialize SDL2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // Initialize SDL2 audio (44100 Hz mono i16)
    let audio_subsystem = sdl_context.audio().unwrap();
    let desired_spec = AudioSpecDesired {
        freq: Some(44100),
        channels: Some(1),  // mono
        samples: Some(1024),
    };
    let audio_queue: AudioQueue<i16> = audio_subsystem
        .open_queue(None, &desired_spec)
        .unwrap();
    audio_queue.resume();

    // Create window with specified scaling factor
    let window_width = (384.0 * video_scaling) as u32;
    let window_height = (240.0 * video_scaling) as u32;
    let window = video_subsystem
        .window("Atari 800 Emulator", window_width, window_height)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    // Create texture for framebuffer (384x240)
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 384, 240)
        .unwrap();

    // Create Atari800 instance
    let mut atari800 = Atari800::with_config(machine_type, cart_path);

    // Attach disk drives
    if let Some(path) = disk1_path {
        match atari800_rs::atrdisk::AtrDisk::new(path, 0x31) {
            Ok(disk) => {
                println!("Loaded D1: {}", path);
                atari800.add_sio_device(Box::new(disk));
            }
            Err(e) => eprintln!("Error loading D1: {}: {}", path, e),
        }
    }

    if let Some(path) = disk2_path {
        match atari800_rs::atrdisk::AtrDisk::new(path, 0x32) {
            Ok(disk) => {
                println!("Loaded D2: {}", path);
                atari800.add_sio_device(Box::new(disk));
            }
            Err(e) => eprintln!("Error loading D2: {}: {}", path, e),
        }
    }

    // Enable debugger if requested
    if debug_mode {
        atari800.enable_debug_mode();
    }

    // Create console
    let mut console = Console::new();

    // Start HTTP API if enabled
    let http_api = api_addr.and_then(|addr| {
        match HttpApi::start(addr) {
            Ok(api) => Some(api),
            Err(e) => {
                eprintln!("Warning: Failed to start HTTP API: {}", e);
                None
            }
        }
    });

    // Emulator state
    let mut powered_on = true;
    let mut speed_limit = speed_limit_initial;

    // Initialize timing for speed limiting
    const FRAME_RATE: f64 = 59.92;  // NTSC frame rate (for speed limiting only)
    let frame_duration = Duration::from_secs_f64(1.0 / FRAME_RATE);
    let mut next_frame_time = Instant::now();

    // Event loop
    let mut event_pump = sdl_context.event_pump().unwrap();

    'running: loop {
        // Process HTTP API requests
        if let Some(ref api) = http_api {
            while let Some(request) = api.try_recv() {
                let mut ctx = ExecutionContext {
                    atari: &mut atari800,
                    powered_on: &mut powered_on,
                    speed_limit: &mut speed_limit,
                };
                let result = execute(&mut ctx, request.command);
                let _ = request.response_tx.send(result);
            }
        }

        // Get current keyboard modifier state before processing events
        let keyboard_state = event_pump.keyboard_state();
        let shift = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LShift) ||
                   keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::RShift);
        let ctrl = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LCtrl) ||
                  keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::RCtrl);
        let host_alt = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LAlt) ||
                      keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::RAlt);

        // Update joystick Option key state and directions (only when console is hidden)
        if !console.visible {
            atari800.set_joystick_option(host_alt);
            if host_alt {
                let up = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Up);
                let down = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Down);
                let left = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Left);
                let right = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Right);
                let trigger = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::Slash);

                atari800.handle_joystick_direction(up, down, left, right);
                atari800.handle_joystick_trigger(trigger);
            }
        }

        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,

                Event::TextInput { text, .. } if console.visible => {
                    console.handle_text_input(&text);
                }

                Event::KeyDown { scancode: Some(scancode), repeat: false, .. } => {
                    // Backtick toggles console
                    if scancode == Scancode::Grave {
                        console.toggle();
                        // Enable/disable text input mode
                        if console.visible {
                            video_subsystem.text_input().start();
                        } else {
                            video_subsystem.text_input().stop();
                        }
                        continue;
                    }

                    if console.visible {
                        // Console mode - handle special keys
                        let console_key = match scancode {
                            Scancode::Return => Some(ConsoleKey::Enter),
                            Scancode::Backspace => Some(ConsoleKey::Backspace),
                            Scancode::Delete => Some(ConsoleKey::Delete),
                            Scancode::Left => Some(ConsoleKey::Left),
                            Scancode::Right => Some(ConsoleKey::Right),
                            Scancode::Home => Some(ConsoleKey::Home),
                            Scancode::End => Some(ConsoleKey::End),
                            Scancode::Up => Some(ConsoleKey::Up),
                            Scancode::Down => Some(ConsoleKey::Down),
                            Scancode::Tab => Some(ConsoleKey::Tab),
                            Scancode::Escape => Some(ConsoleKey::Escape),
                            _ => None,
                        };

                        if let Some(key) = console_key {
                            if let Some(command_text) = console.handle_key(key) {
                                // Execute command
                                match Command::parse(&command_text) {
                                    Ok(cmd) => {
                                        console.print(&format!("> {}", command_text));
                                        let mut ctx = ExecutionContext {
                                            atari: &mut atari800,
                                            powered_on: &mut powered_on,
                                            speed_limit: &mut speed_limit,
                                        };
                                        let result = execute(&mut ctx, cmd);
                                        console.print_result(&result);
                                    }
                                    Err(e) => {
                                        console.print(&format!("> {}", command_text));
                                        console.print(&format!("Error: {}", e));
                                    }
                                }
                            }
                        }
                    } else {
                        // Normal emulator mode
                        // Host Option+key: special functions
                        if host_alt {
                            match scancode {
                                Scancode::Num1 => atari800.console_press(2), // OPTION
                                Scancode::Num2 => atari800.console_press(1), // SELECT
                                Scancode::Num3 => atari800.console_press(0), // START
                                Scancode::Space => atari800.handle_key_press(input::AKEY_ATARI, shift, ctrl), // Inverse video
                                Scancode::Backspace => atari800.handle_break_key(), // Break key
                                Scancode::Tab => atari800.handle_key_press(input::AKEY_CAPS, false, false), // Caps Lock
                                _ => {}
                            }
                        } else if let Some(event) = mapper.map_key(scancode, shift, ctrl) {
                            atari800.handle_key_press(event.key_code, event.shift, event.ctrl);
                        }
                    }
                }

                Event::KeyUp { scancode: Some(scancode), .. } if !console.visible => {
                    // Release console buttons
                    match scancode {
                        Scancode::Num1 => atari800.console_release(2), // OPTION
                        Scancode::Num2 => atari800.console_release(1), // SELECT
                        Scancode::Num3 => atari800.console_release(0), // START
                        _ => {}
                    }
                    atari800.handle_key_release();
                }

                _ => {}
            }
        }

        // Run CPU until ANTIC signals frame completion (if powered on)
        if powered_on {
            // ANTIC internally manages video timing (262 scanlines = 1 frame)
            if debug_mode {
                // In debug mode, just call tick() in a loop (debugger controls execution)
                loop {
                    atari800.tick();
                }
            } else {
                // Normal mode uses tick_scanline()
                loop {
                    if atari800.tick_scanline() {
                        break;
                    }
                }
            }
        }

        // Queue audio samples for this frame.
        // Limit buffering to ~3 frames worth to prevent audio lag.
        let max_queued_bytes = 44100 * 2 * 3 / 60; // ~3 frame's worth of bytes
        if (audio_queue.size() as u32) < max_queued_bytes {
            let samples = atari800.get_audio_samples();
            if !samples.is_empty() {
                audio_queue.queue_audio(samples).ok();
            }
        }

        // Render console overlay if visible
        if console.visible {
            let console_height = 120; // Half screen height
            console.render(
                &mut atari800.gtia.framebuffer.pixels,
                384,
                240,
                console_height,
            );
        }

        // Copy framebuffer to SDL texture
        texture
            .update(None, &atari800.gtia.framebuffer.pixels, 384 * 3)
            .unwrap();

        // Draw to screen
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        // Frame rate limiting with accurate timing
        if speed_limit {
            next_frame_time += frame_duration;
            let now = Instant::now();
            if now < next_frame_time {
                std::thread::sleep(next_frame_time - now);
            } else {
                // We're running behind, skip sleep but don't fall too far behind
                next_frame_time = now;
            }
        }
    }

    println!("Shutting down...");
}

fn run_animated_test(_machine_type: MachineType, video_scaling: f64) {

    // Initialize SDL2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // Create window with specified scaling factor
    let window_width = (384.0 * video_scaling) as u32;
    let window_height = (240.0 * video_scaling) as u32;
    let window = video_subsystem
        .window("Atari 800 Emulator - Animated Test", window_width, window_height)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    // Create texture for framebuffer (384x240)
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 384, 240)
        .unwrap();

    // Create Atari800 instance for render testing (no CPU, no ROMs)
    let mut atari800 = Atari800::for_render_test();

    // Set up test pattern for animation
    atari800.setup_test_pattern();

    // Initialize timing for speed limiting
    const FRAME_RATE: f64 = 59.92;  // NTSC frame rate
    let frame_duration = Duration::from_secs_f64(1.0 / FRAME_RATE);
    let mut next_frame_time = Instant::now();

    // Event loop
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut frame_count: u8 = 0;

    'running: loop {
        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    scancode: Some(Scancode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        // Animate background color - cycle through hues
        let hue = (frame_count >> 2) & 0x0F;  // Slow down color changes
        let color_value = (hue << 4) | 0x0E;  // High luminance
        atari800.gtia.write_register(0xD018, color_value);  // COLPF2 (text background in mode 2)

        // Render frame
        atari800.render();

        // Copy framebuffer to SDL texture
        texture
            .update(None, &atari800.gtia.framebuffer.pixels, 384 * 3)
            .unwrap();

        // Draw to screen
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        // Frame rate limiting with accurate timing (always enabled for animation)
        next_frame_time += frame_duration;
        let now = Instant::now();
        if now < next_frame_time {
            std::thread::sleep(next_frame_time - now);
        } else {
            // We're running behind, skip sleep but don't fall too far behind
            next_frame_time = now;
        }

        frame_count = frame_count.wrapping_add(1);
    }

    println!("Shutting down...");
}

use atari800_rs::atari800::Atari800;
use atari800_rs::functional_test::FunctionalTest;
use atari800_rs::input;
use std::env;
use std::time::{Duration, Instant};
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

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
    println!();
    println!("SPEED LIMITING:");
    println!("    By default, the emulator runs at authentic Atari 800 speed:");
    println!("      - CPU: 1.79 MHz (NTSC)");
    println!("      - Frame rate: 59.92 FPS");
    println!("    Use --fullspeed to run as fast as possible (useful for development).");
    println!("    The --test mode always runs at full speed for faster testing.");
    println!();
    println!("EXAMPLES:");
    println!("    atari800-rs                     # Run at real-time 1.79 MHz (default)");
    println!("    atari800-rs --fullspeed         # Run at maximum speed");
    println!("    atari800-rs -f                  # Same as --fullspeed");
    println!("    atari800-rs --test              # Run functional tests (full speed)");
    println!("    atari800-rs --animate           # Run animation demo");
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

    if run_functional_test {
        // Run the 6502 functional test suite
        let mut test = FunctionalTest::new();
        test.run();
    } else if render_test {
        // Render test pattern and save as image
        println!("Rendering Atari 800 test pattern...");
        let mut atari800 = Atari800::new();

        // Render the screen
        atari800.render();

        // Save as PPM image
        match atari800.save_framebuffer("atari800_output.ppm") {
            Ok(_) => println!("✓ Saved framebuffer to atari800_output.ppm"),
            Err(e) => println!("✗ Error saving framebuffer: {}", e),
        }

        // Convert to PNG using ImageMagick if available
        println!("\nTo view the image:");
        println!("  convert atari800_output.ppm atari800_output.png");
        println!("  open atari800_output.png");
    } else if debugger_mode {
        // Run the Atari 800 emulator with debugger
        println!("Starting Atari 800 with debugger");
        let mut atari800 = Atari800::new();

        loop {
            atari800.tick();
        }
    } else if animate_mode {
        // Run color cycling animation test
        run_animated_test();
    } else {
        // Run with SDL display and CPU execution (default)
        run_with_sdl(speed_limit);
    }
}

fn run_with_sdl(speed_limit: bool) {

    // Initialize SDL2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // Create window (2x scale for better visibility)
    let window = video_subsystem
        .window("Atari 800 Emulator", 640, 384)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    // Create texture for framebuffer (320x192)
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 320, 192)
        .unwrap();

    // Create Atari800 instance
    let mut atari800 = Atari800::new();

    // Enable brief tracing after 5 seconds to see where we are
    let mut trace_enabled = false;

    // Initialize timing for speed limiting
    const FRAME_RATE: f64 = 59.92;  // NTSC frame rate (for speed limiting only)
    let frame_duration = Duration::from_secs_f64(1.0 / FRAME_RATE);
    let mut next_frame_time = Instant::now();

    // Event loop
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut frame_count: u32 = 0;

    'running: loop {
        // Get current keyboard modifier state before processing events
        let keyboard_state = event_pump.keyboard_state();
        let shift = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LShift) ||
                   keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::RShift);
        let ctrl = keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::LCtrl) ||
                  keyboard_state.is_scancode_pressed(sdl2::keyboard::Scancode::RCtrl);

        // Handle events
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,

                Event::KeyDown { keycode: Some(keycode), .. } => {
                    // Check for ESC to quit
                    if keycode == Keycode::Escape {
                        break 'running;
                    }

                    // Convert SDL keycode to Atari key code
                    if let Some(atari_key) = input::sdl_to_atari(keycode) {
                        // Send key press to emulator with current modifier state
                        atari800.handle_key_press(atari_key, shift, ctrl);
                    }
                }

                Event::KeyUp { .. } => {
                    // Don't clear KBCODE on key release - let it stay until next key press
                    // This matches Atari hardware behavior
                }

                _ => {}
            }
        }

        // Run CPU until ANTIC signals frame completion
        // ANTIC internally manages video timing (262 scanlines = 1 frame)
        loop {
            if atari800.tick_cpu() {
                // Frame complete - render and break
                atari800.render();
                break;
            }
        }

        // Trigger VBI (Vertical Blank Interrupt) - required for OS timing and disk I/O
        // Now that IRQ handling is properly implemented, VBI should work correctly
        atari800.trigger_vbi();

        // Copy framebuffer to SDL texture
        texture
            .update(None, &atari800.gtia.framebuffer.pixels, 320 * 3)
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

        frame_count += 1;

        // Enable tracing after 5 seconds (300 frames) to see where execution is
        // if frame_count == 300 && !trace_enabled {
        //     eprintln!("\n=== ENABLING CPU TRACE ===");
        //     atari800.enable_cpu_trace(100);
        //     trace_enabled = true;
        // }
    }

    println!("Shutting down...");
}

fn run_animated_test() {

    // Initialize SDL2
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    // Create window (2x scale for better visibility)
    let window = video_subsystem
        .window("Atari 800 Emulator - Animated Test", 640, 384)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();

    // Create texture for framebuffer (320x192)
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 320, 192)
        .unwrap();

    // Create Atari800 instance
    let mut atari800 = Atari800::new();

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
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        // Animate background color - cycle through hues
        let hue = (frame_count >> 2) & 0x0F;  // Slow down color changes
        let color_value = (hue << 4) | 0x0E;  // High luminance
        atari800.gtia.write_register(0xD01A, color_value);  // COLBK

        // Render frame
        atari800.render();

        // Copy framebuffer to SDL texture
        texture
            .update(None, &atari800.gtia.framebuffer.pixels, 320 * 3)
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

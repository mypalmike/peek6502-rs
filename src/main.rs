use atari800_rs::atari800::Atari800;
use atari800_rs::functional_test::FunctionalTest;
use std::env;
use sdl2::pixels::PixelFormatEnum;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for flags
    let run_functional_test = args.len() > 1 && (args[1] == "--test" || args[1] == "-t");
    let render_test = args.len() > 1 && (args[1] == "--render" || args[1] == "-r");
    let debugger_mode = args.len() > 1 && (args[1] == "--debug" || args[1] == "-d");
    let animate_mode = args.len() > 1 && (args[1] == "--animate" || args[1] == "-a");

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
        run_with_sdl();
    }
}

fn run_with_sdl() {
    println!("Starting Atari 800 with SDL display");
    println!("Press ESC to quit");
    println!();

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

        // Execute CPU for one frame (~29850 cycles at 1.79 MHz, 60 FPS)
        // This allows the OS and software to run between frames
        for _ in 0..29850 {
            atari800.tick();  // Execute one CPU instruction cycle
        }

        // Render frame
        atari800.render();

        // Trigger Vertical Blank Interrupt (VBI) - essential for OS and software
        atari800.trigger_vbi();

        // Copy framebuffer to SDL texture
        texture
            .update(None, &atari800.gtia.framebuffer.pixels, 320 * 3)
            .unwrap();

        // Draw to screen
        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        // Simple frame rate limiting (approximately 60 FPS)
        std::thread::sleep(std::time::Duration::from_millis(16));

        frame_count = frame_count.wrapping_add(1);
    }

    println!("Shutting down...");
}

fn run_animated_test() {
    println!("Starting Atari 800 with animated color test");
    println!("Press ESC to quit");
    println!();

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

        // Simple frame rate limiting (approximately 60 FPS)
        std::thread::sleep(std::time::Duration::from_millis(16));

        frame_count = frame_count.wrapping_add(1);
    }

    println!("Shutting down...");
}

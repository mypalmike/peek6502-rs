use atari800_rs::atari800::Atari800;
use atari800_rs::functional_test::FunctionalTest;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for flags
    let run_functional_test = args.len() > 1 && (args[1] == "--test" || args[1] == "-t");
    let render_test = args.len() > 1 && (args[1] == "--render" || args[1] == "-r");

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
    } else {
        // Run the Atari 800 emulator with debugger
        println!("Starting Atari 800");
        println!("Available options:");
        println!("  --test or -t      Run 6502 functional test");
        println!("  --render or -r    Render test pattern to image");
        println!();
        let mut atari800 = Atari800::new();

        loop {
            atari800.tick();
        }
    }
}

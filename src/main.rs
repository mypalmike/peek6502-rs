use atari800_rs::atari800::Atari800;
use atari800_rs::functional_test::FunctionalTest;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for --test or -t flag
    let run_functional_test = args.len() > 1 && (args[1] == "--test" || args[1] == "-t");

    if run_functional_test {
        // Run the 6502 functional test suite
        let mut test = FunctionalTest::new();
        test.run();
    } else {
        // Run the Atari 800 emulator with debugger
        println!("Starting Atari 800");
        println!("(Use --test or -t to run 6502 functional test instead)");
        println!();
        let mut atari800 = Atari800::new();

        loop {
            atari800.tick();
        }
    }
}

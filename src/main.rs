use atari800_rs::atari800::Atari800;

fn main() {
    println!("Starting Atari 800");
    let mut atari800 = Atari800::new();

    loop {
        atari800.tick();
    }
}

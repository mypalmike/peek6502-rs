use atari800_rs::atari800::Atari800;

fn main() {
    let mut atari800 = Atari800::new();

    loop {
        atari800.tick();
    }
}

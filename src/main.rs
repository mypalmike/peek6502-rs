use atari800_rs::atari800::Atari800;

fn main() {
    println!("Starting Atari 800");

    let atari800_res = Atari800::new();

    match atari800_res {
        Ok(mut atari800) => {
            loop {
                atari800.tick();
            }
        }
        Err(e) => {
            panic!("{}", e);
        }
    }

    // loop {
    //     atari800.tick();
    // }
}

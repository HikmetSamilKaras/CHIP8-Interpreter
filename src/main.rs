pub mod chip8instance;
use chip8instance::*;
fn main() {
    let file_path = std::env::args().nth(1).expect("Usage: ./chip8instance <rom-file>");
    let mut cur = Chip8Instance::from_file_path(file_path);
    cur.run();
}

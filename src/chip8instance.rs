use std::thread::sleep;
use std::time::Duration;
use minifb::*;
use rodio::{Sink, source::SineWave, Source};

pub struct Chip8Instance {
    memory: [u8; 4096],
    registers: [u8; 16],
    index_register: usize,
    delay_timer: u8,
    sound_timer: u8,
    program_counter: usize,
    stack_pointer: usize,
    stack: [usize; 16],
    display: [u32; 64 * 32],
    window: Window
}

impl Chip8Instance {
    pub fn from_file_path(file_path: String) -> Self {
        dbg!(&file_path);

        let width = 64*4; // resize according to your screen resolution and scale

        let mut cur = Self {
            memory: [0; 4096],
            registers: [0; 16],
            index_register: 0,
            delay_timer: 0,
            sound_timer: 0,
            program_counter: 0x200,
            stack_pointer: 0,
            stack: [0; 16],
            display: [0; 64*32],
            window: Window::new(
                "CHIP8",
                width,
                width/2,
                WindowOptions {
                    scale: Scale::X4 ,
                    ..WindowOptions::default()}
            ).unwrap()
        };

        // Load font set into memory
        cur.memory[0x50..=0x9F].copy_from_slice(&[
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80,  // F
        ]);

        let file_content = std::fs::read(file_path).expect("Failed to read ROM file");

        cur.memory[0x200..0x200 + file_content.len()].copy_from_slice(&file_content);

        cur.window.set_target_fps(60);

        cur
    }

    fn is_key_pressed(&self, key: u8) -> bool {
        match key {
            0x0 => self.window.is_key_down(Key::X),
            0x1 => self.window.is_key_down(Key::Key1),
            0x2 => self.window.is_key_down(Key::Key2),
            0x3 => self.window.is_key_down(Key::Key3),
            0x4 => self.window.is_key_down(Key::Q),
            0x5 => self.window.is_key_down(Key::W),
            0x6 => self.window.is_key_down(Key::E),
            0x7 => self.window.is_key_down(Key::A),
            0x8 => self.window.is_key_down(Key::S),
            0x9 => self.window.is_key_down(Key::D),
            0xA => self.window.is_key_down(Key::Z),
            0xB => self.window.is_key_down(Key::C),
            0xC => self.window.is_key_down(Key::Key4),
            0xD => self.window.is_key_down(Key::R),
            0xE => self.window.is_key_down(Key::F),
            0xF => self.window.is_key_down(Key::V),
            _ => false,
        }
    }

    fn advance_instruction(&mut self) {
        let opcode = ((self.memory[self.program_counter] as u16) << 8)
            | (self.memory[self.program_counter + 1] as u16);

        let nnn = (opcode & 0x0FFF) as usize;
        let n = (opcode & 0x000F) as u8;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let nn = (opcode & 0x00FF) as u8;

        let high_nibble = (opcode & 0xF000) >> 12;

        self.program_counter += 2;

        if opcode == 0x00E0 {
            self.display = [0; 64 * 32];
        }
        else if opcode == 0x00EE {
            self.stack_pointer -= 1;
            self.program_counter = self.stack[self.stack_pointer];
        }
        else if high_nibble == 1 {
            self.program_counter = nnn;
        }
        else if high_nibble == 2 {
            self.stack[self.stack_pointer] = self.program_counter;
            self.stack_pointer += 1;
            self.program_counter = nnn;
        }
        else if high_nibble == 3 {
            if self.registers[x] == nn {
                self.program_counter += 2;
            }
        }
        else if high_nibble == 4 {
            if self.registers[x] != nn {
                self.program_counter += 2;
            }
        }
        else if high_nibble == 5 && n == 0 {
            if self.registers[x] == self.registers[y] {
                self.program_counter += 2;
            }
        }
        else if high_nibble == 6 {
            self.registers[x] = nn;
        }
        else if high_nibble == 7 {
            self.registers[x] = self.registers[x].wrapping_add(nn);
        }
        else if high_nibble == 8 && n == 0 {
            self.registers[x] = self.registers[y];
        }
        else if high_nibble == 8 && n == 1 {
            self.registers[x] |= self.registers[y];
            self.registers[0xF] = 0;
        }
        else if high_nibble == 8 && n == 2 {
            self.registers[x] &= self.registers[y];
            self.registers[0xF] = 0;
        }
        else if high_nibble == 8 && n == 3 {
            self.registers[x] ^= self.registers[y];
            self.registers[0xF] = 0;
        }
        else if high_nibble == 8 && n == 4 {
            let (sum, overflow) = self.registers[x].overflowing_add(self.registers[y]);
            self.registers[x] = sum;
            self.registers[0xF] = overflow as u8;
        }
        else if high_nibble == 8 && n == 5 {
            let (diff, borrow) = self.registers[x].overflowing_sub(self.registers[y]);
            self.registers[x] = diff;
            self.registers[0xF] = (!borrow) as u8;
        }
        else if high_nibble == 8 && n == 6 {
            let temp = self.registers[y] & 1;
            self.registers[x] = self.registers[y] >> 1;
            self.registers[0xF] = temp;
        }
        else if high_nibble == 8 && n == 7{
            let (diff, borrow) = self.registers[y].overflowing_sub(self.registers[x]);
            self.registers[x] = diff;
            self.registers[0xF] = (!borrow) as u8;
        }
        else if high_nibble == 8 && n == 0xE {
            let temp = (self.registers[y] >> 7) & 1;
            self.registers[x] = self.registers[y] << 1;
            self.registers[0xF] = temp;
        }
        else if high_nibble == 9 && n == 0 {
            if self.registers[x] != self.registers[y] {
                self.program_counter += 2;
            }
        }
        else if high_nibble == 0xA {
            self.index_register = nnn;
        }
        else if high_nibble == 0xB {
            self.program_counter = nnn + self.registers[0] as usize;
        }
        else if high_nibble == 0xC {
            let random_byte: u8 = rand::random();
            self.registers[x] = random_byte & nn;
        }
        else if high_nibble == 0xD {
            let X = self.registers[x] as usize % 64;
            let mut Y = self.registers[y] as usize % 32;
            self.registers[0xF] = 0;

            for row in 0..n as usize {
                if Y >= 32 {
                    break;
                }
                let sprite_byte = self.memory[self.index_register + row];
                let mut cur_x = X;
                for col in 0..8 {
                    if cur_x >= 64 {
                        break;
                    }
                    let sprite_pixel = (sprite_byte >> (7 - col)) & 1;
                    let display_index = Y * 64 + cur_x;
                    if sprite_pixel == 1 {
                        if self.display[display_index] == 0xFFFFFFFF {
                            self.registers[0xF] = 1;
                        }
                        self.display[display_index] ^= 0xFFFFFFFF;
                    }
                    cur_x += 1;
                }
                Y += 1;
            }
        }
        else if high_nibble == 0xE && nn == 0x9E {
            if self.is_key_pressed(self.registers[x]) {
                self.program_counter += 2;
            }
        }
        else if high_nibble == 0xE && nn == 0xA1 {
            if !self.is_key_pressed(self.registers[x]) {
                self.program_counter += 2;
            }
        }
        else if high_nibble == 0xF && nn == 0x07 {
            self.registers[x] = self.delay_timer;
        }
        else if high_nibble == 0xF && nn == 0x0A {
            for key in 0..16 {
                if self.is_key_pressed(key) {
                    self.registers[x] = key;
                    return;
                }
            }
            self.program_counter -= 2;
        }
        else if high_nibble == 0xF && nn == 0x15 {
            self.delay_timer = self.registers[x];
        }
        else if high_nibble == 0xF && nn == 0x18 {
            self.sound_timer = self.registers[x];
        }
        else if high_nibble == 0xF && nn == 0x1E {
            let (sum, bool) = self.index_register.overflowing_add(self.registers[x] as usize);
            if bool || sum > 0xFFF {
                self.registers[0xF] = 1;
            }
            else {
                self.registers[0xF] = 0;
            }
            self.index_register = sum % 0x1000;
        }
        else if high_nibble == 0xF && nn == 0x29 {
            self.index_register = 0x50 + (self.registers[x] as usize) * 5;
        }
        else if high_nibble == 0xF && nn == 0x33 {
            let value = self.registers[x];
            self.memory[self.index_register] = value / 100;
            self.memory[self.index_register + 1] = (value / 10) % 10;
            self.memory[self.index_register + 2] = value % 10;
        }
        else if high_nibble == 0xF && nn == 0x55 {
            for i in 0..=x {
                self.memory[self.index_register] = self.registers[i];
                self.index_register += 1;
            }
        }
        else if high_nibble == 0xF && nn == 0x65 {
            for i in 0..=x {
                self.registers[i] = self.memory[self.index_register];
                self.index_register += 1;
            }
        }
        else {
            panic!("Unknown opcode: {:04X}, you may be running a rom for a superset of chip8", opcode);
        }
    }

    pub fn run(&mut self) {
        let instructions_per_one_sixtieth_second = 700/60; //recommended by guy

        let stream_handle = rodio::OutputStreamBuilder::open_default_stream()
            .expect("open default audio stream");
        let sink = Sink::connect_new(stream_handle.mixer());

        while self.window.is_open() && !self.window.is_key_down(Key::Escape) {

            for _ in 0..instructions_per_one_sixtieth_second {
                self.advance_instruction();
            }

            if self.delay_timer > 0 {
                self.delay_timer -= 1;
            }

            if self.sound_timer > 0 {
                //LEAKS BUT WHATEVER
                let source = SineWave::new(440.0).take_duration(Duration::from_millis(32)).amplify(0.2);
                sink.append(source);
                self.sound_timer -= 1;
            }
            else {
                sink.stop();
            }

            self.window.update_with_buffer(&self.display, 64, 32).unwrap();
        }
    }
}


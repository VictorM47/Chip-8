use minifb::{Key, Scale, Window, WindowOptions};
use std::env;
use std::fs;

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

struct Cpu {
    memory: [u8; 4096],
    v: [u8; 16],
    i: u16,
    pc: u16,
    stack: [u16; 16],
    sp: u8,
    delay_timer: u8,
    sound_timer: u8,
    display: [bool; WIDTH * HEIGHT],
    keypad: [bool; 16],
    rng_state: u32,
}

impl Cpu {
    pub fn new() -> Self {
        let mut cpu = Self {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: 0x200,
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            display: [false; WIDTH * HEIGHT],
            keypad: [false; 16],
            rng_state: 0x1234_5678,
        };

        let fontset: [u8; 80] = [
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
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ];
        cpu.memory[0..80].copy_from_slice(&fontset);
        cpu
    }

    pub fn load_rom(&mut self, path: &str) {
        let rom = fs::read(path).expect("Failed to read ROM file");
        assert!(0x200 + rom.len() <= 4096, "ROM exceeds available memory");
        self.memory[0x200..0x200 + rom.len()].copy_from_slice(&rom);
    }

    fn next_rand(&mut self) -> u8 {
        self.rng_state = self.rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.rng_state >> 24) as u8
    }

    pub fn fetch(&mut self) -> u16 {
        let hi = self.memory[self.pc as usize] as u16;
        let lo = self.memory[(self.pc + 1) as usize] as u16;
        self.pc += 2;
        (hi << 8) | lo
    }

    pub fn execute(&mut self, opcode: u16) {
        let op = (opcode & 0xF000) >> 12;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let n = (opcode & 0x000F) as u8;
        let nn = (opcode & 0x00FF) as u8;
        let nnn = opcode & 0x0FFF;

        match (op, x, y, n) {
            (0x0, 0x0, 0xE, 0x0) => self.display = [false; WIDTH * HEIGHT],
            (0x0, 0x0, 0xE, 0xE) => {
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize];
            }
            (0x1, _, _, _) => self.pc = nnn,
            (0x2, _, _, _) => {
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = nnn;
            }
            (0x3, _, _, _) => {
                if self.v[x] == nn {
                    self.pc += 2;
                }
            }
            (0x4, _, _, _) => {
                if self.v[x] != nn {
                    self.pc += 2;
                }
            }
            (0x5, _, _, 0x0) => {
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }
            (0x6, _, _, _) => self.v[x] = nn,
            (0x7, _, _, _) => self.v[x] = self.v[x].wrapping_add(nn),
            (0x8, _, _, 0x0) => self.v[x] = self.v[y],
            (0x8, _, _, 0x1) => self.v[x] |= self.v[y],
            (0x8, _, _, 0x2) => self.v[x] &= self.v[y],
            (0x8, _, _, 0x3) => self.v[x] ^= self.v[y],
            (0x8, _, _, 0x4) => {
                let (res, carry) = self.v[x].overflowing_add(self.v[y]);
                self.v[x] = res;
                self.v[0xF] = u8::from(carry);
            }
            (0x8, _, _, 0x5) => {
                let (res, borrow) = self.v[x].overflowing_sub(self.v[y]);
                self.v[x] = res;
                self.v[0xF] = u8::from(!borrow);
            }
            (0x8, _, _, 0x6) => {
                let lsb = self.v[x] & 0x1;
                self.v[x] >>= 1;
                self.v[0xF] = lsb;
            }
            (0x8, _, _, 0x7) => {
                let (res, borrow) = self.v[y].overflowing_sub(self.v[x]);
                self.v[x] = res;
                self.v[0xF] = u8::from(!borrow);
            }
            (0x8, _, _, 0xE) => {
                let msb = (self.v[x] & 0x80) >> 7;
                self.v[x] <<= 1;
                self.v[0xF] = msb;
            }
            (0x9, _, _, 0x0) => {
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }
            (0xA, _, _, _) => self.i = nnn,
            (0xB, _, _, _) => self.pc = nnn + self.v[0] as u16,
            (0xC, _, _, _) => self.v[x] = self.next_rand() & nn,
            (0xD, _, _, _) => {
                let vx = self.v[x] as usize;
                let vy = self.v[y] as usize;
                self.v[0xF] = 0;
                for row in 0..n {
                    let sprite_byte = self.memory[(self.i + row as u16) as usize];
                    for col in 0..8 {
                        if (sprite_byte & (0x80 >> col)) != 0 {
                            let idx = ((vy + row as usize) % HEIGHT) * WIDTH + ((vx + col) % WIDTH);
                            if self.display[idx] {
                                self.v[0xF] = 1;
                            }
                            self.display[idx] ^= true;
                        }
                    }
                }
            }
            (0xE, _, 0x9, 0xE) => {
                if self.keypad[(self.v[x] & 0x0F) as usize] {
                    self.pc += 2;
                }
            }
            (0xE, _, 0xA, 0x1) => {
                if !self.keypad[(self.v[x] & 0x0F) as usize] {
                    self.pc += 2;
                }
            }
            (0xF, _, 0x0, 0x7) => self.v[x] = self.delay_timer,
            (0xF, _, 0x0, 0xA) => {
                if let Some(key_idx) = self.keypad.iter().position(|&pressed| pressed) {
                    self.v[x] = key_idx as u8;
                } else {
                    self.pc -= 2; // Repeat instruction until a key is pressed
                }
            }
            (0xF, _, 0x1, 0x5) => self.delay_timer = self.v[x],
            (0xF, _, 0x1, 0x8) => self.sound_timer = self.v[x],
            (0xF, _, 0x1, 0xE) => self.i = self.i.wrapping_add(self.v[x] as u16),
            (0xF, _, 0x2, 0x9) => self.i = (self.v[x] & 0x0F) as u16 * 5,
            (0xF, _, 0x3, 0x3) => {
                let val = self.v[x];
                self.memory[self.i as usize] = val / 100;
                self.memory[(self.i + 1) as usize] = (val / 10) % 10;
                self.memory[(self.i + 2) as usize] = val % 10;
            }
            (0xF, _, 0x5, 0x5) => {
                for reg in 0..=x {
                    self.memory[self.i as usize + reg] = self.v[reg];
                }
            }
            (0xF, _, 0x6, 0x5) => {
                for reg in 0..=x {
                    self.v[reg] = self.memory[self.i as usize + reg];
                }
            }
            _ => eprintln!("Unknown opcode: {:#06X}", opcode),
        }
    }

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }
}

fn update_keypad(window: &Window, keypad: &mut [bool; 16]) {
    let keymap = [
        (Key::X, 0x0),
        (Key::Key1, 0x1),
        (Key::Key2, 0x2),
        (Key::Key3, 0x3),
        (Key::Q, 0x4),
        (Key::W, 0x5),
        (Key::E, 0x6),
        (Key::A, 0x7),
        (Key::S, 0x8),
        (Key::D, 0x9),
        (Key::Z, 0xA),
        (Key::C, 0xB),
        (Key::Key4, 0xC),
        (Key::R, 0xD),
        (Key::F, 0xE),
        (Key::V, 0xF),
    ];

    for (key, idx) in keymap {
        keypad[idx] = window.is_key_down(key);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run -- <path_to_rom>");
        std::process::exit(1);
    }

    let mut cpu = Cpu::new();
    cpu.load_rom(&args[1]);

    let mut buffer: [u32; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];
    let mut window = Window::new(
        "CHIP-8 Emulator (Rust)",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X16,
            ..WindowOptions::default()
        },
    )
    .expect("Failed to create window");

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        update_keypad(&window, &mut cpu.keypad);

        for _ in 0..10 {
            let opcode = cpu.fetch();
            cpu.execute(opcode);
        }
        cpu.tick_timers();

        for (i, pixel) in buffer.iter_mut().enumerate() {
            *pixel = if cpu.display[i] { 0x00FF_FFFF } else { 0x0000_0000 };
        }
        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .expect("Failed to update display buffer");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_with_carry() {
        let mut cpu = Cpu::new();
        cpu.v[0] = 200;
        cpu.v[1] = 100;
        cpu.execute(0x8014); // ADD V0, V1
        assert_eq!(cpu.v[0], 44);
        assert_eq!(cpu.v[0xF], 1);
    }

    #[test]
    fn test_bcd_conversion() {
        let mut cpu = Cpu::new();
        cpu.i = 0x300;
        cpu.v[2] = 254;
        cpu.execute(0xF233); // LD B, V2
        assert_eq!(cpu.memory[0x300], 2);
        assert_eq!(cpu.memory[0x301], 5);
        assert_eq!(cpu.memory[0x302], 4);
    }

    #[test]
    fn test_call_and_ret() {
        let mut cpu = Cpu::new();
        cpu.pc = 0x202;
        cpu.execute(0x2300); 
        assert_eq!(cpu.pc, 0x300);
        assert_eq!(cpu.sp, 1);
        cpu.execute(0x00EE); 
        assert_eq!(cpu.pc, 0x202);
        assert_eq!(cpu.sp, 0);
        }
}

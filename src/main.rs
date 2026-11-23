use minifb::{Key, Scale, Window, WindowOptions};
use std::env;
use std::fs;

const LARGURA: usize = 64;
const ALTURA: usize = 32;

struct Cpu {
    memoria: [u8; 4096],
    registradores_v: [u8; 16],
    registrador_i: u16,
    pc: u16,
    pilha: [u16; 16],
    sp: u8,
    delay_timer: u8,
    sound_timer: u8,
    tela: [bool; 2048],
}

impl Cpu {
    pub fn new() -> Self {
        let mut cpu = Self {
            memoria: [0; 4096],
            registradores_v: [0; 16],
            registrador_i: 0,
            pc: 0x200,
            pilha: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            tela: [false; 2048],
        };

        // Fontes padrão (0-F) - Essencial para jogos que mostram números
        let fontes: [u8; 80] = [
            0xF0, 0x90, 0x90, 0x90, 0xF0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xF0, 0x10, 0xF0, 0x80, 0xF0, 0xF0, 0x10, 0xF0, 0x10, 0xF0,
            0x90, 0x90, 0xF0, 0x10, 0x10, 0xF0, 0x80, 0xF0, 0x10, 0xF0, 0xF0, 0x80, 0xF0, 0x90, 0xF0, 0xF0, 0x10, 0x20, 0x40, 0x40,
            0xF0, 0x90, 0xF0, 0x90, 0xF0, 0xF0, 0x90, 0xF0, 0x10, 0xF0, 0xF0, 0x90, 0xF0, 0x90, 0x90, 0xE0, 0x90, 0xE0, 0x90, 0xE0,
            0xF0, 0x80, 0x80, 0x80, 0xF0, 0xE0, 0x90, 0x90, 0x90, 0xE0, 0xF0, 0x80, 0xF0, 0x80, 0xF0, 0xF0, 0x80, 0xF0, 0x80, 0x80,
        ];
        cpu.memoria[0..80].copy_from_slice(&fontes);
        cpu
    }

    pub fn carregar_rom(&mut self, caminho: &str) {
        let rom = fs::read(caminho).expect("Erro ao ler arquivo ROM");
        for (i, &byte) in rom.iter().enumerate() {
            self.memoria[0x200 + i] = byte;
        }
    }

    pub fn fetch(&mut self) -> u16 {
        let byte_alto = self.memoria[self.pc as usize] as u16;
        let byte_baixo = self.memoria[(self.pc + 1) as usize] as u16;
        let instrucao = (byte_alto << 8) | byte_baixo;
        self.pc += 2;
        instrucao
    }

    pub fn executar_instrucao(&mut self, instrucao: u16) {
        let op = (instrucao & 0xF000) >> 12;
        let x = ((instrucao & 0x0F00) >> 8) as usize;
        let y = ((instrucao & 0x00F0) >> 4) as usize;
        let n = (instrucao & 0x000F) as u8;
        let nn = (instrucao & 0x00FF) as u8;
        let nnn = instrucao & 0x0FFF;

        match (op, x, y, n) {
            (0, 0, 0xE, 0) => self.tela = [false; 2048], // 00E0: Clear
            (0, 0, 0xE, 0xE) => { // 00EE: Return
                self.sp -= 1;
                self.pc = self.pilha[self.sp as usize];
            }
            (1, _, _, _) => self.pc = nnn, // 1NNN: JMP
            (2, _, _, _) => { // 2NNN: CALL
                self.pilha[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = nnn;
            }
            (3, x, _, _) => if self.registradores_v[x] == nn { self.pc += 2 },
            (4, x, _, _) => if self.registradores_v[x] != nn { self.pc += 2 },
            (6, x, _, _) => self.registradores_v[x] = nn,
            (7, x, _, _) => self.registradores_v[x] = self.registradores_v[x].wrapping_add(nn),
            (0xA, _, _, _) => self.registrador_i = nnn,
            (0xD, x, y, n) => { // DXYN: DRAW
                let vx = self.registradores_v[x] as usize;
                let vy = self.registradores_v[y] as usize;
                self.registradores_v[0xF] = 0;
                for linha in 0..n {
                    let pixel_byte = self.memoria[(self.registrador_i + linha as u16) as usize];
                    for col in 0..8 {
                        if (pixel_byte & (0x80 >> col)) != 0 {
                            let idx = ((vy + linha as usize) % ALTURA) * LARGURA + ((vx + col) % LARGURA);
                            if self.tela[idx] { self.registradores_v[0xF] = 1; }
                            self.tela[idx] ^= true;
                        }
                    }
                }
            }
            _ => (), // Outros opcodes podem ser adicionados depois
        }
    }

    pub fn tick_timers(&mut self) {
        if self.delay_timer > 0 { self.delay_timer -= 1; }
        if self.sound_timer > 0 { self.sound_timer -= 1; }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 { panic!("Uso: cargo run -- <caminho_da_rom>"); }

    let mut cpu = Cpu::new();
    cpu.carregar_rom(&args[1]);

    let mut buffer: [u32; LARGURA * ALTURA] = [0; LARGURA * ALTURA];
    let mut window = Window::new("CHIP-8 Rust", LARGURA, ALTURA, 
        WindowOptions { scale: Scale::X16, ..WindowOptions::default() }).unwrap();

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Rodar vários ciclos por frame para manter a velocidade
        for _ in 0..10 {
            let inst = cpu.fetch();
            cpu.executar_instrucao(inst);
        }
        cpu.tick_timers();

        for i in 0..2048 {
            buffer[i] = if cpu.tela[i] { 0xFFFFFF } else { 0x000000 };
        }
        window.update_with_buffer(&buffer, LARGURA, ALTURA).unwrap();
    }
    }

use utils::get_nth_hex_digit;
use rand;

const NUM_ROWS: usize = 32;

pub struct Display {
    // i64 x 32 rows (64x32 monochrome)
    pub pixels: [u64; NUM_ROWS]
}

impl Display {
    pub fn new() -> Display {
        Display { pixels: [0; NUM_ROWS] }
    }

    pub fn clear(&mut self) {
        for i in 0..NUM_ROWS {
            self.pixels[i] = 0x00;
        }
    }
}

pub struct Cpu {
    pub v_reg: [u8; 0xF + 1], // 16
    pub i_reg: u16,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub prog_counter: u16,
    pub stack_pointer: u8,

    // Program/data memory starts at 0x200
    pub memory: [u8; 0xFFF + 1], // 4,096 bytes
    pub stack: [u16; 0xF + 1], // 16

    pub keys: u16, // bitfield for keys pressed
    pub running: bool, // set to false if waiting for a key press
    pub key_pause_register_to_set: u8, // register to set if waiting for key, set by 0xFx0A

    pub display: Display
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut cpu = Cpu {
            v_reg: [0; 16], i_reg: 0, delay_timer: 0, sound_timer: 0,
            prog_counter: 0, stack_pointer: 0,
            memory: [0; 4096], stack: [0; 16], keys: 0, running: true, key_pause_register_to_set: 0,
            display: Display::new()
        };

        // Add font data
        assert!(cpu.write_bytes(0,
            &[0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
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
              0xF0, 0x80, 0xF0, 0x80, 0x80] // F
        ));

        cpu
    }

    /// Set key to down, where `key` is between 0x1 and 0xF
    /// If the cpu is currently paused due to a call to 0xFx0A, it will set register Vx to key
    pub fn press_key(&mut self, key: u8) -> bool {
        if !(0x1 <= key && key <= 0xF) {
            return false
        }

        self.keys |= 0x1 << (key - 1);

        if !self.running {
            self.v_reg[self.key_pause_register_to_set as usize] = key;
            self.running = true;
        }

        true
    }

    pub fn write_bytes(&mut self, addr: u16, bytes: &[u8]) -> bool {
        let addr = addr as usize;
        if addr + bytes.len() > 0xFFF + 1 {
            return false;
        }

        for (i, &byte) in bytes.iter().enumerate() {
            self.memory[addr + i] = byte;
        }
        true
    }

    /// Execute next 2-byte instruction from memory, msb first.
    /// Only execute if `self.running` is true
    pub fn tick(&mut self) {
        // NOTE: "If a program includes sprite data, it should be padded so any
        // instructions following it will be properly situated in RAM."

        if self.running {
            let ins = (self.memory[self.prog_counter as usize] as u16) << 8
                | self.memory[self.prog_counter as usize + 1] as u16;

            self.prog_counter += 2;
            self.execute(ins);
        }
    }

    /// Execute two-byte instruction given by `instruction`
    /// Does not change program counter unless `instruction` triggers a skip or jump
    pub fn execute(&mut self, instruction: u16) {
        match instruction {
            // CLS: clear screen
            0x00E0 => self.display.clear(),
            // RET: return from subroutine
            0x00EE => {
                self.prog_counter = self.stack[self.stack_pointer as usize];
                self.stack_pointer -= 1;
            }
            // JMP 0x1nnn: set PC to nnn
            a if a < 0x2000 => self.prog_counter = a - 0x1000,
            // CALL 0x2nnn: call subroutine at nnn
            a if a < 0x3000 => {
                self.stack_pointer += 1;
                self.stack[self.stack_pointer as usize] = self.prog_counter;
                self.prog_counter = a - 0x2000;
            }
            // 0x3xkk, SE Vx, byte: Skip next instruction if Vx == kk
            a if a < 0x4000 => {
                let xkk = a - 0x3000;
                let x = xkk >> 4 * 2;
                let kk = (xkk - (x << 4 * 2)) as u8;

                if self.v_reg[x as usize] == kk {
                    self.prog_counter += 2;
                }
            }
            // 0x4xkk - SNE Vx, byte: same as above but Vx != kk
            a if a < 0x5000 => {
                let xkk = a - 0x4000;
                let x = xkk >> 4 * 2;
                let kk = (xkk - (x << 4 * 2)) as u8;

                if self.v_reg[x as usize] != kk {
                    self.prog_counter += 2;
                }
            }
            // 5xy0 - SE Vx, Vy: skip if Vx == Vy
            a if a < 0x6000 && get_nth_hex_digit(a as u32, 0) == 0 => {
                let x = get_nth_hex_digit(a as u32, 2);
                let y = get_nth_hex_digit(a as u32, 1);

                if self.v_reg[x as usize] == self.v_reg[y as usize] {
                    self.prog_counter += 2;
                }
            }
            // 6xkk - LD Vx, byte: put kk into register Vx
            a if a < 0x7000 => {
                let xkk = a - 0x6000;
                let x = get_nth_hex_digit(xkk as u32, 2) as u16;
                let kk = (xkk - (x << 4 * 2)) as u8;
                self.v_reg[x as usize] = kk;
            }
            // 7xkk - ADD Vx, byte: set Vx = Vx + kk
            a if a < 0x8000 => {
                let xkk = a - 0x7000;
                let x = get_nth_hex_digit(xkk as u32, 2) as u16;
                let kk = (xkk - (x << 4 * 2)) as u8;
                self.v_reg[x as usize] = self.v_reg[x as usize].wrapping_add(kk);
            }
            a if a < 0x9000 => {
                let x = get_nth_hex_digit(a as u32, 2);
                let y = get_nth_hex_digit(a as u32, 1);

                match get_nth_hex_digit(a as u32, 0) {
                    // 8xy0 - LD Vx, Vy: set Vx = Vy
                    0 => self.v_reg[x as usize] = self.v_reg[y as usize],
                    // 8xy1 - OR Vx, Vy: set Vx = Vx OR Vy
                    1 => self.v_reg[x as usize] = self.v_reg[x as usize] | self.v_reg[y as usize],
                    // 8xy2 - AND Vx, Vy: set Vx = Vx AND Vy
                    2 => self.v_reg[x as usize] = self.v_reg[x as usize] & self.v_reg[y as usize],
                    // 8xy3 - XOR Vx, Vy: set Vx = Vx XOR Vy
                    3 => self.v_reg[x as usize] = self.v_reg[x as usize] ^ self.v_reg[y as usize],
                    // 8xy4 - ADD Vx, Vy: set Vx = Vx + Vy, set VF = carry
                    4 => {
                        let (res, carry) = self.v_reg[x as usize].overflowing_add(self.v_reg[y as usize]);
                        self.v_reg[x as usize] = res;
                        self.v_reg[0xF] = carry as u8;
                    },
                    // 8xy5 - SUB Vx, Vy: set Vx = Vx - Vy, set VF = NOT borrow
                    5 => {
                        let (res, borrow) = self.v_reg[x as usize].overflowing_sub(self.v_reg[y as usize]);
                        self.v_reg[x as usize] = res;
                        self.v_reg[0xF] = !borrow as u8;
                    },
                    // 8xy6 - SHR Vx {, Vy}: set Vx = Vx SHR 1
                    6 => {
                        self.v_reg[0xF] = (self.v_reg[x as usize] & 1 == 1) as u8;
                        self.v_reg[x as usize] >>= 1;
                    },
                    // 8xy7 - SUBN Vx, Vy: set Vx = Vy - Vx, set VF = NOT borrow
                    7 => {
                        let (res, borrow) = self.v_reg[y as usize].overflowing_sub(self.v_reg[x as usize]);
                        self.v_reg[x as usize] = res;
                        self.v_reg[0xF] = !borrow as u8;
                    },
                    //8xyE - SHL Vx {, Vy}: set Vx = Vx SHL 1
                    0xE => {
                        self.v_reg[0xF] = (self.v_reg[x as usize] & 0b1000_0000 > 0) as u8;
                        self.v_reg[x as usize] <<= 1;
                    },
                    _ => {}
                }
            },
            // 9xy0 - SNE Vx, Vy: skip next instruction if Vx != Vy
            a if a < 0xA000 && get_nth_hex_digit(a as u32, 0) == 0 => {
                let x = get_nth_hex_digit(a as u32, 2);
                let y = get_nth_hex_digit(a as u32, 1);

                if self.v_reg[x as usize] != self.v_reg[y as usize] {
                    self.prog_counter += 2;
                }
            },
            // Annn - LD I, addr: set I = nnn
            a if a < 0xB000 => {
                let nnn = a - 0xA000;
                self.i_reg = nnn;
            },
            // Bnnn - JP V0, addr: jump to location nnn + V0
            a if a < 0xC000 => {
                let nnn = a - 0xB000;
                self.prog_counter = (self.v_reg[0] as u16) + nnn;
            },
            // Cxkk - RND Vx, byte: set Vx = random byte AND kk
            a if a < 0xD000 => {
                let xkk = a - 0xC000;
                let x = get_nth_hex_digit(xkk as u32, 2) as u16;
                let kk = (xkk - (x << 4 * 2)) as u8;
                self.v_reg[x as usize] = rand::random::<u8>() & kk;
            },
            // Dxyn - DRW Vx, Vy, nibble
            // display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
            a if a < 0xE000 => {
                let x = get_nth_hex_digit(a as u32, 2);
                let y = get_nth_hex_digit(a as u32, 1);
                let n = get_nth_hex_digit(a as u32, 0);

                let x = self.v_reg[x as usize];
                let y = self.v_reg[y as usize];

                for i in 0..n {
                    let mut line = self.memory[self.i_reg as usize + i as usize] as u64;
                    let mut mask = 0b1111_1111 as u64;
                    // Wrap sprite around edge of screen
                    let x_to_edge = 64 - x;
                    if x_to_edge < 8 {
                        line = line << (64 - 8 + x_to_edge) | line >> (8 - x_to_edge);
                        mask = mask << (64 - 8 + x_to_edge) | mask >> (8 - x_to_edge);
                    } else {
                        line = line << (64 - 8 - x);
                        mask = mask << (64 - 8 - x);
                    }
                    let overwrote = line & self.display.pixels[y as usize + i as usize] & mask != 0;
                    self.v_reg[0xF] = overwrote as u8;

                    self.display.pixels[y as usize + i as usize] ^= line;
                }
            },
            // Ex9E - SKP Vx: skip next instruction if key with the value of Vx is pressed
            a if a & 0xF0FF == 0xE09E => {
                let x = get_nth_hex_digit(a as u32, 2);

                if self.keys & ((0b0000_0001 << self.v_reg[x as usize]) as u16) != 0 {
                    self.prog_counter += 2;
                }
            },
            // ExA1 - SKNP Vx: skip next instruction if key with the value of Vx is not pressed
            a if a & 0xF0FF == 0xE0A1 => {
                let x = get_nth_hex_digit(a as u32, 2);

                if self.keys & ((0b0000_0001 << self.v_reg[x as usize]) as u16) == 0 {
                    self.prog_counter += 2;
                }
            },
            // Fx07 - LD Vx, DT: set Vx = delay timer value
            a if a & 0xF0FF == 0xF007 => {
                let x = get_nth_hex_digit(a as u32, 2);
                self.v_reg[x as usize] = self.delay_timer;
            },
            // Fx0A - LD Vx, K: wait for a key press, store the value of the key in Vx
            a if a & 0xF0FF == 0xF00A => {
                let x = get_nth_hex_digit(a as u32, 2);
                self.key_pause_register_to_set = x;
                self.running = false;
            },
            // Fx15 - LD DT, Vx: set delay timer = Vx
            a if a & 0xF0FF == 0xF015 => {
                let x = get_nth_hex_digit(a as u32, 2);
                self.delay_timer = self.v_reg[x as usize];
            },
            // Fx18 - LD ST, Vx: set sound timer = Vx
            a if a & 0xF0FF == 0xF018 => {
                let x = get_nth_hex_digit(a as u32, 2);
                self.sound_timer = self.v_reg[x as usize];
            },
            // Fx1E - ADD I, Vx: set I = I + Vx
            a if a & 0xF0FF == 0xF01E => {
                let x = get_nth_hex_digit(a as u32, 2);
                self.i_reg += self.v_reg[x as usize] as u16;
            },
            // Fx29 - LD F, Vx: set I = location of sprite for digit Vx
            a if a & 0xF0FF == 0xF029 => {
                let x = get_nth_hex_digit(a as u32, 2);
                self.i_reg = self.v_reg[x as usize] as u16 * 5;
            },
            // Fx33 - LD B, Vx: store BCD representation of Vx in memory locations I, I+1, and I+2
            a if a & 0xF0FF == 0xF033 => {
                let x = get_nth_hex_digit(a as u32, 2);
                let n = self.v_reg[x as usize];
                // Convert to string to access digits
                let s: String = n.to_string();

                for (i, ch) in s.chars().enumerate() {
                    let n = ch.to_digit(10).unwrap() as u8;
                    self.memory[self.i_reg as usize + i] = n;
                }
            },
            // Fx55 - LD [I], Vx: store registers V0 through Vx in memory starting at location I
            a if a & 0xF0FF == 0xF055 => {
                let x = get_nth_hex_digit(a as u32, 2);
                // TODO: bounds check
                for i in 0..(x + 1) {
                    self.memory[self.i_reg as usize + i as usize] = self.v_reg[i as usize];
                }
            },
            // Fx65 - LD Vx, [I]: read registers V0 through Vx from memory starting at location I
            a if a & 0xF0FF == 0xF065 => {
                let x = get_nth_hex_digit(a as u32, 2);
                // TODO: bounds check
                for i in 0..(x + 1) {
                    self.v_reg[i as usize] = self.memory[self.i_reg as usize + i as usize];
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_screen() {
        let mut cpu = Cpu::new();
        cpu.display.pixels[0] = 0xFF;
        cpu.execute(0x00E0);
        assert_eq!(cpu.display.pixels[0], 0);
    }

    #[test]
    fn ins_jmp() {
        let mut cpu = Cpu::new();
        cpu.execute(0x1234);
        assert_eq!(cpu.prog_counter, 0x0234);
    }

    #[test]
    fn ins_call() {
        let mut cpu = Cpu::new();
        let pc = 0x0200;
        cpu.prog_counter = pc;
        cpu.execute(0x2456);
        assert_eq!(cpu.stack_pointer, 1);
        assert_eq!(cpu.stack[1], pc);
        assert_eq!(cpu.prog_counter, 0x0456);
    }

    #[test]
    fn ins_se_and_sne() {
        let mut cpu = Cpu::new();
        cpu.prog_counter = 0x0200;
        cpu.v_reg[0x2] = 0x34;
        cpu.execute(0x3235);
        assert_eq!(cpu.prog_counter, 0x0200);
        cpu.execute(0x3234);
        assert_eq!(cpu.prog_counter, 0x0202);
        cpu.execute(0x3F34); // Choosing wrong register
        assert_eq!(cpu.prog_counter, 0x0202);
        // sne
        cpu.execute(0x4234);
        assert_eq!(cpu.prog_counter, 0x0202);
        cpu.execute(0x4200);
        assert_eq!(cpu.prog_counter, 0x0204);
        // se Vx Vy
        cpu.execute(0x5230);
        assert_eq!(cpu.prog_counter, 0x0204);
        cpu.v_reg[0x3] = 0x34;
        cpu.execute(0x5230);
        assert_eq!(cpu.prog_counter, 0x0206);
        // sne Vx Vy
        cpu.execute(0x9230);
        assert_eq!(cpu.prog_counter, 0x0206);
        cpu.v_reg[0x3] = 0x33;
        cpu.execute(0x9230);
        assert_eq!(cpu.prog_counter, 0x0208);
    }

    #[test]
    fn ins_ld() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6234);
        assert_eq!(cpu.v_reg[0x2], 0x34);
        cpu.execute(0xA123);
        assert_eq!(cpu.i_reg, 0x123);
    }

    #[test]
    fn ins_add() {
        let mut cpu = Cpu::new();
        cpu.execute(0x7123);
        assert_eq!(cpu.v_reg[0x1], 0x23);
        cpu.execute(0x7123);
        assert_eq!(cpu.v_reg[0x1], 0x46);
        cpu.execute(0x71FF);
        assert_eq!(cpu.v_reg[0x1], 0x45);
    }

    #[test]
    fn ins_vx_vy() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6234);
        cpu.execute(0x8420);
        assert_eq!(cpu.v_reg[0x2], cpu.v_reg[0x4]);
        assert_eq!(cpu.v_reg[0x2], 0x34);
    }

    #[test]
    fn ins_or_and_xor() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6234);
        cpu.execute(0x6356);
        cpu.execute(0x8231);
        assert_eq!(cpu.v_reg[0x2], 0x34 | 0x56);
        cpu.execute(0x6478);
        cpu.execute(0x8242);
        assert_eq!(cpu.v_reg[0x2], (0x34 | 0x56) & 0x78);
        cpu.execute(0x6523);
        cpu.execute(0x8253);
        assert_eq!(cpu.v_reg[0x2], (0x34 | 0x56) & 0x78 ^ 0x23);
    }

    #[test]
    fn ins_add_vx_vy() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6234);
        cpu.execute(0x6356);
        cpu.execute(0x8234);
        assert_eq!(cpu.v_reg[0x2], 0x34 + 0x56);
        assert_eq!(cpu.v_reg[0xF], 0);
        cpu.execute(0x63FF);
        cpu.execute(0x8234);
        assert_eq!(cpu.v_reg[0x2], (0x34 + 0x56 as u8).wrapping_add(0xFF));
        assert_eq!(cpu.v_reg[0xF], 1);
    }

    #[test]
    fn ins_add_i_vx() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6210);
        cpu.execute(0xF21E);
        assert_eq!(cpu.i_reg, 0x10);
        cpu.execute(0xF21E);
        assert_eq!(cpu.i_reg, 0x20);
    }

    #[test]
    fn ins_sub_vx_vy() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6234);
        cpu.execute(0x6356);
        cpu.execute(0x8325);
        assert_eq!(cpu.v_reg[0x3], 0x56 - 0x34);
        assert_eq!(cpu.v_reg[0xF], 1);
        cpu.execute(0x62FF);
        cpu.execute(0x8325);
        assert_eq!(cpu.v_reg[0x3], (0x56 - 0x34 as u8).wrapping_sub(0xFF));
        assert_eq!(cpu.v_reg[0xF], 0);
    }

    #[test]
    fn ins_shr() {
        let mut cpu = Cpu::new();
        cpu.execute(0x620E);
        cpu.execute(0x8206);
        assert_eq!(cpu.v_reg[0x2], 0x7);
        assert_eq!(cpu.v_reg[0xF], 0);
        cpu.execute(0x620F);
        cpu.execute(0x8206);
        assert_eq!(cpu.v_reg[0x2], 0x7);
        assert_eq!(cpu.v_reg[0xF], 1);
    }

    #[test]
    fn ins_sub_vy_vx() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6256);
        cpu.execute(0x6334);
        cpu.execute(0x8327);
        assert_eq!(cpu.v_reg[0x3], 0x56 - 0x34);
        assert_eq!(cpu.v_reg[0xF], 1);
        cpu.execute(0x6200);
        cpu.execute(0x8327);
        assert_eq!(cpu.v_reg[0x3], (0 as u8).wrapping_sub(0x56 - 0x34));
        assert_eq!(cpu.v_reg[0xF], 0);
    }

    #[test]
    fn ins_shl() {
        let mut cpu = Cpu::new();
        cpu.execute(0x627F);
        cpu.execute(0x820E);
        assert_eq!(cpu.v_reg[0x2], 0xFE);
        assert_eq!(cpu.v_reg[0xF], 0);
        cpu.execute(0x62FF);
        cpu.execute(0x820E);
        assert_eq!(cpu.v_reg[0x2], 0xFE);
        assert_eq!(cpu.v_reg[0xF], 1);
    }

    #[test]
    fn ins_jp() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6034);
        cpu.execute(0xB123);
        assert_eq!(cpu.prog_counter, 0x34 + 0x123);
    }

    #[test]
    fn ins_draw() {
        let sprite = &[
            0b0101_1110,
            0b0101_1001,
            0b0000_0001
        ];

        let mut cpu = Cpu::new();
        cpu.write_bytes(0x234, sprite);
        cpu.execute(0xA234);

        let x = 6;
        let y = 10;
        cpu.execute(0x6000 + x);
        cpu.execute(0x6100 + y);
        cpu.execute(0xD013);

        assert_eq!(cpu.v_reg[0xF], 0);
        for i in 0..3 {
            println!("{:b}", cpu.display.pixels[y as usize + i]);
            assert_eq!(cpu.display.pixels[y as usize + i], (sprite[i] as u64) << (64 - 8 - x));
        }

        // Overwriting current pixels
        cpu.execute(0xD013);
        assert_eq!(cpu.v_reg[0xF], 1);
        for i in 0..3 {
            assert_eq!(cpu.display.pixels[y as usize + i], 0);
        }

        // Wrapping around edge of screen
        let x = 60;
        let x_to_edge = 64 - x;
        cpu.execute(0x6000 + x);
        cpu.execute(0xD013);
        for i in 0..3 {
            assert_eq!(cpu.display.pixels[y as usize + i],
                       (sprite[i] as u64) >> (8 - x_to_edge) | (sprite[i] as u64) << (64 - 8 + x_to_edge));
        }
        assert_eq!(cpu.v_reg[0xF], 0);

        // Overwriting current pixels
        cpu.execute(0xD013);
        assert_eq!(cpu.v_reg[0xF], 1);
        for i in 0..3 {
            assert_eq!(cpu.display.pixels[y as usize + i], 0);
        }
    }

    #[test]
    fn ins_skp() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6504);
        cpu.execute(0xE59E);
        assert_eq!(cpu.prog_counter, 0);
        cpu.keys = 0b0000_0000_0001_0100;
        cpu.execute(0xE59E);
        assert_eq!(cpu.prog_counter, 2);
        cpu.execute(0x6304);
        cpu.execute(0xE3A1);
        assert_eq!(cpu.prog_counter, 2);
        cpu.execute(0x6303);
        cpu.execute(0xE3A1);
        assert_eq!(cpu.prog_counter, 4);
    }

    #[test]
    fn ins_wait_for_keypress() {
        let mut cpu = Cpu::new();
        cpu.execute(0xF30A);
        assert!(!cpu.running);
        assert_eq!(cpu.key_pause_register_to_set, 0x3);
        cpu.press_key(0xA);
        assert_eq!(cpu.v_reg[0x3], 0xA);
        assert!(cpu.running);
    }

    #[test]
    fn ins_ld_dt_st() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6304);
        cpu.execute(0xF315);
        assert_eq!(cpu.delay_timer, 0x4);
        cpu.execute(0xF318);
        assert_eq!(cpu.sound_timer, 0x4);
    }

    #[test]
    fn ins_ld_font_sprite() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6504);
        cpu.execute(0xF529);
        assert_eq!(cpu.i_reg, 4 * 5);
    }

    #[test]
    fn ins_ld_bcd() {
        let mut cpu = Cpu::new();
        cpu.execute(0x65FF);
        cpu.execute(0xA200);
        cpu.execute(0xF533);
        assert_eq!(cpu.memory[0x200], 2);
        assert_eq!(cpu.memory[0x201], 5);
        assert_eq!(cpu.memory[0x202], 5);
    }

    #[test]
    fn ins_ld_registers() {
        let mut cpu = Cpu::new();
        cpu.execute(0x6012);
        cpu.execute(0x6113);
        cpu.execute(0x6214);
        cpu.execute(0xA200);
        cpu.execute(0xF255);
        assert_eq!(cpu.memory[0x200], 0x12);
        assert_eq!(cpu.memory[0x201], 0x13);
        assert_eq!(cpu.memory[0x202], 0x14);
        assert_eq!(cpu.memory[0x203], 0);
        cpu.execute(0x6000);
        cpu.execute(0x6100);
        cpu.execute(0x6200);
        cpu.execute(0xF265);
        assert_eq!(cpu.v_reg[0], 0x12);
        assert_eq!(cpu.v_reg[1], 0x13);
        assert_eq!(cpu.v_reg[2], 0x14);
        assert_eq!(cpu.v_reg[3], 0);
    }

    #[test]
    fn writing_bytes() {
        let bytes = &[0x33, 0x45, 0x70, 0x33, 0x87, 0x29];
        let mut cpu = Cpu::new();
        assert!(cpu.write_bytes(0x200, bytes));

        for i in 0..3 {
            assert_eq!(cpu.memory[0x200 + i], bytes[i])
        }

        assert!(!cpu.write_bytes(0xFFC, bytes));
    }

    #[test]
    fn press_key() {
        let mut cpu = Cpu::new();
        assert!(cpu.press_key(0x3));
        assert_eq!(cpu.keys, 0b0000_0000_0000_0100);
        assert!(!cpu.press_key(0x0));
        assert!(!cpu.press_key(0x10));
    }

    #[test]
    fn tick() {
        let mut cpu = Cpu::new();
        cpu.prog_counter = 0x200;
        cpu.memory[0x200] = 0x63;
        cpu.memory[0x201] = 0x12;
        cpu.tick();
        assert_eq!(cpu.prog_counter, 0x202);
        assert_eq!(cpu.v_reg[3], 0x12);
        cpu.running = false;
        cpu.tick();
        assert_eq!(cpu.prog_counter, 0x202);
    }
}

use utils::get_nth_hex_digit;
use display::Display;
use rand;

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

    pub display: Display
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            v_reg: [0; 16], i_reg: 0, delay_timer: 0, sound_timer: 0,
            prog_counter: 0, stack_pointer: 0,
            memory: [0; 4096], stack: [0; 16], display: Display::new()
        }
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
    fn writing_bytes() {
        let bytes = &[0x33, 0x45, 0x70, 0x33, 0x87, 0x29];
        let mut cpu = Cpu::new();
        assert!(cpu.write_bytes(0x200, bytes));

        for i in 0..3 {
            assert_eq!(cpu.memory[0x200 + i], bytes[i])
        }

        assert!(!cpu.write_bytes(0xFFC, bytes));
    }
}
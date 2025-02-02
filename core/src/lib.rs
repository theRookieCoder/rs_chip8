#![no_std]

mod default_font;

use heapless::Vec;

pub const DISPLAY_WIDTH: usize = 128;
pub const DISPLAY_HEIGHT: usize = 64;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("Stack overflowed!")]
    StackOverflow,

    #[error("Illegal instruction: {0:04X}")]
    IllegalInstruction(u16),

    #[error("Program exited")]
    ProgramExited,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EmulationSystem {
    #[default]
    Chip8,
    SuperChip,
}

#[derive(Debug, Clone)]
pub struct MachineState {
    system: EmulationSystem,

    pub display_buffer: [[bool; DISPLAY_HEIGHT]; DISPLAY_WIDTH],

    ram: [u8; 4096],

    program_counter: u16,
    index_register: u16,
    var_registers: [u8; 16],

    stack: Vec<u16, 16>,

    delay_timer: u8,
    pub sound_timer: u8,

    previous_keystate: u16,

    high_res: bool,
}

impl Default for MachineState {
    fn default() -> Self {
        Self {
            system: EmulationSystem::default(),

            display_buffer: [[false; DISPLAY_HEIGHT]; DISPLAY_WIDTH],

            ram: [0; 4096],

            program_counter: 0x200,
            index_register: 0,
            var_registers: [0; 16],

            stack: Vec::new(),

            delay_timer: 0,
            sound_timer: 0,

            previous_keystate: 0,

            high_res: false,
        }
    }
}

impl MachineState {
    pub fn new(system: EmulationSystem) -> Self {
        Self {
            system,
            ..Default::default()
        }
    }

    pub fn load_default_font(&mut self) {
        self.load_font(&default_font::DEFAULT_FONT);
        if self.system == EmulationSystem::SuperChip {
            self.load_big_font(&default_font::DEFAULT_BIG_FONT);
        }
    }

    pub fn load_font(&mut self, font: &[u8; 0x50]) {
        self.ram[0x050..0x050 + size_of_val(font)].copy_from_slice(font);
    }

    pub fn load_big_font(&mut self, big_font: &[u8; 0xA0]) {
        self.ram[0x0A0..0x0A0 + size_of_val(big_font)].copy_from_slice(big_font);
    }

    pub fn load_program(&mut self, program: &[u8]) {
        self.ram[0x200..(0x200 + program.len())].copy_from_slice(program);
    }

    pub fn tick_timer(&mut self) {
        if self.delay_timer > 0 {
            self.delay_timer -= 1;
        }
        if self.sound_timer > 0 {
            self.sound_timer -= 1;
        }
    }

    pub fn tick(
        &mut self,
        mut held_keys: impl FnMut() -> u16,
        mut random: impl FnMut() -> u8,
    ) -> Result<bool, Error> {
        let mut disp_updated = false;

        /* FETCH */
        let instruction: u16 = ((self.ram[self.program_counter as usize] as u16) << 8)
            + (self.ram[(self.program_counter + 1) as usize] as u16);
        self.program_counter += 2;

        /* DECODE */
        let x = ((instruction & 0x0F00) >> 8) as usize;
        let y = ((instruction & 0x00F0) >> 4) as usize;
        let n = instruction & 0x000F;
        let nn = (instruction & 0x00FF) as u8;
        let nnn = instruction & 0x0FFF;

        // #[cfg(debug_assertions)]
        // {
        //     // Display machine state
        //     println!("PC: 0x{:04X}", self.program_counter - 2);
        //     println!("Instruction: {:04X}", instruction);
        //     print!("Stack: ");
        //     for address in &self.stack {
        //         print!("{address:03X}, ");
        //     }
        //     println!();
        //     println!("I : 0x{:04X}", self.index_register);
        //     print!("V : ");
        //     for var in self.var_registers {
        //         print!("0x{var:02X}, ");
        //     }
        //     println!();
        //     println!("VX: 0x{:02X}", self.var_registers[x & 0xF]);
        //     println!("VY: 0x{:02X}", self.var_registers[y & 0xF]);
        // }

        match ((instruction & 0xF000) >> 12, nn, n) {
            // 00E0
            (0x0, _, 0x0) if y == 0xE => {
                self.display_buffer = [[false; DISPLAY_HEIGHT]; DISPLAY_WIDTH];
                disp_updated = true;
            }

            // 00EE
            (0x0, _, 0xE) if y == 0xE => self.program_counter = self.stack.pop().unwrap_or(0x200),

            // 1nnn
            (0x1, _, _) => self.program_counter = nnn,

            // 2nnn
            (0x2, _, _) => {
                self.stack
                    .push(self.program_counter)
                    .map_err(|_| Error::StackOverflow)?;
                self.program_counter = nnn;
            }

            // 3xnn
            (0x3, _, _) => {
                if self.var_registers[x & 0xF] == nn {
                    self.program_counter += 2;
                }
            }

            // 4xnn
            (0x4, _, _) => {
                if self.var_registers[x & 0xF] != nn {
                    self.program_counter += 2;
                }
            }

            // 5xy
            (0x5, _, _) => {
                if self.var_registers[x & 0xF] == self.var_registers[y & 0xF] {
                    self.program_counter += 2;
                }
            }

            // 9xy
            (0x9, _, _) => {
                if self.var_registers[x & 0xF] != self.var_registers[y & 0xF] {
                    self.program_counter += 2;
                }
            }

            // 8xy0
            (0x8, _, 0x0) => self.var_registers[x & 0xF] = self.var_registers[y & 0xF],

            // 8xy1
            (0x8, _, 0x1) => {
                self.var_registers[x & 0xF] |= self.var_registers[y & 0xF];
                if self.system == EmulationSystem::Chip8 {
                    self.var_registers[0xF] = 0;
                }
            }

            // 8xy2
            (0x8, _, 0x2) => {
                self.var_registers[x & 0xF] &= self.var_registers[y & 0xF];
                if self.system == EmulationSystem::Chip8 {
                    self.var_registers[0xF] = 0;
                }
            }

            // 8xy3
            (0x8, _, 0x3) => {
                self.var_registers[x & 0xF] ^= self.var_registers[y & 0xF];
                if self.system == EmulationSystem::Chip8 {
                    self.var_registers[0xF] = 0;
                }
            }

            // 8xy4
            (0x8, _, 0x4) => {
                let overflow = if self.var_registers[x & 0xF] as usize
                    + self.var_registers[y & 0xF] as usize
                    > 0xFF
                {
                    1
                } else {
                    0
                };
                self.var_registers[x & 0xF] =
                    u8::wrapping_add(self.var_registers[x & 0xF], self.var_registers[y & 0xF]);
                self.var_registers[0xF] = overflow;
            }

            // 8xy5
            (0x8, _, 0x5) => {
                let borrow = if self.var_registers[x & 0xF] >= self.var_registers[y & 0xF] {
                    1
                } else {
                    0
                };

                self.var_registers[x & 0xF] =
                    u8::wrapping_sub(self.var_registers[x & 0xF], self.var_registers[y & 0xF]);

                self.var_registers[0xF] = borrow;
            }

            // 8xy7
            (0x8, _, 0x7) => {
                let borrow = if self.var_registers[y & 0xF] >= self.var_registers[x & 0xF] {
                    1
                } else {
                    0
                };

                self.var_registers[x & 0xF] =
                    u8::wrapping_sub(self.var_registers[y & 0xF], self.var_registers[x & 0xF]);

                self.var_registers[0xF] = borrow;
            }

            // 8xy6
            (0x8, _, 0x6) => {
                let shifted_out = self.var_registers[y & 0xF] & 0b00000001;
                self.var_registers[x & 0xF] =
                    self.var_registers[if self.system == EmulationSystem::SuperChip {
                        x
                    } else {
                        y
                    } & 0xF]
                        >> 1;
                self.var_registers[0xF] = shifted_out;
            }

            // 8xyE
            (0x8, _, 0xE) => {
                let shifted_out = (self.var_registers[y & 0xF] & 0b10000000) >> 7;
                self.var_registers[x & 0xF] =
                    self.var_registers[if self.system == EmulationSystem::SuperChip {
                        x
                    } else {
                        y
                    } & 0xF]
                        << 1;
                self.var_registers[0xF] = shifted_out;
            }

            // 6xnn
            (0x6, _, _) => self.var_registers[x & 0xF] = nn,

            // 7xnn
            (0x7, _, _) => {
                self.var_registers[x & 0xF] = u8::wrapping_add(self.var_registers[x & 0xF], nn)
            }

            // Annn
            (0xA, _, _) => self.index_register = nnn,

            // Bnnn
            (0xB, _, _) => {
                self.program_counter = nnn
                    + self.var_registers[if self.system == EmulationSystem::SuperChip {
                        x
                    } else {
                        0
                    }] as u16
            }

            // Cxnn
            (0xC, _, _) => {
                self.var_registers[x & 0xF] = random() & nn;
            }

            // Dxyn
            (0xD, _, _) => {
                if self.high_res {
                    let x = (self.var_registers[x] % DISPLAY_WIDTH as u8) as usize;
                    let y = (self.var_registers[y] % DISPLAY_HEIGHT as u8) as usize;

                    let (n, sprite16) = if n == 0 {
                        (16, true)
                    } else {
                        (n as usize, false)
                    };

                    self.var_registers[0xF] = 0;

                    for i in 0..n {
                        if y + i >= DISPLAY_HEIGHT {
                            self.var_registers[0xF] += (n - i) as u8;
                            break;
                        }

                        let address_offset =
                            self.index_register as usize + if sprite16 { i * 2 } else { i };
                        let sprite_row = if sprite16 {
                            ((self.ram[address_offset] as u16) << 8)
                                + (self.ram[address_offset + 1] as u16)
                        } else {
                            self.ram[self.index_register as usize + i] as u16
                        };

                        let mut collision = false;

                        for j in 0..if sprite16 { 16 } else { 8 } {
                            if x + j >= DISPLAY_WIDTH {
                                break;
                            }

                            let pixel = if sprite16 {
                                (sprite_row >> (15 - j)) & 0b1 == 1
                            } else {
                                (sprite_row >> (7 - j)) & 0b1 == 1
                            };

                            if pixel {
                                if self.display_buffer[x + j][y + i] {
                                    collision = true;
                                }

                                self.display_buffer[x + j][y + i] =
                                    !self.display_buffer[x + j][y + i];
                            }
                        }

                        if collision {
                            self.var_registers[0xF] += 1;
                        }
                    }
                } else {
                    let x = (self.var_registers[x & 0xF] % (DISPLAY_WIDTH / 2) as u8) as usize;
                    let y = (self.var_registers[y & 0xF] % (DISPLAY_HEIGHT / 2) as u8) as usize;

                    let n = n as usize;

                    self.var_registers[0xF] = 0;

                    for i in 0..n {
                        if 2 * (y + i) >= DISPLAY_HEIGHT {
                            break;
                        }

                        let sprite_row = self.ram[self.index_register as usize + i];

                        for j in 0..8 {
                            if 2 * (x + j) >= DISPLAY_WIDTH {
                                break;
                            }

                            if (sprite_row >> (7 - j)) & 0b1 == 1 {
                                if self.display_buffer[2 * (x + j)][2 * (y + i)] {
                                    self.var_registers[0xF] = 1;
                                }

                                #[expect(clippy::identity_op)]
                                {
                                    self.display_buffer[2 * (x + j) + 0][2 * (y + i) + 0] =
                                        !self.display_buffer[2 * (x + j) + 0][2 * (y + i) + 0];
                                    self.display_buffer[2 * (x + j) + 0][2 * (y + i) + 1] =
                                        !self.display_buffer[2 * (x + j) + 0][2 * (y + i) + 1];
                                    self.display_buffer[2 * (x + j) + 1][2 * (y + i) + 0] =
                                        !self.display_buffer[2 * (x + j) + 1][2 * (y + i) + 0];
                                    self.display_buffer[2 * (x + j) + 1][2 * (y + i) + 1] =
                                        !self.display_buffer[2 * (x + j) + 1][2 * (y + i) + 1];
                                }
                            }
                        }
                    }
                }

                disp_updated = true;
            }

            // Ex9E
            (0xE, 0x9E, _) => {
                if (held_keys() >> (self.var_registers[x & 0xF] & 0xF)) & 0b1 == 1 {
                    self.program_counter += 2;
                }
            }

            // ExA1
            (0xE, 0xA1, _) => {
                if (held_keys() >> (self.var_registers[x & 0xF] & 0xF)) & 0b1 == 0 {
                    self.program_counter += 2;
                }
            }

            // Fx07
            (0xF, 0x07, _) => self.var_registers[x & 0xF] = self.delay_timer,

            // Fx15
            (0xF, 0x15, _) => self.delay_timer = self.var_registers[x & 0xF],

            // Fx18
            (0xF, 0x18, _) => self.sound_timer = self.var_registers[x & 0xF],

            // Fx1E
            (0xF, 0x1E, _) => self.index_register += self.var_registers[x & 0xF] as u16,

            // Fx0A
            (0xF, 0xA, _) => {
                let current_keystate = held_keys();

                if current_keystate < self.previous_keystate {
                    let key_diff = self.previous_keystate - current_keystate;

                    for i in 0..16 {
                        if (key_diff >> i) & 0b1 == 1 {
                            self.var_registers[x & 0xF] = i;
                            break;
                        }
                    }
                    self.previous_keystate = 0;
                } else {
                    self.previous_keystate = current_keystate;
                    self.program_counter -= 2;
                }
            }

            // Fx29
            (0xF, 0x29, _) => {
                self.index_register = (0x050 + (self.var_registers[x & 0xF] & 0xF) * 5) as u16;
            }

            // Fx33
            #[allow(clippy::identity_op)]
            (0xF, 0x33, _) => {
                self.ram[self.index_register as usize + 2] = (self.var_registers[x & 0xF] / 1) % 10;
                self.ram[self.index_register as usize + 1] =
                    (self.var_registers[x & 0xF] / 10) % 10;
                self.ram[self.index_register as usize + 0] =
                    (self.var_registers[x & 0xF] / 100) % 10;
            }

            // Fx55
            (0xF, 0x55, _) => {
                for (i, var) in self.var_registers[..=(x & 0xF)].iter().enumerate() {
                    self.ram[self.index_register as usize + i] = *var;
                }
                if self.system == EmulationSystem::Chip8 {
                    self.index_register += (x & 0xF) as u16 + 1;
                }
            }

            // Fx65
            (0xF, 0x65, _) => {
                for (i, var) in self.var_registers[..=(x & 0xF)].iter_mut().enumerate() {
                    *var = self.ram[self.index_register as usize + i];
                }
                if self.system == EmulationSystem::Chip8 {
                    self.index_register += (x & 0xF) as u16 + 1;
                }
            }

            _ => {
                if self.system == EmulationSystem::SuperChip {
                    match instruction {
                        0x00FD => return Err(Error::ProgramExited),

                        0x00FE => self.high_res = false,

                        0x00FF => self.high_res = true,

                        _ if instruction & 0xF0FF == 0xF075 => todo!("Store user flags"),

                        _ if instruction & 0xF0FF == 0xF085 => todo!("Read user flags"),

                        0x00FB => {
                            if self.high_res {
                                self.display_buffer.copy_within(0..DISPLAY_WIDTH - 4, 4);
                                self.display_buffer[0..4].fill([false; DISPLAY_HEIGHT]);
                            } else {
                                self.display_buffer.copy_within(0..DISPLAY_WIDTH - 8, 8);
                                self.display_buffer[0..8].fill([false; DISPLAY_HEIGHT]);
                            }
                        }

                        0x00FC => {
                            if self.high_res {
                                self.display_buffer.copy_within(4..DISPLAY_WIDTH, 0);
                                self.display_buffer[DISPLAY_WIDTH - 4..DISPLAY_WIDTH]
                                    .fill([false; DISPLAY_HEIGHT]);
                            } else {
                                self.display_buffer.copy_within(8..DISPLAY_WIDTH, 0);
                                self.display_buffer[DISPLAY_WIDTH - 8..DISPLAY_WIDTH]
                                    .fill([false; DISPLAY_HEIGHT]);
                            }
                        }

                        _ if instruction & 0xFFF0 == 0x00C0 => {
                            let n = if self.high_res {
                                n as usize
                            } else {
                                n as usize * 2
                            };
                            for x in (0..DISPLAY_WIDTH).rev() {
                                self.display_buffer[x].copy_within(0..DISPLAY_HEIGHT - n, n);
                                self.display_buffer[x][0..n].fill(false);
                            }
                        }

                        _ if instruction & 0xF0FF == 0xF030 => {
                            self.index_register =
                                0x0A0 + (self.var_registers[x & 0xF] & 0xF) as u16 * 10;
                        }

                        _ => return Err(Error::IllegalInstruction(instruction)),
                    }
                } else {
                    return Err(Error::IllegalInstruction(instruction));
                }
            }
        }

        Ok(disp_updated)
    }
}

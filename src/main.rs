#![feature(duration_constants)]
#![feature(random)]
#![allow(arithmetic_overflow)]

mod core;
mod default_font;
pub use default_font::DEFAULT_FONT;

use core::MachineState;
use std::{
    env::args,
    fs::read,
    time::{Duration, Instant},
};

const EMULATION_FREQ: u32 = 500;

fn main() -> Result<(), &'static str> {
    let mut machine_state = MachineState::new();
    machine_state.load_default_font();

    let Some(rom_file) = args().nth(1) else {
        return Err("No ROM file specified!");
    };
    let Ok(program) = read(rom_file) else {
        return Err("File could not be opened");
    };

    machine_state.load_program(&program);

    let mut previous_timer_tick = Instant::now();
    let mut previous_emul_tick = Instant::now();

    loop {
        if previous_timer_tick.elapsed() > Duration::SECOND / 60 {
            previous_timer_tick = Instant::now();
            machine_state.tick_timer();
        }

        if previous_emul_tick.elapsed() > Duration::SECOND / EMULATION_FREQ {
            previous_emul_tick = Instant::now();

            print!("\x1b[2J\x1b[H");
            machine_state.tick(|| 0);

            println!();
            for y in 0..32 {
                for x in 0..64 {
                    print!(
                        "|{}",
                        if machine_state.display_buffer[x][y] {
                            '█'
                        } else {
                            ' '
                        }
                    )
                }
                println!("|");
            }
        }
    }
}

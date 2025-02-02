use rand::Rng;
use rs_chip8_core::{DISPLAY_HEIGHT, DISPLAY_WIDTH, EmulationSystem, MachineState};
use sdl3::{event::Event, keyboard::Scancode, pixels::Color, rect::Point};
use std::{
    ffi::OsStr,
    path::PathBuf,
    process::ExitCode,
    time::{Duration, Instant},
};

const OFF_COLOUR: Color = Color::RGB(0x8f, 0x91, 0x85);
const ON_COLOUR: Color = Color::RGB(0x11, 0x1d, 0x2b);

const INSTR_PER_FRAME: u32 = 10;

const KEYMAP: [Scancode; 16] = [
    Scancode::X,
    Scancode::_1,
    Scancode::_2,
    Scancode::_3,
    Scancode::Q,
    Scancode::W,
    Scancode::E,
    Scancode::A,
    Scancode::S,
    Scancode::D,
    Scancode::Z,
    Scancode::C,
    Scancode::_4,
    Scancode::R,
    Scancode::F,
    Scancode::V,
];

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
enum Error {
    Core(#[from] rs_chip8_core::Error),
    #[error("One argument required")]
    Argument,
    IO(#[from] std::io::Error),
}

fn main() -> ExitCode {
    match actual_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn actual_main() -> Result<(), Error> {
    // Initialise SDL
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    // let audio_subsystem = sdl_context.audio().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let window = video_subsystem
        .window("rs_chip8", 1280, 640)
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    // Set logical resolution to the DISPLAY_WIDTH x DISPLAY_HEIGHT
    let mut canvas = window.into_canvas();
    canvas
        .set_logical_size(
            DISPLAY_WIDTH as u32,
            DISPLAY_HEIGHT as u32,
            sdl3::sys::render::SDL_RendererLogicalPresentation::LETTERBOX,
        )
        .unwrap();

    // Clear the screen
    canvas.set_draw_color(OFF_COLOUR);
    canvas.clear();
    canvas.present();

    // Open and read the program
    let rom_filepath = PathBuf::from(std::env::args().nth(1).ok_or(Error::Argument)?);
    let program = std::fs::read(&rom_filepath)?;

    // Initialise the machine state
    // Choose the system to emulate based on the ROM file extension
    let mut machine_state =
        MachineState::new(match rom_filepath.extension().and_then(OsStr::to_str) {
            Some("ch8") => EmulationSystem::Chip8,
            Some("sc8") => EmulationSystem::SuperChip,
            _ => EmulationSystem::default(),
        });
    machine_state.load_default_font();
    machine_state.load_program(&program);

    // Time period of 60 Hz
    let time_period = Duration::from_secs(1) / 60;
    let mut prev_tick = Instant::now();

    let mut held_keys: u16 = 0;
    let mut rng = rand::rng();
    let mut update_window = false;

    loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(()),
                Event::Window { .. } => update_window = true,
                Event::KeyDown {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(i) = KEYMAP.iter().position(|key| key == &scancode) {
                        held_keys |= 0b1 << i;
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(i) = KEYMAP.iter().position(|key| key == &scancode) {
                        held_keys -= 0b1 << i;
                    }
                }
                _ => (),
            }
        }

        let delta = prev_tick.elapsed();
        if delta < time_period {
            continue;
        } else {
            prev_tick += time_period;
        }

        machine_state.tick_timer();

        if machine_state.sound_timer > 0 {
            // TODO: make sound
        } else {
            // TODO: stop the sound
        }

        let mut disp_updated = false;
        for _ in 0..=INSTR_PER_FRAME {
            disp_updated |= machine_state.tick(|| held_keys, || rng.random())?;
        }

        if disp_updated || update_window {
            canvas.set_draw_color(OFF_COLOUR);
            canvas.clear();

            canvas.set_draw_color(ON_COLOUR);
            for y in 0..DISPLAY_HEIGHT {
                for x in 0..DISPLAY_WIDTH {
                    if machine_state.display_buffer[x][y] {
                        canvas.draw_point(Point::new(x as i32, y as i32)).unwrap();
                    }
                }
            }
            canvas.present();

            update_window = false;
        }
    }
}

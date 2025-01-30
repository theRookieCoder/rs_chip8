use rand::Rng;
use rs_chip8_core::MachineState;
use sdl3::{event::Event, keyboard::Scancode, pixels::Color, rect::Point};
use std::time::{Duration, Instant};

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

fn main() -> Result<(), Error> {
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

    // Set logical resolution to 64x32
    let mut canvas = window.into_canvas();
    canvas
        .set_logical_size(
            64,
            32,
            sdl3::sys::render::SDL_RendererLogicalPresentation::LETTERBOX,
        )
        .unwrap();

    // Clear the screen
    canvas.set_draw_color(OFF_COLOUR);
    canvas.clear();
    canvas.present();

    // Initialise the machine state and load the default font
    let mut machine_state = MachineState::new();
    machine_state.load_default_font();

    let rom_file = std::env::args().nth(1).ok_or(Error::Argument)?;
    let program = std::fs::read(rom_file)?;

    machine_state.load_program(&program);

    // Time period in nanoseconds for 60 Hz
    let time_period = Duration::from_secs(1) / 60;
    let mut prev_tick = Instant::now();

    let mut held_keys: u16 = 0;
    let mut rng = rand::rng();
    let mut window_update = false;

    loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(()),
                Event::Window { .. } => window_update = true,
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
            print!("\x1b[2J\x1b[H");
            println!("Held keys: {held_keys:016b}");
            println!("           FEDCBA9876543210\n");

            disp_updated |= machine_state.tick(|| held_keys, || rng.random())?;

            // Render to terminal
            println!();
            for y in 0..32 {
                for x in 0..64 {
                    print!(
                        "{}",
                        if machine_state.display_buffer[x][y] {
                            "██"
                        } else {
                            "  "
                        }
                    )
                }
                println!();
            }
        }

        if disp_updated || window_update {
            canvas.set_draw_color(OFF_COLOUR);
            canvas.clear();

            canvas.set_draw_color(ON_COLOUR);
            for y in 0..32 {
                for x in 0..64 {
                    if machine_state.display_buffer[x][y] {
                        canvas.draw_point(Point::new(x as i32, y as i32)).unwrap();
                    }
                }
            }
            canvas.present();

            window_update = false;
        }
    }
}

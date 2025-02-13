use parking_lot::Mutex;
use rand::Rng;
use rs_chip8_core::{DISPLAY_HEIGHT, DISPLAY_WIDTH, EmulationSystem, MachineState};
use sdl3::{
    event::{Event, WindowEvent},
    keyboard::Scancode,
    pixels::Color,
    rect::Point,
};
use std::{
    ffi::OsStr,
    path::PathBuf,
    process::ExitCode,
    thread::sleep,
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
    Sdl(#[from] sdl3::Error),
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
    let machine_state = Mutex::new(machine_state);

    // Initialise SDL
    let sdl_context = sdl3::init()?;
    let video_subsystem = sdl_context.video()?;
    let _audio_subsystem = sdl_context.audio()?;
    let event_subsystem = sdl_context.event()?;
    let mut event_pump = sdl_context.event_pump()?;

    let window = match video_subsystem
        .window("rs_chip8", 1280, 640)
        .position_centered()
        .resizable()
        .build()
    {
        Ok(window) => window,
        Err(err) => match err {
            sdl3::video::WindowBuildError::SdlError(err) => return Err(err.into()),
            _ => panic!(
                "Expected window dimensions and title to be valid, but {}",
                err
            ),
        },
    };

    // Set logical resolution to the DISPLAY_WIDTH x DISPLAY_HEIGHT
    let canvas = Mutex::new(window.into_canvas());
    if let Err(err) = canvas.lock().set_logical_size(
        DISPLAY_WIDTH as u32,
        DISPLAY_HEIGHT as u32,
        sdl3::sys::render::SDL_RendererLogicalPresentation::LETTERBOX,
    ) {
        if let sdl3::IntegerOrSdlError::SdlError(err) = err {
            return Err(err.into());
        } else {
            panic!("Expected display height and width to be valid");
        }
    }

    // Time period of 60 Hz
    let time_period = Duration::from_secs(1) / 60;
    let prev_tick = Mutex::new(Instant::now());

    let held_keys = Mutex::new(0_u16);
    let rng = Mutex::new(rand::rng());

    struct ExecutionErrorEvent(Error);
    event_subsystem.register_custom_event::<ExecutionErrorEvent>()?;

    let execution_loop = || -> Result<(), Error> {
        let mut machine_state = machine_state.lock();
        let held_keys = held_keys.lock();
        let mut rng = rng.lock();

        machine_state.tick_timer();

        if machine_state.sound_timer > 0 {
            // TODO: make sound
        } else {
            // TODO: stop the sound
        }

        for _ in 0..=INSTR_PER_FRAME {
            machine_state.tick(|| *held_keys, || rng.random())?;
        }

        let mut canvas = canvas.lock();

        canvas.set_draw_color(OFF_COLOUR);
        canvas.clear();

        canvas.set_draw_color(ON_COLOUR);
        for y in 0..DISPLAY_HEIGHT {
            for x in 0..DISPLAY_WIDTH {
                if machine_state.display_buffer[x][y] {
                    canvas.draw_point(Point::new(x as i32, y as i32))?;
                }
            }
        }

        canvas.present();

        Ok(())
    };

    let _window_update_eventwatch = event_subsystem.add_event_watch(|event| {
        if let Event::Window {
            win_event: WindowEvent::Exposed,
            ..
        } = event
        {
            let delta = prev_tick.lock().elapsed();
            if delta > time_period {
                *prev_tick.lock() += time_period;
                if let Err(err) = execution_loop() {
                    event_subsystem
                        .push_custom_event(ExecutionErrorEvent(err))
                        .expect("Custom event was not registered");
                }
            }
        }
    });

    loop {
        let delta = prev_tick.lock().elapsed();
        if delta < time_period {
            sleep(time_period - delta);
            continue;
        }
        *prev_tick.lock() += time_period;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(()),
                Event::KeyDown {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(i) = KEYMAP.iter().position(|key| key == &scancode) {
                        *held_keys.lock() |= 0b1 << i;
                    }
                }
                Event::KeyUp {
                    scancode: Some(scancode),
                    ..
                } => {
                    if let Some(i) = KEYMAP.iter().position(|key| key == &scancode) {
                        *held_keys.lock() -= 0b1 << i;
                    }
                }
                _ => {
                    if let Some(event) = event.as_user_event_type::<ExecutionErrorEvent>() {
                        return Err(event.0);
                    }
                }
            }
        }

        execution_loop()?;
    }
}

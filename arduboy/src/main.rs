#![no_std]
#![no_main]

use embedded_graphics::{
    Drawable as _,
    mono_font::{MonoTextStyleBuilder, iso_8859_13::FONT_5X7},
    pixelcolor::BinaryColor,
    prelude::Point,
    text::Text,
};
use panic_halt as _;
use ssd1306::{
    I2CDisplayInterface, Ssd1306, mode::DisplayConfig, rotation::DisplayRotation,
    size::DisplaySize128x64,
};

#[arduino_hal::entry]
fn main() -> ! {
    let peripherals = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(peripherals);
    let mut serial = arduino_hal::default_serial!(peripherals, pins, 57600);
    ufmt::uwriteln!(&mut serial, "Hello from Rust over serial!").unwrap();

    let mut rgd_led = [
        pins.d10.into_output_high().downgrade(),
        pins.d9.into_output_high().downgrade(),
        pins.d11.into_output_high().downgrade(),
    ];

    // Up, Down, Left, Right
    let d_pad = [
        pins.a0.into_pull_up_input().downgrade(),
        pins.a3.into_pull_up_input().downgrade(),
        pins.a2.into_pull_up_input().downgrade(),
        pins.a1.into_pull_up_input().downgrade(),
    ];
    let a_button = pins.d7.into_pull_up_input();
    let b_button = pins.d8.into_pull_up_input();

    let mut i2c = arduino_hal::I2c::new(
        peripherals.TWI,
        pins.d2.into_pull_up_input(),
        pins.d3.into_pull_up_input(),
        50000,
    );

    ufmt::uwriteln!(&mut serial, "Write direction test:\r").unwrap();
    i2c.i2cdetect(&mut serial, arduino_hal::i2c::Direction::Write)
        .unwrap();
    ufmt::uwriteln!(&mut serial, "\r\nRead direction test:\r").unwrap();
    i2c.i2cdetect(&mut serial, arduino_hal::i2c::Direction::Read)
        .unwrap();

    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_5X7)
        .text_color(BinaryColor::On)
        .build();

    Text::new("Hello world!", Point::zero(), text_style)
        .draw(&mut display)
        .unwrap();

    display.flush().unwrap();

    loop {}
}

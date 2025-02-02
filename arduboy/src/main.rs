// #![no_std]
// #![no_main]

// use panic_halt as _;

// #[arduino_hal::entry]
// fn main() -> ! {
//     let peripherals = arduino_hal::Peripherals::take().unwrap();
//     let pins = arduino_hal::pins!(peripherals);
//     let mut serial = arduino_hal::default_serial!(peripherals, pins, 57600);

//     let mut rgd_led = [
//         pins.d10.into_output_high().downgrade(),
//         pins.d9.into_output_high().downgrade(),
//         pins.d11.into_output_high().downgrade(),
//     ];

//     // Up, Down, Left, Right
//     let d_pad = [
//         pins.a0.into_pull_up_input().downgrade(),
//         pins.a3.into_pull_up_input().downgrade(),
//         pins.a2.into_pull_up_input().downgrade(),
//         pins.a1.into_pull_up_input().downgrade(),
//     ];
//     let a_button = pins.d7.into_pull_up_input();
//     let b_button = pins.d8.into_pull_up_input();

//     let mut angle = 0.;
//     loop {
//         let mut red = 0.;
//         let mut green = 0.;
//         let mut blue = 0.;

//         if angle < 60. {
//             red = 255.;
//             green = angle * 4.25 - 0.01;
//             blue = 0.;
//         } else if angle < 120. {
//             red = (120. - angle) * 4.25 - 0.01;
//             green = 255.;
//             blue = 0.;
//         } else if angle < 180. {
//             red = 0.;
//             green = 255.;
//             blue = (angle - 120.) * 4.25 - 0.01;
//         } else if angle < 240. {
//             red = 0.;
//             green = (240. - angle) * 4.25 - 0.01;
//             blue = 255.;
//         } else if angle < 300. {
//             red = (angle - 240.) * 4.25 - 0.01;
//             green = 0.;
//             blue = 255.;
//         } else {
//             red = 255.;
//             green = 0.;
//             blue = (360. - angle) * 4.25 - 0.01;
//         }

//         angle = (angle + 1.) % 360.;
//     }
// }

fn main() {}

mod bus;
mod cartridge;
mod cpu;
mod ppu;
use std::path::Path;

use crate::cpu::Cpu;
use bus::Bus;
use cartridge::Cartridge;
use minifb::Key;
use minifb::Window;
use minifb::WindowOptions;

const SCREEN_HEIGHT: usize = 144;
const SCREEN_WIDTH: usize = 160;

fn main() {
    let args: Option<String> = std::env::args().nth(1);
    let sys_path = match args {
        Some(path) => path,

        None => {
            eprintln!("Couldnt find syspath, use: Gameboy <game.gb>");
            std::process::exit(1);
        }
    };
    let rom = match std::fs::read(&sys_path) {
        Ok(rom) => rom,

        Err(e) => {
            eprintln!("No such file, Error: {}", e);
            std::process::exit(1);
        }
    };
    let save = std::fs::read(Path::new(&sys_path).with_extension("sav")).unwrap_or_default();
    let cart = Cartridge::new(rom, &save);
    let mut bus = Bus::new(cart);
    let mut cpu = Cpu::new();
    let mut window = Window::new(
        "RustBoy",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        WindowOptions {
            resize: true,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .expect("couldnt open window");
    window.set_target_fps(60);
    let mut cpu_counter: usize = 0;
    while window.is_open() && !window.is_key_down(Key::RightShift) {
        let mut direction: u8 = 0b0000_1111;
        let mut action: u8 = 0b0000_1111;
        if window.is_key_down(Key::D) {
            direction ^= 1
        }
        if window.is_key_down(Key::A) {
            direction ^= 2
        }
        if window.is_key_down(Key::W) {
            direction ^= 4
        }
        if window.is_key_down(Key::S) {
            direction ^= 8
        }
        if window.is_key_down(Key::J) {
            action ^= 1
        }
        if window.is_key_down(Key::K) {
            action ^= 2
        }
        if window.is_key_down(Key::LeftShift) {
            action ^= 4
        }
        if window.is_key_down(Key::Escape) {
            action ^= 8
        }
        bus.set_p1_buttons(direction, action);
        let cycles = cpu.step(&mut bus);
        bus.tick(cycles);
        cpu_counter += 1;
        if cpu_counter >= 70224 {
            window.update();
            cpu_counter = 0;
        }
        let vblank = bus.tick_ppu(cycles);
        if vblank {
            let _ = window.update_with_buffer(bus.read_buffer(), SCREEN_WIDTH, SCREEN_HEIGHT);
            cpu_counter = 0;
        }
    }
    if bus.has_battery() {
        if bus.has_time() {
            let times = bus.get_time();
            let times_arr: [u8; 10] = times.into();
            let times_vec: Vec<u8> = times_arr
                .into_iter()
                .flat_map(|time| (time as u32).to_le_bytes())
                .collect();
            let _ = std::fs::write(
                Path::new(&sys_path).with_extension("sav"),
                [bus.get_ram_bytes(), &times_vec[..]].concat(),
            );
        } else {
            let _ = std::fs::write(
                Path::new(&sys_path).with_extension("sav"),
                bus.get_ram_bytes(),
            );
        }
    }
}

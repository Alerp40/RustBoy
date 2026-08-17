use rustboy::{
    bus::Bus,
    cpu::Cpu,
    ppu::{SCREEN_HEIGHT, SCREEN_WIDTH},
    cartridge::Cartridge,
};
use std::path::Path;
use cpal::traits::DeviceTrait;
use cpal::traits::HostTrait;
use cpal::traits::StreamTrait;
use minifb::Key;
use minifb::Window;
use minifb::WindowOptions;
use ringbuf::SharedRb;
use ringbuf::storage::Heap;
use ringbuf::{traits::*, HeapRb};


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

    let heap: SharedRb<Heap<f32>> = HeapRb::new(4096);
    let ( mut producer, mut consumer) = heap.split();
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No default output device found");
    let supported_config = device.default_output_config().expect("No default stream config for device");
    let sample_rate = supported_config.sample_rate();
    let channels = supported_config.channels();
    let stream_config = supported_config.into();
    let stream = device.build_output_stream(stream_config, 
        move |data: &mut [f32], _:&cpal::OutputCallbackInfo | {
            for frame in data.chunks_mut(channels as usize){
                frame.iter_mut().for_each(|channel| *channel = consumer.try_pop().unwrap_or(0.0));
            }
        },
        move |err| {
            eprint!("an error ocurred on stream, {}", err);
        },
        None).expect("error building stream");
    let _stream = stream.play();

    let save = std::fs::read(Path::new(&sys_path).with_extension("sav")).unwrap_or_default();
    let cart = Cartridge::new(rom, &save);
    let mut bus = Bus::new(cart,sample_rate as f32);
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
        let audio_options = bus.tick(cycles);
        if let Some((audio_left,audio_right)) = audio_options{
            while producer.vacant_len() < 2{
                std::thread::yield_now();
            }
            let _ = producer.try_push(audio_left);
            let _ = producer.try_push(audio_right);
        }
        let vblank = bus.tick_ppu(cycles);
        if vblank {
            let _ = window.update_with_buffer(bus.read_buffer(), SCREEN_WIDTH, SCREEN_HEIGHT);
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

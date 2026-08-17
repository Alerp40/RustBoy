use rustboy::{bus::Bus, cartridge::Cartridge, cpu::Cpu};

fn run_test(rom_bytes: &[u8], max_cycles: u64){
    let cart = Cartridge::new(rom_bytes.to_vec(), &[0]);
    let mut bus = Bus::new(cart, 44100.0);
    let mut cpu = Cpu::new();

    let mut total_cycles: u64 = 0;

    while total_cycles < max_cycles {
        let cycles = cpu.step(&mut bus);
        total_cycles += cycles as u64;
        bus.tick(cycles);
        bus.tick_ppu(cycles);
        if bus.serial_output.contains("Passed"){
            return;
        }
        if bus.serial_output.contains("Failed"){
            panic!("Test failed: {} ", bus.serial_output)
        }
    }

    panic!("Max cycles reached, output: {}", bus.serial_output)
}

macro_rules! blargg_test {
    ($name: ident, $path: expr) => {
        #[test]
        fn $name() {
            let rom = include_bytes!($path);
            run_test(rom, 1_000_000_000);
        }
    };
}


blargg_test!(test_01_special, "./roms/01-special.gb");
blargg_test!(test_02_interrupts, "./roms/02-interrupts.gb");
blargg_test!(test_03_op_sphl, "./roms/03-op sp,hl.gb");
blargg_test!(test_04_op_rimm, "./roms/04-op r,imm.gb");
blargg_test!(test_05_op_rp, "./roms/05-op rp.gb");
blargg_test!(test_06_ld_rr, "./roms/06-ld r,r.gb");
blargg_test!(test_07_jrjpcallretrst, "./roms/07-jr,jp,call,ret,rst.gb");
blargg_test!(test_08_misc_instrs, "./roms/08-misc instrs.gb");
blargg_test!(test_09_op_rr, "./roms/09-op r,r.gb");
blargg_test!(test_10_bit_ops, "./roms/10-bit ops.gb");
blargg_test!(test_11_op_ahl, "./roms/11-op a,(hl).gb");

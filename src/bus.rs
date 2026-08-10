use crate::apu::Apu;
use crate::cartridge::Cartridge;
use crate::ppu::Ppu;
const WRAM_SIZE: usize = 8192;
pub struct Bus {
    ppu: Ppu,
    cartridge: Cartridge,
    wram: [u8; WRAM_SIZE],
    hram: [u8; 127],
    serial_data: u8,
    serial_control: u8,
    ie: u8,
    if_: u8,
    div: u8,
    tima: u8,
    tma: u8,
    tac: u8,
    cycle_acc: u16,
    cycle_acc_2: u16,
    action: u8,
    direction: u8,
    mode: u8,
}

impl Bus {
    pub fn new(cart: Cartridge) -> Self {
        let ppu = Ppu::new();
        Bus {
            ppu,
            cartridge: cart,
            wram: [0; 8192],
            hram: [0; 127],
            serial_data: 0,
            serial_control: 0,
            ie: 0,
            if_: 0,
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            cycle_acc: 0,
            cycle_acc_2: 0,
            action: 0,
            direction: 0,
            mode: 0,
        }
    }

    pub fn get_time(&self) -> (u8, u8, u8, u8, u8, u8, u8, u8, u8, u8) {
        self.cartridge.get_time()
    }

    pub fn has_time(&self) -> bool {
        self.cartridge.has_time()
    }

    pub fn set_p1_buttons(&mut self, direction: u8, action: u8) {
        if (self.direction & !direction) != 0 {
            self.if_ |= 0b0001_0000
        }
        if (self.action & !action) != 0 {
            self.if_ |= 0b0001_0000
        }
        self.direction = direction;
        self.action = action;
    }

    pub fn get_ram_bytes(&self) -> &[u8] {
        self.cartridge.get_ram_bytes()
    }

    pub fn has_battery(&self) -> bool {
        self.cartridge.get_battery()
    }

    pub fn tick_ppu(&mut self, cycles: u8) -> bool {
        let result = self.ppu.tick(cycles);
        self.if_ |= result;
        result & 1 == 1
    }
    pub fn read_buffer(&self) -> &[u32] {
        self.ppu.read_buffer()
    }

    fn build_p1_byte(&self) -> u8 {
        let mut p1: u8 = 0b1100_0000;
        p1 |= self.mode;
        if (self.mode & 0b0011_0000) == 0 {
            p1 |= self.direction & self.action;
            p1
        } else if (self.mode & 0b0010_0000) == 0 {
            p1 |= self.action;
            p1
        } else if (self.mode & 0b0001_0000) == 0 {
            p1 |= self.direction;
            p1
        } else {
            p1 | 0b0000_1111
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFFFF => self.ie,

            0xFF00 => self.build_p1_byte(),

            0xFF0F => self.if_ | 0b1110_0000,

            0xFF04 => self.div,

            0xFF05 => self.tima,

            0xFF06 => self.tma,

            0xFF07 => self.tac,

            0xFF26 => 0xF0,

            0xFF10..=0xFF3F => 0x00,

            0xA000..=0xBFFF => self.cartridge.read(addr),

            0xFF40..=0xFF4B => self.ppu.read(addr),

            0xFE00..=0xFE9F => self.ppu.read(addr),

            0x8000..=0x9FFF => self.ppu.read(addr),

            0x0000..=0x7FFF => self.cartridge.read(addr),

            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize],

            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize],

            0xFF01 => self.serial_data,

            0xFF02 => self.serial_control,

            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, byte: u8) {
        match addr {
            0xFFFF => self.ie = byte,

            0xFF00 => self.mode = byte & 0b0011_0000,

            0xFF0F => self.if_ = byte,

            0xFF04 => {
                self.div = 0;
                self.cycle_acc = 0;
            }

            0xFF05 => self.tima = byte,

            0xFF06 => self.tma = byte,

            0xFF07 => self.tac = byte,

            0xFF46 => {
                let source_base = (byte as u16) << 8;
                for i in 0_u16..160 {
                    let buffer = self.read(source_base + i);
                    self.ppu.write(0xFE00 + i, buffer);
                }
            }


            0xA000..=0xBFFF => self.cartridge.write_decoder(addr, byte),

            0xFF40..=0xFF4B => self.ppu.write(addr, byte),

            0xFE00..=0xFE9F => self.ppu.write(addr, byte),

            0x8000..=0x9FFF => self.ppu.write(addr, byte),

            0x0000..=0x7FFF => self.cartridge.write_decoder(addr, byte),

            0xC000..=0xDFFF => self.wram[(addr - 0xC000) as usize] = byte,

            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = byte,

            0xFF01 => self.serial_data = byte,

            0xFF02 => {
                if (byte & 0b1000_0000) != 0 {
                    print!("{}", self.serial_data as char);
                }
                self.serial_control = byte & 0b0111_1111;
            }

            _ => (),
        }
    }

    pub fn tick(&mut self, cycles: u8) {
        self.cycle_acc += cycles as u16;
        self.cartridge.tick_rtc(cycles);
        if self.cycle_acc >= 256 {
            self.div = self.div.wrapping_add(1);
            self.cycle_acc -= 256
        }
        if (self.tac & 0b100) != 0 {
            self.cycle_acc_2 += cycles as u16;
            let increment: u16 = match self.tac & 0b0011 {
                0 => 1024,
                1 => 16,
                2 => 64,
                3 => 256,
                _ => 0xFF,
            };
            while self.cycle_acc_2 >= increment {
                self.tima =  self.tima.wrapping_add(1);
                self.cycle_acc_2 -= increment;
                if self.tima == 0 {
                    self.tima =  self.tma;
                    self.if_ |= 0b100;
                }
            }
        }
    }
}

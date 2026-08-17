use crate::bus::Bus;

const FLAG_Z: u8 = 0b1000_0000;
const FLAG_N: u8 = 0b0100_0000;
const FLAG_H: u8 = 0b0010_0000;
const FLAG_C: u8 = 0b0001_0000;
const CYCLES: [u8; 256] = [
    //   x0  x1  x2  x3  x4  x5  x6  x7  x8  x9  xA  xB  xC  xD  xE  xF
    4, 12, 8, 8, 4, 4, 8, 4, 20, 8, 8, 8, 4, 4, 8, 4, // 0x
    4, 12, 8, 8, 4, 4, 8, 4, 12, 8, 8, 8, 4, 4, 8, 4, // 1x
    8, 12, 8, 8, 4, 4, 8, 4, 8, 8, 8, 8, 4, 4, 8, 4, // 2x
    8, 12, 8, 8, 12, 12, 12, 4, 8, 8, 8, 8, 4, 4, 8, 4, // 3x
    4, 4, 4, 4, 4, 4, 8, 4, 4, 4, 4, 4, 4, 4, 8, 4, // 4x
    4, 4, 4, 4, 4, 4, 8, 4, 4, 4, 4, 4, 4, 4, 8, 4, // 5x
    4, 4, 4, 4, 4, 4, 8, 4, 4, 4, 4, 4, 4, 4, 8, 4, // 6x
    8, 8, 8, 8, 8, 8, 4, 8, 4, 4, 4, 4, 4, 4, 8, 4, // 7x
    4, 4, 4, 4, 4, 4, 8, 4, 4, 4, 4, 4, 4, 4, 8, 4, // 8x
    4, 4, 4, 4, 4, 4, 8, 4, 4, 4, 4, 4, 4, 4, 8, 4, // 9x
    4, 4, 4, 4, 4, 4, 8, 4, 4, 4, 4, 4, 4, 4, 8, 4, // Ax
    4, 4, 4, 4, 4, 4, 8, 4, 4, 4, 4, 4, 4, 4, 8, 4, // Bx
    8, 12, 12, 16, 12, 16, 8, 16, 8, 16, 12, 4, 12, 24, 8, 16, // Cx
    8, 12, 12, 4, 12, 16, 8, 16, 8, 16, 12, 4, 12, 4, 8, 16, // Dx
    12, 12, 8, 4, 4, 16, 8, 16, 16, 4, 16, 4, 4, 4, 8, 16, // Ex
    12, 12, 8, 4, 4, 16, 8, 16, 12, 8, 16, 4, 4, 4, 8, 16, // Fx
];

pub struct Cpu {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: u8,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
    ime: bool,
    halted: bool,
    ime_pending: bool,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            f: 0xB0,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
            ime: false,
            halted: false,
            ime_pending: false,
        }
    }
    pub fn af(&self) -> u16 {
        (self.a as u16) << 8 | self.f as u16
    }
    pub fn bc(&self) -> u16 {
        (self.b as u16) << 8 | self.c as u16
    }
    pub fn de(&self) -> u16 {
        (self.d as u16) << 8 | self.e as u16
    }
    pub fn hl(&self) -> u16 {
        (self.h as u16) << 8 | self.l as u16
    }

    pub fn set_af(&mut self, val: u16) {
        self.a = (val >> 8) as u8;
        self.f = (val & 0b1111_0000) as u8;
    }
    pub fn set_bc(&mut self, val: u16) {
        self.b = (val >> 8) as u8;
        self.c = (val & 0b1111_1111) as u8;
    }
    pub fn set_de(&mut self, val: u16) {
        self.d = (val >> 8) as u8;
        self.e = (val & 0b1111_1111) as u8;
    }
    pub fn set_hl(&mut self, val: u16) {
        self.h = (val >> 8) as u8;
        self.l = (val & 0b1111_1111) as u8;
    }

    pub fn flag_z(&self) -> bool {
        self.f & FLAG_Z != 0
    }
    pub fn flag_n(&self) -> bool {
        self.f & FLAG_N != 0
    }
    pub fn flag_h(&self) -> bool {
        self.f & FLAG_H != 0
    }
    pub fn flag_c(&self) -> bool {
        self.f & FLAG_C != 0
    }

    pub fn set_flag_z(&mut self, on: bool) {
        self.f &= !FLAG_Z;
        if on {
            self.f |= FLAG_Z
        }
    }
    pub fn set_flag_n(&mut self, on: bool) {
        self.f &= !FLAG_N;
        if on {
            self.f |= FLAG_N
        }
    }
    pub fn set_flag_h(&mut self, on: bool) {
        self.f &= !FLAG_H;
        if on {
            self.f |= FLAG_H
        }
    }
    pub fn set_flag_c(&mut self, on: bool) {
        self.f &= !FLAG_C;
        if on {
            self.f |= FLAG_C
        }
    }

    fn fetch_byte(&mut self, bus: &Bus) -> u8 {
        let byte = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        byte
    }

    fn fetch_word(&mut self, bus: &Bus) -> u16 {
        let lo = self.fetch_byte(bus);
        let hi = self.fetch_byte(bus) as u16;
        (hi << 8) | lo as u16
    }

    fn reg_mut(&mut self, code: u8) -> &mut u8 {
        match code {
            0 => &mut self.b,
            1 => &mut self.c,
            2 => &mut self.d,
            3 => &mut self.e,
            4 => &mut self.h,
            5 => &mut self.l,
            7 => &mut self.a,
            _ => &mut self.a,
        }
    }
    fn reg(&self, code: u8) -> u8 {
        match code {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            7 => self.a,
            _ => self.a,
        }
    }
    fn full_read(&self, code: u8, bus: &Bus) -> u8 {
        if code == 6 {
            bus.read(self.hl())
        } else {
            self.reg(code)
        }
    }
    fn full_write(&mut self, code: u8, value: u8, bus: &mut Bus) {
        if code == 6 {
            bus.write(self.hl(), value);
        } else {
            *self.reg_mut(code) = value;
        }
    }

    fn execute_cb(&mut self, cb: u8, bus: &mut Bus, cycles: &mut u8) {
        match cb {
            0x00..=0x3F => {
                let operation = (cb >> 3) & 0b111;
                match operation {
                    0 => {
                        if (cb & 0b111) == 6 {
                            let mut mem_byte = bus.read(self.hl());
                            mem_byte = mem_byte.rotate_left(1);
                            bus.write(self.hl(), mem_byte);
                            self.set_flag_c(mem_byte & 1 != 0);
                            self.set_flag_z(mem_byte == 0);
                        } else {
                            *self.reg_mut(cb & 0b111) = self.reg(cb & 0b111).rotate_left(1);
                            self.set_flag_c(self.reg(cb & 0b111) & 1 == 1);
                            self.set_flag_z(self.reg(cb & 0b111) == 0);
                        }
                        self.set_flag_n(false);
                        self.set_flag_h(false);
                    }
                    1 => {
                        if (cb & 0b111) == 6 {
                            let mut mem_byte = bus.read(self.hl());
                            mem_byte = mem_byte.rotate_right(1);
                            bus.write(self.hl(), mem_byte);
                            self.set_flag_c(mem_byte >> 7 == 1);
                            self.set_flag_z(mem_byte == 0);
                        } else {
                            *self.reg_mut(cb & 0b111) = self.reg(cb & 0b111).rotate_right(1);
                            self.set_flag_c((self.reg(cb & 0b111) >> 7) != 0);
                            self.set_flag_z(self.reg(cb & 0b111) == 0);
                        }
                        self.set_flag_n(false);
                        self.set_flag_h(false);
                    }
                    2 => {
                        let old_c = if self.flag_c() { 1 } else { 0 } as u8;
                        if (cb & 0b111) == 6 {
                            let mut mem_byte = bus.read(self.hl());
                            self.set_flag_c((mem_byte >> 7) == 1);
                            mem_byte = (mem_byte << 1) | old_c;
                            bus.write(self.hl(), mem_byte);
                            self.set_flag_z(mem_byte == 0);
                        } else {
                            self.set_flag_c((self.reg(cb & 0b111) >> 7) != 0);
                            *self.reg_mut(cb & 0b111) = (self.reg(cb & 0b111) << 1) | old_c;
                            self.set_flag_z(self.reg(cb & 0b111) == 0);
                        }
                        self.set_flag_n(false);
                        self.set_flag_h(false);
                    }
                    3 => {
                        let old_c = if self.flag_c() { 1 } else { 0 } as u8;
                        if (cb & 0b111) == 6 {
                            let mut mem_byte = bus.read(self.hl());
                            self.set_flag_c((mem_byte & 0b01) != 0);
                            mem_byte = (mem_byte >> 1) | (old_c << 7);
                            bus.write(self.hl(), mem_byte);
                            self.set_flag_z(mem_byte == 0);
                        } else {
                            self.set_flag_c((self.reg(cb & 0b111) & 0b01) != 0);
                            *self.reg_mut(cb & 0b111) = (self.reg(cb & 0b111) >> 1) | (old_c << 7);
                            self.set_flag_z(self.reg(cb & 0b111) == 0);
                        }
                        self.set_flag_n(false);
                        self.set_flag_h(false);
                    }

                    4 => {
                        let code = cb & 0b111;
                        let old_bit = self.full_read(code, bus) >> 7;
                        self.full_write(code, self.full_read(code, bus) << 1, bus);
                        self.set_flag_c(old_bit == 1);
                        self.set_flag_z(self.full_read(code, bus) == 0);
                        self.set_flag_n(false);
                        self.set_flag_h(false);
                    }
                    5 => {
                        let code = cb & 0b111;
                        let old_bit = self.full_read(code, bus) & 0b01;
                        self.full_write(
                            code,
                            (self.full_read(code, bus) >> 1) | (self.full_read(code, bus)) & 0b1000_0000,
                            bus,
                        );
                        self.set_flag_c(old_bit == 1);
                        self.set_flag_z(self.full_read(code, bus) == 0);
                        self.set_flag_n(false);
                        self.set_flag_h(false);
                    }
                    7 => {
                        let code = cb & 0b111;
                        let old_bit = self.full_read(code, bus) & 1;
                        self.full_write(code, self.full_read(code, bus) >> 1, bus);
                        self.set_flag_c(old_bit == 1);
                        self.set_flag_z(self.full_read(code, bus) == 0);
                        self.set_flag_n(false);
                        self.set_flag_h(false);
                    }
                    6 => {
                        let code = cb & 0b111;
                        self.full_write(
                            code,
                            (self.full_read(code, bus) << 4) | (self.full_read(code, bus) >> 4),
                            bus,
                        );
                        self.set_flag_c(false);
                        self.set_flag_h(false);
                        self.set_flag_z(self.full_read(code, bus) == 0);
                        self.set_flag_n(false);
                    }
                    _ => (),
                }
            }

            0x40..=0x7F => {
                let register = self.full_read(cb & 0b111, bus);
                let bit = (cb >> 3) & 0b111;
                match bit {
                    0 => self.set_flag_z(!(register & 0b0000_0001 != 0)),
                    1 => self.set_flag_z(!(register & 0b0000_0010 != 0)),
                    2 => self.set_flag_z(!(register & 0b0000_0100 != 0)),
                    3 => self.set_flag_z(!(register & 0b0000_1000 != 0)),
                    4 => self.set_flag_z(!(register & 0b0001_0000 != 0)),
                    5 => self.set_flag_z(!(register & 0b0010_0000 != 0)),
                    6 => self.set_flag_z(!(register & 0b0100_0000 != 0)),
                    7 => self.set_flag_z(!(register & 0b1000_0000 != 0)),
                    _ => println!("non existend bit: {:04x}", bit),
                }
                self.set_flag_n(false);
                self.set_flag_h(true);
            }
            0x80..=0xBF => {
                let code = cb & 0b111;
                let bit = (cb >> 3) & 0b111;
                match bit {
                    0 => self.full_write(code, self.full_read(code, bus) & 0b1111_1110, bus),
                    1 => self.full_write(code, self.full_read(code, bus) & 0b1111_1101, bus),
                    2 => self.full_write(code, self.full_read(code, bus) & 0b1111_1011, bus),
                    3 => self.full_write(code, self.full_read(code, bus) & 0b1111_0111, bus),
                    4 => self.full_write(code, self.full_read(code, bus) & 0b1110_1111, bus),
                    5 => self.full_write(code, self.full_read(code, bus) & 0b1101_1111, bus),
                    6 => self.full_write(code, self.full_read(code, bus) & 0b1011_1111, bus),
                    7 => self.full_write(code, self.full_read(code, bus) & 0b0111_1111, bus),
                    _ => println!("non existend bit: {:04x}", bit),
                }
            }
            0xC0..=0xFF => {
                let code = cb & 0b111;
                let bit = (cb >> 3) & 0b111;
                match bit {
                    0 => self.full_write(code, self.full_read(code, bus) | 0b0000_0001, bus),
                    1 => self.full_write(code, self.full_read(code, bus) | 0b0000_0010, bus),
                    2 => self.full_write(code, self.full_read(code, bus) | 0b0000_0100, bus),
                    3 => self.full_write(code, self.full_read(code, bus) | 0b0000_1000, bus),
                    4 => self.full_write(code, self.full_read(code, bus) | 0b0001_0000, bus),
                    5 => self.full_write(code, self.full_read(code, bus) | 0b0010_0000, bus),
                    6 => self.full_write(code, self.full_read(code, bus) | 0b0100_0000, bus),
                    7 => self.full_write(code, self.full_read(code, bus) | 0b1000_0000, bus),
                    _ => println!("non existend bit: {:04x}", bit),
                }
            }
        }
        if cb & 0b111 != 6 {
            *cycles = 8
        } else if (0x40_u8..=0x7F).contains(&cb) {
            *cycles = 12
        } else {
            *cycles = 16
        }
    }

    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        let enabling = self.ime_pending;
        let mut cycles: u8 = 0;
        let pending = (bus.read(0xFFFF) & bus.read(0xFF0F)) & 0b0001_1111;
        if pending != 0 {
            self.halted = false
        }
        if self.halted {
            cycles += 4;
        }
        if self.ime && (pending != 0) {
            let lowest_set = pending & pending.wrapping_neg();
            let lowest_set_index = pending.trailing_zeros();
            self.ime = false;
            self.sp = self.sp.wrapping_sub(1);
            bus.write(self.sp, (self.pc >> 8) as u8);
            self.sp = self.sp.wrapping_sub(1);
            bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
            let if_byte = bus.read(0xFF0F);
            bus.write(0xFF0F, if_byte & !lowest_set);
            self.pc = 0x40 + (lowest_set_index as u16) * 8;
            cycles += 20
        } else if !self.halted {
            let byte = self.fetch_byte(bus);
            cycles += CYCLES[byte as usize];
            match byte {
                0x00 => (),

                0x02 => bus.write(self.bc(), self.a),
                0x0A => self.a = bus.read(self.bc()),
                0x12 => bus.write(self.de(), self.a),
                0x1A => self.a = bus.read(self.de()),
                0x22 => {
                    bus.write(self.hl(), self.a);
                    self.set_hl(self.hl().wrapping_add(1));
                }
                0x2A => {
                    self.a = bus.read(self.hl());
                    self.set_hl(self.hl().wrapping_add(1));
                }
                0x32 => {
                    bus.write(self.hl(), self.a);
                    self.set_hl(self.hl().wrapping_sub(1));
                }
                0x3A => {
                    self.a = bus.read(self.hl());
                    self.set_hl(self.hl().wrapping_sub(1));
                }
                0x36 => bus.write(self.hl(), self.fetch_byte(bus)),
                0xE0 => bus.write(0xFF00 + (self.fetch_byte(bus) as u16), self.a),
                0xF0 => self.a = bus.read(0xFF00 + (self.fetch_byte(bus) as u16)),
                0xE2 => bus.write(0xFF00 + (self.c as u16), self.a),
                0xF2 => self.a = bus.read(0xFF00 + (self.c as u16)),
                0xEA => bus.write(self.fetch_word(bus), self.a),
                0xFA => self.a = bus.read(self.fetch_word(bus)),
                0xF9 => self.sp = self.hl(),
                0x08 => {
                    let word = self.fetch_word(bus);
                    let lo = (self.sp & 0b1111_1111) as u8;
                    let hi = (self.sp >> 8) as u8;
                    bus.write(word, lo);
                    bus.write(word.wrapping_add(1), hi);
                }

                0xF3 => self.ime = false,
                0xFB => self.ime_pending = true,
                0x10 => (),

                0x3E => {
                    let nbyte = self.fetch_byte(bus);
                    self.a = nbyte;
                }
                0x06 => {
                    let nbyte = self.fetch_byte(bus);
                    self.b = nbyte;
                }
                0x0E => {
                    let nbyte = self.fetch_byte(bus);
                    self.c = nbyte;
                }
                0x16 => {
                    let nbyte = self.fetch_byte(bus);
                    self.d = nbyte;
                }
                0x1E => {
                    let nbyte = self.fetch_byte(bus);
                    self.e = nbyte;
                }
                0x26 => {
                    let nbyte = self.fetch_byte(bus);
                    self.h = nbyte;
                }
                0x2E => {
                    let nbyte = self.fetch_byte(bus);
                    self.l = nbyte;
                }

                0xCB => {
                    let operand = self.fetch_byte(bus);
                    self.execute_cb(operand, bus, &mut cycles);
                }

                0x3C => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_add(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0x0F);
                    self.set_flag_n(false);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_add(1);
                }
                0x04 => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_add(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0x0F);
                    self.set_flag_n(false);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_add(1);
                }
                0x0C => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_add(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0x0F);
                    self.set_flag_n(false);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_add(1);
                }
                0x14 => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_add(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0x0F);
                    self.set_flag_n(false);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_add(1);
                }
                0x1C => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_add(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0x0F);
                    self.set_flag_n(false);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_add(1);
                }
                0x24 => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_add(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0x0F);
                    self.set_flag_n(false);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_add(1);
                }
                0x2C => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_add(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0x0F);
                    self.set_flag_n(false);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_add(1);
                }
                0x34 => {
                    let opcode = bus.read(self.hl());
                    self.set_flag_z(opcode.wrapping_add(1) == 0);
                    self.set_flag_h((opcode & 0b0000_1111) == 0x0F);
                    self.set_flag_n(false);
                    bus.write(self.hl(), opcode.wrapping_add(1));
                }

                0x3D => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_sub(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0);
                    self.set_flag_n(true);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_sub(1);
                }
                0x05 => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_sub(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0);
                    self.set_flag_n(true);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_sub(1);
                }
                0x0D => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_sub(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0);
                    self.set_flag_n(true);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_sub(1);
                }
                0x15 => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_sub(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0);
                    self.set_flag_n(true);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_sub(1);
                }
                0x1D => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_sub(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0);
                    self.set_flag_n(true);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_sub(1);
                }
                0x25 => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_sub(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0);
                    self.set_flag_n(true);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_sub(1);
                }
                0x2D => {
                    self.set_flag_z(self.reg((byte >> 3) & 0b111).wrapping_sub(1) == 0);
                    self.set_flag_h((self.reg((byte >> 3) & 0b111) & 0b0000_1111) == 0);
                    self.set_flag_n(true);
                    *self.reg_mut((byte >> 3) & 0b111) =
                        self.reg((byte >> 3) & 0b111).wrapping_sub(1);
                }
                0x35 => {
                    let opcode = bus.read(self.hl());
                    self.set_flag_z(opcode.wrapping_sub(1) == 0);
                    self.set_flag_h((opcode & 0b0000_1111) == 0);
                    self.set_flag_n(true);
                    bus.write(self.hl(), opcode.wrapping_sub(1));
                }

                0x03 => {
                    let pair = self.bc();
                    self.set_bc(pair.wrapping_add(1));
                }
                0x13 => {
                    let pair = self.de();
                    self.set_de(pair.wrapping_add(1));
                }
                0x23 => {
                    let pair = self.hl();
                    self.set_hl(pair.wrapping_add(1));
                }
                0x33 => self.sp = self.sp.wrapping_add(1),

                0x0B => {
                    let pair = self.bc();
                    self.set_bc(pair.wrapping_sub(1));
                }
                0x1B => {
                    let pair = self.de();
                    self.set_de(pair.wrapping_sub(1));
                }
                0x2B => {
                    let pair = self.hl();
                    self.set_hl(pair.wrapping_sub(1));
                }
                0x3B => self.sp = self.sp.wrapping_sub(1),

                0x09 => {
                    let src = self.bc();
                    self.set_flag_n(false);
                    if (self.hl() & 0b0000_1111_1111_1111).wrapping_add(src & 0b0000_1111_1111_1111) > 0x0FFF {
                        self.set_flag_h(true)
                    } else {
                        self.set_flag_h(false)
                    }
                    if (self.hl() as u32).wrapping_add(src as u32) > 0xFFFF {
                        self.set_flag_c(true)
                    } else {
                        self.set_flag_c(false)
                    }
                    self.set_hl(self.hl().wrapping_add(src));
                }
                0x19 => {
                    let src = self.de();
                    self.set_flag_n(false);
                    if (self.hl() & 0b0000_1111_1111_1111).wrapping_add(src & 0b0000_1111_1111_1111) > 0x0FFF {
                        self.set_flag_h(true)
                    } else {
                        self.set_flag_h(false)
                    }
                    if (self.hl() as u32).wrapping_add(src as u32) > 0xFFFF {
                        self.set_flag_c(true)
                    } else {
                        self.set_flag_c(false)
                    }
                    self.set_hl(self.hl().wrapping_add(src));
                }
                0x29 => {
                    let src = self.hl();
                    self.set_flag_n(false);
                    if (self.hl() & 0b0000_1111_1111_1111).wrapping_add(src & 0b0000_1111_1111_1111) > 0x0FFF {
                        self.set_flag_h(true)
                    } else {
                        self.set_flag_h(false)
                    }
                    if (self.hl() as u32).wrapping_add(src as u32) > 0xFFFF {
                        self.set_flag_c(true)
                    } else {
                        self.set_flag_c(false)
                    }
                    self.set_hl(self.hl().wrapping_add(src));
                }
                0x39 => {
                    let src = self.sp;
                    self.set_flag_n(false);
                    if (self.hl() & 0b0000_1111_1111_1111).wrapping_add(src & 0b0000_1111_1111_1111) > 0x0FFF {
                        self.set_flag_h(true)
                    } else {
                        self.set_flag_h(false)
                    }
                    if (self.hl() as u32).wrapping_add(src as u32) > 0xFFFF {
                        self.set_flag_c(true)
                    } else {
                        self.set_flag_c(false)
                    }
                    self.set_hl(self.hl().wrapping_add(src));
                }

                0xE8 => {
                    let opcode = (self.fetch_byte(bus) as i8) as i16;
                    self.set_flag_h((self.sp & 0b0000_1111).wrapping_add(opcode as u16 & 0b0000_1111) > 0x0F);
                    self.set_flag_c((self.sp & 0b1111_1111).wrapping_add(opcode as u16 & 0b1111_1111) > 0xFF);
                    self.set_flag_z(false);
                    self.set_flag_n(false);
                    self.sp = self.sp.wrapping_add(opcode as u16)
                }
                0xF8 => {
                    let opcode = (self.fetch_byte(bus) as i8) as i16;
                    self.set_flag_h((self.sp & 0b0000_1111).wrapping_add(opcode as u16 & 0b0000_1111) > 0x0F);
                    self.set_flag_c((self.sp & 0b1111_1111).wrapping_add(opcode as u16 & 0b1111_1111) > 0xFF);
                    self.set_flag_z(false);
                    self.set_flag_n(false);
                    self.set_hl(self.sp.wrapping_add(opcode as u16));
                }

                0x07 => {
                    self.set_flag_z(false);
                    self.set_flag_n(false);
                    self.set_flag_h(false);
                    self.a = self.a.rotate_left(1);
                    self.set_flag_c((self.a & 0b0000_0001) != 0);
                }
                0x0F => {
                    self.set_flag_z(false);
                    self.set_flag_n(false);
                    self.set_flag_h(false);
                    self.a = self.a.rotate_right(1);
                    self.set_flag_c((self.a & 0b1000_0000) != 0);
                }
                0x17 => {
                    self.set_flag_z(false);
                    self.set_flag_n(false);
                    self.set_flag_h(false);
                    let old_carry = if self.flag_c() { 1 } else { 0 };
                    self.set_flag_c(self.a >> 7 == 1);
                    self.a = (self.a << 1) | old_carry;
                }
                0x1F => {
                    self.set_flag_z(false);
                    self.set_flag_n(false);
                    self.set_flag_h(false);
                    let old_carry = if self.flag_c() { 1 } else { 0 };
                    self.set_flag_c(self.a & 0b01 != 0);
                    self.a = (self.a >> 1) | (old_carry << 7);
                }

                0x2F => {
                    self.set_flag_n(true);
                    self.set_flag_h(true);
                    self.a ^= 0xFF
                }
                0x37 => {
                    self.set_flag_c(true);
                    self.set_flag_n(false);
                    self.set_flag_h(false);
                }
                0x3F => {
                    self.set_flag_n(false);
                    self.set_flag_h(false);
                    self.set_flag_c(!self.flag_c());
                }

                0x27 => {
                    let mut offset = 0;
                    let mut carry = self.flag_c();
                    if self.flag_h() || (!self.flag_n() && (self.a & 0b0000_1111) > 0x09) {
                        offset |= 0b0000_0110;
                    }
                    if self.flag_c() || (!self.flag_n() && self.a > 0x99) {
                        offset |= 0b0110_0000;
                        carry = true
                    }
                    self.a = if self.flag_n() {
                        self.a.wrapping_sub(offset)
                    } else {
                        self.a.wrapping_add(offset)
                    };
                    self.set_flag_z(self.a == 0);
                    self.set_flag_h(false);
                    self.set_flag_c(carry);
                }

                0xC3 => self.pc = self.fetch_word(bus),
                0xC2 => {
                    let word = self.fetch_word(bus);
                    if !self.flag_z() {
                        self.pc = word;
                        cycles += 4
                    }
                }
                0xCA => {
                    let word = self.fetch_word(bus);
                    if self.flag_z() {
                        self.pc = word;
                        cycles += 4
                    }
                }
                0xD2 => {
                    let word = self.fetch_word(bus);
                    if !self.flag_c() {
                        self.pc = word;
                        cycles += 4
                    }
                }
                0xDA => {
                    let word = self.fetch_word(bus);
                    if self.flag_c() {
                        self.pc = word;
                        cycles += 4
                    }
                }
                0xE9 => self.pc = self.hl(),

                0xCD => {
                    let target = self.fetch_word(bus);
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc >> 8) as u8);
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                    self.pc = target
                }
                0xC4 => {
                    let target = self.fetch_word(bus);
                    if !self.flag_z() {
                        self.sp = self.sp.wrapping_sub(1);
                        bus.write(self.sp, (self.pc >> 8) as u8);
                        self.sp = self.sp.wrapping_sub(1);
                        bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                        self.pc = target;
                        cycles += 12
                    }
                }
                0xCC => {
                    let target = self.fetch_word(bus);
                    if self.flag_z() {
                        self.sp = self.sp.wrapping_sub(1);
                        bus.write(self.sp, (self.pc >> 8) as u8);
                        self.sp = self.sp.wrapping_sub(1);
                        bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                        self.pc = target;
                        cycles += 12
                    }
                }
                0xD4 => {
                    let target = self.fetch_word(bus);
                    if !self.flag_c() {
                        self.sp = self.sp.wrapping_sub(1);
                        bus.write(self.sp, (self.pc >> 8) as u8);
                        self.sp = self.sp.wrapping_sub(1);
                        bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                        self.pc = target;
                        cycles += 12
                    }
                }
                0xDC => {
                    let target = self.fetch_word(bus);
                    if self.flag_c() {
                        self.sp = self.sp.wrapping_sub(1);
                        bus.write(self.sp, (self.pc >> 8) as u8);
                        self.sp = self.sp.wrapping_sub(1);
                        bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                        self.pc = target;
                        cycles += 12
                    }
                }

                0xC9 => {
                    let lo = bus.read(self.sp) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    let hi = bus.read(self.sp) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    self.pc = hi << 8 | lo;
                }
                0xD9 => {
                    let lo = bus.read(self.sp) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    let hi = bus.read(self.sp) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    self.pc = hi << 8 | lo;
                    self.ime = true
                }
                0xC0 => {
                    if !self.flag_z() {
                        let lo = bus.read(self.sp) as u16;
                        self.sp = self.sp.wrapping_add(1);
                        let hi = bus.read(self.sp) as u16;
                        self.sp = self.sp.wrapping_add(1);
                        self.pc = hi << 8 | lo;
                        cycles += 12
                    }
                }
                0xC8 => {
                    if self.flag_z() {
                        let lo = bus.read(self.sp) as u16;
                        self.sp = self.sp.wrapping_add(1);
                        let hi = bus.read(self.sp) as u16;
                        self.sp = self.sp.wrapping_add(1);
                        self.pc = hi << 8 | lo;
                        cycles += 12
                    }
                }
                0xD0 => {
                    if !self.flag_c() {
                        let lo = bus.read(self.sp) as u16;
                        self.sp = self.sp.wrapping_add(1);
                        let hi = bus.read(self.sp) as u16;
                        self.sp = self.sp.wrapping_add(1);
                        self.pc = hi << 8 | lo;
                        cycles += 12
                    }
                }
                0xD8 => {
                    if self.flag_c() {
                        let lo = bus.read(self.sp) as u16;
                        self.sp = self.sp.wrapping_add(1);
                        let hi = bus.read(self.sp) as u16;
                        self.sp = self.sp.wrapping_add(1);
                        self.pc = hi << 8 | lo;
                        cycles += 12
                    }
                }

                0xC7 => {
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc >> 8) as u8);
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                    self.pc = 0x00
                }
                0xCF => {
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc >> 8) as u8);
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                    self.pc = 0x08
                }
                0xD7 => {
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc >> 8) as u8);
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                    self.pc = 0x10
                }
                0xDF => {
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc >> 8) as u8);
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                    self.pc = 0x18
                }
                0xE7 => {
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc >> 8) as u8);
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                    self.pc = 0x20
                }
                0xEF => {
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc >> 8) as u8);
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                    self.pc = 0x28
                }
                0xF7 => {
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc >> 8) as u8);
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                    self.pc = 0x30
                }
                0xFF => {
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc >> 8) as u8);
                    self.sp = self.sp.wrapping_sub(1);
                    bus.write(self.sp, (self.pc & 0b1111_1111) as u8);
                    self.pc = 0x38
                }

                0x01 => {
                    let word = self.fetch_word(bus);
                    self.set_bc(word);
                }
                0x11 => {
                    let word = self.fetch_word(bus);
                    self.set_de(word);
                }
                0x21 => {
                    let word = self.fetch_word(bus);
                    self.set_hl(word);
                }
                0x31 => self.sp = self.fetch_word(bus),

                0x76 => self.halted = true,

                0xC5 => {
                    self.sp = self.sp.wrapping_sub(1);
                    let hi = (self.bc() >> 8) as u8;
                    bus.write(self.sp, hi);
                    self.sp = self.sp.wrapping_sub(1);
                    let lo = (self.bc() & 0b1111_1111) as u8;
                    bus.write(self.sp, lo);
                }
                0xD5 => {
                    self.sp = self.sp.wrapping_sub(1);
                    let hi = (self.de() >> 8) as u8;
                    bus.write(self.sp, hi);
                    self.sp = self.sp.wrapping_sub(1);
                    let lo = (self.de() & 0b1111_1111) as u8;
                    bus.write(self.sp, lo);
                }
                0xE5 => {
                    self.sp = self.sp.wrapping_sub(1);
                    let hi = (self.hl() >> 8) as u8;
                    bus.write(self.sp, hi);
                    self.sp = self.sp.wrapping_sub(1);
                    let lo = (self.hl() & 0b1111_1111) as u8;
                    bus.write(self.sp, lo);
                }
                0xF5 => {
                    self.sp = self.sp.wrapping_sub(1);
                    let hi = (self.af() >> 8) as u8;
                    bus.write(self.sp, hi);
                    self.sp = self.sp.wrapping_sub(1);
                    let lo = (self.af() & 0b1111_1111) as u8;
                    bus.write(self.sp, lo)
                }

                0x80..=0x87 => {
                    let mut src = byte & 0b111;
                    if src == 6 {
                        src = bus.read(self.hl())
                    } else {
                        src = self.reg(src)
                    }
                    self.set_flag_z(self.a.wrapping_add(src) == 0);
                    self.set_flag_n(false);
                    if (self.a & 0b0000_1111).wrapping_add(src & 0b0000_1111) > 0x0F {
                        self.set_flag_h(true)
                    } else {
                        self.set_flag_h(false)
                    }
                    if (self.a as u16).wrapping_add(src as u16) > 0xFF {
                        self.set_flag_c(true)
                    } else {
                        self.set_flag_c(false)
                    }
                    self.a = self.a.wrapping_add(src)
                }
                0x88..=0x8F => {
                    let c = if self.flag_c() { 1 } else { 0 };
                    let mut src = byte & 0b111;
                    if src == 6 {
                        src = bus.read(self.hl())
                    } else {
                        src = self.reg(src)
                    }
                    self.set_flag_z(self.a.wrapping_add(src).wrapping_add(c) == 0);
                    self.set_flag_n(false);
                    if (self.a & 0b0000_1111).wrapping_add(src & 0b0000_1111).wrapping_add(c) > 0x0F {
                        self.set_flag_h(true)
                    } else {
                        self.set_flag_h(false)
                    }
                    if (self.a as u16)
                        .wrapping_add(src as u16)
                        .wrapping_add(c as u16)
                        > 0xFF
                    {
                        self.set_flag_c(true)
                    } else {
                        self.set_flag_c(false)
                    }
                    self.a = self.a.wrapping_add(src).wrapping_add(c)
                }

                0xCE => {
                    let c = if self.flag_c() { 1 } else { 0 };
                    let src = self.fetch_byte(bus);
                    self.set_flag_z(self.a.wrapping_add(src).wrapping_add(c) == 0);
                    self.set_flag_n(false);
                    if (self.a & 0b0000_1111).wrapping_add(src & 0b0000_1111).wrapping_add(c) > 0x0F {
                        self.set_flag_h(true)
                    } else {
                        self.set_flag_h(false)
                    }
                    if (self.a as u16)
                        .wrapping_add(src as u16)
                        .wrapping_add(c as u16)
                        > 0xFF
                    {
                        self.set_flag_c(true)
                    } else {
                        self.set_flag_c(false)
                    }
                    self.a = self.a.wrapping_add(src).wrapping_add(c)
                }

                0xC6 => {
                    let src = self.fetch_byte(bus);
                    self.set_flag_z(self.a.wrapping_add(src) == 0);
                    self.set_flag_n(false);
                    if (self.a & 0b0000_1111).wrapping_add(src & 0b0000_1111) > 0x0F {
                        self.set_flag_h(true)
                    } else {
                        self.set_flag_h(false)
                    }
                    if (self.a as u16).wrapping_add(src as u16) > 0xFF {
                        self.set_flag_c(true)
                    } else {
                        self.set_flag_c(false)
                    }
                    self.a = self.a.wrapping_add(src)
                }

                0x90..=0x97 => {
                    let mut src = byte & 0b111;
                    if src == 6 {
                        src = bus.read(self.hl())
                    } else {
                        src = self.reg(src)
                    }
                    self.set_flag_z(self.a.wrapping_sub(src) == 0);
                    self.set_flag_n(true);
                    self.set_flag_h((self.a & 0b0000_1111) < (src & 0b0000_1111));
                    self.set_flag_c(self.a < src);
                    self.a = self.a.wrapping_sub(src)
                }
                0x98..=0x9F => {
                    let c = if self.flag_c() { 1 } else { 0 };
                    let mut src = byte & 0b111;
                    if src == 6 {
                        src = bus.read(self.hl())
                    } else {
                        src = self.reg(src)
                    }
                    self.set_flag_z(self.a.wrapping_sub(src).wrapping_sub(c) == 0);
                    self.set_flag_n(true);
                    self.set_flag_h((self.a & 0b0000_1111) < (src & 0b0000_1111).wrapping_add(c));
                    self.set_flag_c((self.a as u16) < (src as u16).wrapping_add(c as u16));
                    self.a = self.a.wrapping_sub(src).wrapping_sub(c)
                }

                0xD6 => {
                    let src = self.fetch_byte(bus);
                    self.set_flag_z(self.a.wrapping_sub(src) == 0);
                    self.set_flag_n(true);
                    self.set_flag_h((self.a & 0b0000_1111) < (src & 0b0000_1111));
                    self.set_flag_c(self.a < src);
                    self.a = self.a.wrapping_sub(src)
                }

                0xDE => {
                    let c = if self.flag_c() { 1 } else { 0 };
                    let src = self.fetch_byte(bus);
                    self.set_flag_z(self.a.wrapping_sub(src).wrapping_sub(c) == 0);
                    self.set_flag_n(true);
                    self.set_flag_h((self.a & 0b0000_1111) < (src & 0b0000_1111).wrapping_add(c));
                    self.set_flag_c((self.a as u16) < (src as u16).wrapping_add(c as u16));
                    self.a = self.a.wrapping_sub(src).wrapping_sub(c)
                }

                0xA0..=0xA7 => {
                    let mut src = byte & 0b111;
                    if src == 6 {
                        src = bus.read(self.hl())
                    } else {
                        src = self.reg(src)
                    }
                    self.set_flag_z((self.a & src) == 0);
                    self.set_flag_n(false);
                    self.set_flag_h(true);
                    self.set_flag_c(false);
                    self.a &= src
                }
                0xE6 => {
                    let src = self.fetch_byte(bus);
                    self.set_flag_z((self.a & src) == 0);
                    self.set_flag_n(false);
                    self.set_flag_h(true);
                    self.set_flag_c(false);
                    self.a &= src
                }

                0xA8..=0xAF => {
                    let mut src = byte & 0b111;
                    if src == 6 {
                        src = bus.read(self.hl())
                    } else {
                        src = self.reg(src)
                    }
                    self.set_flag_z((self.a ^ src) == 0);
                    self.set_flag_n(false);
                    self.set_flag_h(false);
                    self.set_flag_c(false);
                    self.a ^= src
                }
                0xEE => {
                    let src = self.fetch_byte(bus);
                    self.set_flag_z((self.a ^ src) == 0);
                    self.set_flag_n(false);
                    self.set_flag_h(false);
                    self.set_flag_c(false);
                    self.a ^= src
                }

                0xB0..=0xB7 => {
                    let mut src = byte & 0b111;
                    if src == 6 {
                        src = bus.read(self.hl())
                    } else {
                        src = self.reg(src)
                    }
                    self.set_flag_z((self.a | src) == 0);
                    self.set_flag_n(false);
                    self.set_flag_h(false);
                    self.set_flag_c(false);
                    self.a |= src
                }
                0xF6 => {
                    let src = self.fetch_byte(bus);
                    self.set_flag_z((self.a | src) == 0);
                    self.set_flag_n(false);
                    self.set_flag_h(false);
                    self.set_flag_c(false);
                    self.a |= src
                }

                0xB8..=0xBF => {
                    let mut src = byte & 0b111;
                    if src == 6 {
                        src = bus.read(self.hl())
                    } else {
                        src = self.reg(src)
                    }
                    self.set_flag_z(self.a == src);
                    self.set_flag_n(true);
                    self.set_flag_h((self.a & 0b0000_1111) < (src & 0x0F));
                    self.set_flag_c(self.a < src);
                }
                0xFE => {
                    let src = self.fetch_byte(bus);
                    self.set_flag_z(self.a == src);
                    self.set_flag_n(true);
                    self.set_flag_h((self.a & 0b0000_1111) < (src & 0x0F));
                    self.set_flag_c(self.a < src);
                }

                0x18 => {
                    let operand = (self.fetch_byte(bus) as i8) as i16;
                    self.pc = self.pc.wrapping_add(operand as u16);
                }
                0x20 => {
                    let operand = (self.fetch_byte(bus) as i8) as i16;
                    if !self.flag_z() {
                        self.pc = self.pc.wrapping_add(operand as u16);
                        cycles += 4
                    }
                }
                0x28 => {
                    let operand = (self.fetch_byte(bus) as i8) as i16;
                    if self.flag_z() {
                        self.pc = self.pc.wrapping_add(operand as u16);
                        cycles += 4
                    }
                }
                0x30 => {
                    let operand = (self.fetch_byte(bus) as i8) as i16;
                    if !self.flag_c() {
                        self.pc = self.pc.wrapping_add(operand as u16);
                        cycles += 4
                    }
                }
                0x38 => {
                    let operand = (self.fetch_byte(bus) as i8) as i16;
                    if self.flag_c() {
                        self.pc = self.pc.wrapping_add(operand as u16);
                        cycles += 4
                    }
                }

                0xC1 => {
                    let lo = (bus.read(self.sp)) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    let hi = (bus.read(self.sp)) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    self.set_bc((hi << 8) | lo);
                }
                0xD1 => {
                    let lo = (bus.read(self.sp)) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    let hi = (bus.read(self.sp)) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    self.set_de((hi << 8) | lo);
                }
                0xE1 => {
                    let lo = (bus.read(self.sp)) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    let hi = (bus.read(self.sp)) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    self.set_hl((hi << 8) | lo);
                }
                0xF1 => {
                    let lo = (bus.read(self.sp)) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    let hi = (bus.read(self.sp)) as u16;
                    self.sp = self.sp.wrapping_add(1);
                    self.set_af((hi << 8) | lo);
                }

                0b0100_0000..=0b0111_1111 => {
                    let dst = (byte >> 3) & 0b111;
                    let src = byte & 0b111;
                    if src == 6 {
                        *self.reg_mut(dst) = bus.read(self.hl());
                    } else if dst == 6 {
                        bus.write(self.hl(), self.reg(src));
                    } else {
                        *self.reg_mut(dst) = self.reg(src);
                    }
                }

                _ => {
                    println!("{:#04x}", byte);
                }
            };
            if enabling {
                self.ime = true;
                self.ime_pending = false;
            }
        }
        cycles
    }
}

pub struct Cartridge {
    rom_bytes: Vec<u8>,
    ram_bytes: Vec<u8>,
    found_type: u8,
    rom_size: u8,
    rom_bank: u8,
    ram_bank: u8,
    ram_enable: bool,
    bank_mode: u8,
    rtc_s: u8,
    rtc_m: u8,
    rtc_h: u8,
    rtc_dl: u8,
    rtc_dh: u8,
    rtc_s_latched: u8,
    rtc_m_latched: u8,
    rtc_h_latched: u8,
    rtc_dl_latched: u8,
    rtc_dh_latched: u8,
    rtc_latch_prev: u8,
    cycles_counter: u32,
}
const CPU_HZ: u32 = 4194304;

impl Cartridge {
    pub fn new(rom: Vec<u8>, save: &[u8]) -> Self {
        let found_battery = rom[0x0147];
        let found_rom_size = rom[0x0148];
        let found_ram_size = rom[0x0149];
        let decoded_size: usize = match found_ram_size {
            0x00 => 0,
            0x01 => 2048,
            0x02 => 8192,
            0x03 => 32768,
            0x04 => 131072,
            0x05 => 65536,
            _ => 0,
        };
        let mut ram = vec![0xFF; decoded_size];
        let has_battery = matches!(found_battery, 0x03 | 0x10 | 0x13 | 0x1B);
        let has_time = matches!(found_battery, 0x10 | 0x0F);
        let mut time_decoded: Vec<u32> = vec![0; 10];
        if has_time && has_battery && (save.len() >= decoded_size + 40) {
            let time: &[u8] = &save[decoded_size..];
            time_decoded = time
                .chunks_exact(4)
                .map(|chunk| {
                    let array: [u8; 4] = chunk.try_into().unwrap();
                    u32::from_le_bytes(array)
                })
                .collect();
        }
        if has_battery && (save.len() >= decoded_size) && !save.is_empty() {
            ram[0..decoded_size].copy_from_slice(&save[0..decoded_size]);
        };
        Cartridge {
            rom_bytes: rom,
            ram_bytes: ram,
            found_type: found_battery,
            rom_size: found_rom_size,
            rom_bank: 1,
            ram_bank: 0,
            ram_enable: false,
            bank_mode: 0,
            rtc_s: time_decoded[0] as u8,
            rtc_m: time_decoded[1] as u8,
            rtc_h: time_decoded[2] as u8,
            rtc_dl: time_decoded[3] as u8,
            rtc_dh: time_decoded[4] as u8,
            rtc_s_latched: time_decoded[5] as u8,
            rtc_m_latched: time_decoded[6] as u8,
            rtc_h_latched: time_decoded[7] as u8,
            rtc_dl_latched: time_decoded[8] as u8,
            rtc_dh_latched: time_decoded[9] as u8,
            rtc_latch_prev: 0,
            cycles_counter: 0,
        }
    }

    pub fn get_battery(&self) -> bool {
        matches!(self.found_type, 0x03 | 0x10 | 0x13 | 0x1B)
    }

    pub fn get_ram_bytes(&self) -> &[u8] {
        &self.ram_bytes
    }

    pub fn get_time(&self) -> (u8, u8, u8, u8, u8, u8, u8, u8, u8, u8) {
        (
            self.rtc_s,
            self.rtc_m,
            self.rtc_h,
            self.rtc_dl,
            self.rtc_dh,
            self.rtc_s_latched,
            self.rtc_m_latched,
            self.rtc_h_latched,
            self.rtc_dl_latched,
            self.rtc_dh_latched,
        )
    }

    pub fn has_time(&self) -> bool {
        (self.found_type == 0x0F) || (self.found_type == 0x10)
    }

    pub fn tick_rtc(&mut self, cycles: u8) {
        if (self.rtc_dh & 0b0100_0000) != 0 {
            return;
        }
        self.cycles_counter += cycles as u32;
        if self.cycles_counter >= CPU_HZ {
            self.cycles_counter -= CPU_HZ;
            self.rtc_s += 1;

            if self.rtc_s >= 60 {
                self.rtc_s = 0;
                self.rtc_m += 1;

                if self.rtc_m >= 60 {
                    self.rtc_m = 0;
                    self.rtc_h += 1;

                    if self.rtc_h >= 24 {
                        let mut days = self.rtc_dl as u16 | ((self.rtc_dh & 0b1) as u16) << 8;
                        self.rtc_h = 0;
                        days += 1;

                        if days > 511 {
                            days = 0;
                            self.rtc_dh |= 0b1000_0000;
                        }
                        self.rtc_dl = days as u8;
                        self.rtc_dh = (self.rtc_dh & 0b1100_0000) | ((days >> 8) & 0b1) as u8;
                    }
                }
            }
        }
    }

    pub fn write_decoder(&mut self, addr: u16, byte: u8) {
        let mbc3 = matches!(self.found_type, 0x0F..=0x13);
        match addr {
            0x2000..=0x3FFF => {
                if !mbc3 {
                    if (byte & 0b0001_1111) == 0 {
                        self.rom_bank = 1
                    } else {
                        self.rom_bank = byte & 0b0001_1111;
                    }
                } else {
                    if (byte & 0b0111_1111) == 0 {
                        self.rom_bank = 1
                    } else {
                        self.rom_bank = byte & 0b0111_1111;
                    }
                }
            }

            0x4000..=0x5FFF => {
                if !mbc3 {
                    self.ram_bank = byte & 0b0000_0011;
                } else {
                    self.ram_bank = byte;
                }
            }
            0xA000..=0xBFFF => {
                if !self.ram_enable {
                    return;
                }
                if (0x08..=0x0C).contains(&self.ram_bank) {
                    return match self.ram_bank {
                        0x08 => self.rtc_s = byte,
                        0x09 => self.rtc_m = byte,
                        0x0A => self.rtc_h = byte,
                        0x0B => self.rtc_dl = byte,
                        0x0C => self.rtc_dh = byte & 0b0000_1100,
                        _ => (),
                    };
                };
                if self.ram_bytes.is_empty() {
                    return;
                }
                let bank: usize = if !mbc3 {
                    if self.bank_mode == 1 {
                        self.ram_bank as usize
                    } else {
                        0
                    }
                } else {
                    self.ram_bank as usize
                };
                let offset: usize = bank * 0x2000 + ((addr as usize) - 0xA000);
                let size = self.ram_bytes.len() - 1;
                self.ram_bytes[offset & size] = byte;
            }

            0x0000..=0x1FFF => self.ram_enable = (byte & 0b0000_1111) == 0x0A,
            0x6000..=0x7FFF if !mbc3 => self.bank_mode = byte & 1,
            0x6000..=0x7FFF => {
                if (self.rtc_latch_prev == 0x00) && (byte == 0x01) {
                    self.rtc_s_latched = self.rtc_s;
                    self.rtc_m_latched = self.rtc_m;
                    self.rtc_h_latched = self.rtc_h;
                    self.rtc_dl_latched = self.rtc_dl;
                    self.rtc_dh_latched = self.rtc_dh;
                }
                self.rtc_latch_prev = byte;
            }

            _ => (),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let mbc3 = matches!(self.found_type, 0x0F..=0x13);
        match addr {
            0x0000..=0x3FFF => self.rom_bytes[addr as usize],

            0xA000..=0xBFFF => {
                if !self.ram_enable {
                    return 0xFF;
                }
                if (0x08..=0x0C).contains(&self.ram_bank) {
                    return match self.ram_bank {
                        0x08 => self.rtc_s_latched,
                        0x09 => self.rtc_m_latched,
                        0x0A => self.rtc_h_latched,
                        0x0B => self.rtc_dl_latched,
                        0x0C => self.rtc_dh_latched,
                        _ => 0,
                    };
                };
                if self.ram_bytes.is_empty() {
                    return 0xFF;
                }
                let bank: usize = if !mbc3 {
                    if self.bank_mode == 1 {
                        self.ram_bank as usize
                    } else {
                        0
                    }
                } else {
                    self.ram_bank as usize
                };
                let offset: usize = bank * 0x2000 + ((addr as usize) - 0xA000);
                self.ram_bytes[offset & (self.ram_bytes.len() - 1)]
            }
            0x4000..=0x7FFF => {
                if !mbc3 {
                    let mut effective_bank: usize =
                        ((self.ram_bank as usize) << 5) | (self.rom_bank as usize);
                    let num_banks: usize = 2 << self.rom_size;
                    effective_bank &= num_banks - 1;
                    let offset = (effective_bank) * 0x4000 + (addr as usize - 0x4000);
                    self.rom_bytes[offset]
                } else {
                    let mut effective_bank: usize = self.rom_bank as usize;
                    let num_banks: usize = 2 << self.rom_size;
                    effective_bank &= num_banks - 1;
                    let offset = (effective_bank) * 0x4000 + (addr as usize - 0x4000);
                    self.rom_bytes[offset]
                }
            }

            _ => 0xFF,
        }
    }
}

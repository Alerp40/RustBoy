pub const SCREEN_HEIGHT: usize = 144;
pub const SCREEN_WIDTH: usize = 160;
const DOTS_PER_LINE: u16 = 456;
const VRAM_SIZE: usize = 8192;
const REG_LCDC: u16 = 0xFF40;
const REG_STAT: u16 = 0xFF41;
const REG_SCY: u16 = 0xFF42;
const REG_SCX: u16 = 0xFF43;
const REG_LY: u16 = 0xFF44;
const REG_LYC: u16 = 0xFF45;
const REG_BGP: u16 = 0xFF47;
const REG_OBP0: u16 = 0xFF48;
const REG_OBP1: u16 = 0xFF49;
const REG_WY: u16 = 0xFF4A;
const REG_WX: u16 = 0xFF4B;
pub struct Ppu {
    buffer: [u32; SCREEN_WIDTH* SCREEN_HEIGHT],
    vram: [u8; 8192],
    oam: [u8; SCREEN_WIDTH],
    lcdc: u8,
    stat: u8,
    scy: u8,
    ly: u8,
    lyc: u8,
    scx: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    dots: u16,
    old_mode: u8,
    old_stat_bool: bool,
    window_line: u8,
}
#[derive(Debug, Clone, Copy)]
struct Sprite {
    y: u8,
    x: u8,
    tile: u8,
    flags: u8,
    oam_index: u8,
}

impl Default for Ppu{
    fn default() -> Self {
        Self {
            buffer: [0x00FFFFFF; 23040],
            vram: [0; VRAM_SIZE],
            oam: [0; SCREEN_WIDTH],
            lcdc: 0x91,
            stat: 0x85,
            scy: 0,
            ly: 0,
            scx: 0,
            lyc: 0,
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0,
            wx: 0,
            dots: 0,
            old_mode: 3,
            old_stat_bool: false,
            window_line: 0,
        }
    }
}

impl Ppu {
    pub fn new() -> Self {
        Self::default()
    }

    fn lcd_enabled(&self) -> bool{
        (self.lcdc & 0b1000_0000) != 0
    }
    fn window_map_area(&self) -> bool{
        (self.lcdc & 0b0100_0000) != 0
    }
    fn window_enabled(&self) -> bool{
        (self.lcdc & 0b0010_0000) != 0
    }
    fn bg_window_data_area(&self) -> bool{
        (self.lcdc & 0b0001_0000) != 0
    }
    fn bg_map_area(&self) -> bool{
        (self.lcdc & 0b0000_1000) != 0
    }
    fn obj_size_16(&self) -> bool{
        (self.lcdc & 0b0000_0100) != 0
    }
    fn obj_enabled(&self) -> bool{
        (self.lcdc & 0b0000_0010) != 0
    }
    fn bg_enabled(&self) -> bool{
        (self.lcdc & 0b0000_0001) != 0
    }

    pub fn calculate_offset(&self, px: u8, py: u8) -> u16 {
        let column = px / 8;
        let row = py / 8;
        let offset: u16 = (row as u16) * 32 + (column as u16);
        offset
    }

    pub fn tile_data_addr(&self, index: u8) -> u16 {
        let map_base: u32 = if self.bg_window_data_area() {
            0x0000
        } else {
            0x1000
        };
        let index_signed = index as i8;
        let is_signed = !self.bg_window_data_area();
        if is_signed {
            ((map_base as i16) + (index_signed as i16) * 16) as u16
        } else {
            (map_base as u16) + (index as u16) * 16
        }
    }

    pub fn decode_pixel(&self, low: u8, high: u8, col: u8) -> u8 {
        let check = 0b1000_0000 >> col;
        let low_bit: u8 = if (check & low) > 0 { 1 } else { 0 };
        let hi_bit: u8 = if (check & high) > 0 { 1 } else { 0 };
        (hi_bit << 1) | low_bit
    }

    fn scan_oam(&self) -> [Option<Sprite>; 10] {
        let mut sprite_array: [Option<Sprite>; 10] = [None;10];
        let mut counter: usize = 0;
        for i in 0_usize..40 {
            let sprite_y = self.oam[i * 4];
            let sprite_x = self.oam[(i * 4) + 1];
            let sprite_tile = self.oam[(i * 4) + 2];
            let sprite_flags = self.oam[(i * 4) + 3];
            let height: u8 = if self.obj_size_16() { 16 } else { 8 };
            let touch_ly = (sprite_y <= (self.ly + 16))
                && (((self.ly as u16) + 16) < ((sprite_y as u16) + (height as u16)));
            if touch_ly {
                sprite_array[counter] = Some(Sprite {
                    x: sprite_x,
                    tile: sprite_tile,
                    flags: sprite_flags,
                    oam_index: i as u8,
                    y: sprite_y,
                });
                counter += 1;
                if counter == 10 {
                    return sprite_array;
                }
            }
        }
        sprite_array
    }

    fn fetch_sprite_row(&self, sprite: Sprite) -> [u8; 8] {
        let mut output = [0; 8];
        let mut row = (self.ly + 16) - sprite.y;
        let height = if self.obj_size_16() { 16 } else { 8 };
        if (sprite.flags & 0b0100_0000) != 0 {
            row = (height - 1) - row
        };
        let tile_number = if height == 16 {
            sprite.tile & 0b1111_1110
        } else {
            sprite.tile
        };
        let base = (tile_number as u16) * 16;
        let low = self.vram[(base + (row as u16) * 2) as usize];
        let high = self.vram[(base + (row as u16) * 2 + 1) as usize];
        let mut col;
        for i in 0_u8..8 {
            if (sprite.flags & 0b0010_0000) != 0 {
                col = 7 - i;
            } else {
                col = i;
            }
            output[i as usize] = self.decode_pixel(low, high, col)
        }
        output
    }

    pub fn render_scanline(&mut self) {
        if self.ly >= SCREEN_HEIGHT as u8 {
            return;
        }
        let mut bg_index_line: [u8; SCREEN_WIDTH] = [0; SCREEN_WIDTH];
        let mut drawn = false;
        if !self.bg_enabled() {
            let rgb = self.shade_to_rgb(0);
            for (i,bg_index) in bg_index_line.iter_mut().enumerate() {
                self.buffer[((self.ly as u16) * SCREEN_WIDTH as u16 + i as u16) as usize] = rgb;
                *bg_index = 0;
            }
        } else {
            let window_active = self.window_enabled() && (self.ly >= self.wy);
            let wx_offset = self.wx.wrapping_sub(7);
            for screen_x in 0_u8..(SCREEN_WIDTH as u8) {
                let calculated_x: u8;
                let calculated_y: u8;
                let map_pick: u16;
                if window_active && (screen_x >= wx_offset) {
                    drawn = true;
                    calculated_x = screen_x - (wx_offset);
                    calculated_y = self.window_line;
                    map_pick = if self.window_map_area() {
                        0x1C00
                    } else {
                        0x1800
                    };
                } else {
                    calculated_y = self.ly.wrapping_add(self.scy);
                    calculated_x = screen_x.wrapping_add(self.scx);
                    map_pick = if self.bg_map_area() {
                        0x1C00
                    } else {
                        0x1800
                    };
                }
                let offset = self.calculate_offset(calculated_x, calculated_y);
                let index = self.vram[(map_pick.wrapping_add(offset)) as usize];
                let tile_base = self.tile_data_addr(index);
                let low = self.vram[(tile_base + 2 * (calculated_y as u16 % 8)) as usize];
                let high = self.vram[(tile_base + 2 * (calculated_y as u16 % 8) + 1) as usize];
                let slot = self.decode_pixel(low, high, calculated_x % 8);
                bg_index_line[screen_x as usize] = slot;
                let shade = (self.bgp >> (slot * 2)) & 0b11;
                let rgb = self.shade_to_rgb(shade);
                self.buffer[((self.ly as u16) * SCREEN_WIDTH as u16 + screen_x as u16) as usize] = rgb;
            }
        }
        if self.obj_enabled() {
            let sprites = self.scan_oam();
            let mut collected_sprites: Vec<Sprite> = sprites.into_iter().flatten().collect();
            collected_sprites.sort_by_key(|item| (item.x, item.oam_index));
            collected_sprites.reverse();
            for sprite in collected_sprites.iter().copied() {
                let row = self.fetch_sprite_row(sprite);
                for col in 0..8 {
                    let screen_x = sprite.x as i16 - 8 + col as i16;
                    if !((0..(SCREEN_WIDTH as i16)).contains(&screen_x)) {
                        continue;
                    }
                    let index = row[col as usize];
                    if index == 0 {
                        continue;
                    }
                    if (sprite.flags & 0b1000_0000) != 0 && bg_index_line[screen_x as usize] != 0 {
                        continue;
                    }
                    let palette = if (sprite.flags & 0b0001_0000) != 0 {
                        self.obp1
                    } else {
                        self.obp0
                    };
                    let shade = (palette >> (index * 2)) & 0b11;
                    let rgb = self.shade_to_rgb(shade);
                    self.buffer[self.ly as usize * SCREEN_WIDTH + screen_x as usize] = rgb;
                }
            }
        }
        if drawn {
            self.window_line += 1
        }
    }

    pub fn shade_to_rgb(&self, shade: u8) -> u32 {
        match shade {
            0 => 0x00FFFFFF,
            1 => 0x00AAAAAA,
            2 => 0x00555555,
            3 => 0x00000000,
            _ => 0x00FFFFFF,
        }
    }

    pub fn read_buffer(&self) -> &[u32] {
        &self.buffer
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            REG_LCDC => self.lcdc,

            REG_STAT => self.stat | 0b1000_0000,

            REG_SCY => self.scy,

            REG_SCX => self.scx,

            REG_LY => self.ly,

            REG_LYC => self.lyc,

            REG_BGP => self.bgp,

            REG_OBP0 => self.obp0,

            REG_OBP1 => self.obp1,

            REG_WY => self.wy,

            REG_WX => self.wx,

            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize],

            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize],

            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, byte: u8) {
        match addr {
            REG_LCDC => {
                if (self.lcd_enabled()) && ((byte & 0b1000_0000) == 0) {
                    self.ly = 0;
                    self.dots = 0;
                    self.stat &= !0b11;
                    self.old_stat_bool = false;
                    self.old_mode = 0;
                    self.window_line = 0;
                }
                self.lcdc = byte;
            }

            REG_STAT => self.stat = (byte & 0b1111_1000) | (self.stat & 0b0000_0111),

            REG_SCY => self.scy = byte,

            REG_SCX => self.scx = byte,

            REG_LY => (),

            REG_LYC => self.lyc = byte,

            REG_BGP => self.bgp = byte,

            REG_OBP0 => self.obp0 = byte,

            REG_OBP1 => self.obp1 = byte,

            REG_WY => self.wy = byte,

            REG_WX => self.wx = byte,

            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = byte,

            0x8000..=0x9FFF => self.vram[(addr - 0x8000) as usize] = byte,

            _ => (),
        }
    }

    pub fn stat_bool(&self, mode: u8) -> bool {
        (((self.stat & 0b0100_0000) == 0b0100_0000) && (self.ly == self.lyc))
            || (((self.stat & 0b0010_0000) == 0b0010_0000) && (mode == 2))
            || (((self.stat & 0b0001_0000) == 0b0001_0000) && (mode == 1))
            || (((self.stat & 0b0000_1000) == 0b0000_1000) && (mode == 0))
    }

    pub fn tick(&mut self, cycles: u8) -> u8 {
        if !self.lcd_enabled() {
            return 0;
        };
        let mut interrupts = 0;
        self.dots += cycles as u16;
        if self.dots >= DOTS_PER_LINE {
            self.dots -= DOTS_PER_LINE;
            self.ly = self.ly.wrapping_add(1);

            if self.ly == SCREEN_HEIGHT as u8 {
                interrupts = 1;
            }
            if self.ly > 153 {
                self.ly = 0;
                self.window_line = 0;
            }
        }
        if self.ly == self.lyc {
            self.stat = (self.stat & 0b1111_1011) | 4;
        } else {
            self.stat &= 0b1111_1011
        }
        let mode = if self.ly >= SCREEN_HEIGHT as u8 {
            1
        } else if self.dots < 80 {
            2
        } else if self.dots < 252 {
            3
        } else {
            0
        };
        self.stat = (self.stat & !0b11) | mode;
        if self.stat_bool(mode) && !self.old_stat_bool {
            interrupts |= 0b10;
        }

        if (self.old_mode != 3) && (mode == 3) {
            self.render_scanline();
        }
        self.old_mode = mode;
        self.old_stat_bool = self.stat_bool(mode);
        interrupts
    }
}

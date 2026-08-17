use crate::{noise, square, wave};

const APU_READ_MASKS: [u8; 23] = [0x80,0x3F,0x00,0xFF,0xBF,0xFF,0x3F,0x00,0xFF,0xBF,0x7F,0xFF,0x9F,0xFF,0xBF,0xFF,0xFF,0x00,0x00,0xBF,0x00,0x00,0x70];

pub struct Apu{
    enabled: bool,
    nr50: u8,
    nr51: u8,
    channel1 : square::SquareChannel,
    channel2 : square::SquareChannel,
    channel3 : wave::WaveChannel,
    channel4 : noise::NoiseChannel,
    sequence_timer : u32,
    frame_step : u8,
    sample_timer : f32,
    cycles_per_sample: f32,
}


impl Apu{
    pub fn new(sample_rate : f32) -> Self{
        let channel2 = square::SquareChannel::new(false);
        let channel1 = square::SquareChannel::new(true);
        let channel3 = wave::WaveChannel::new();
        let channel4 = noise::NoiseChannel::new();
        Apu{
            enabled: true,
            nr50: 0b0111_0111,
            nr51: 0b1111_0011,
            channel1,
            channel2,
            channel3,
            channel4,
            sequence_timer : 0,
            frame_step : 0,
            sample_timer : 0.0,
            cycles_per_sample : 4194304.0/sample_rate,
        }
    }
    fn mixer(&self, s1 : f32, s2: f32, s3: f32, s4: f32) -> Option<(f32,f32)>{
        let left_sum = {( 0.0 +
            if (self.nr51 & 0b0001_0000) != 0 { s1 } else { 0.0 } +
            if (self.nr51 & 0b0010_0000) != 0 { s2 } else { 0.0 } +
            if (self.nr51 & 0b0100_0000) != 0 { s3 } else { 0.0 } +
            if (self.nr51 & 0b1000_0000) != 0 { s4 } else { 0.0 } )
            / 4.0
        };
        let right_sum = {( 0.0 +
            if (self.nr51 & 0b0000_0001) != 0 { s1 } else { 0.0 } +
            if (self.nr51 & 0b0000_0010) != 0 { s2 } else { 0.0 } +
            if (self.nr51 & 0b0000_0100) != 0 { s3 } else { 0.0 } +
            if (self.nr51 & 0b0000_1000) != 0 { s4 } else { 0.0 } )
            / 4.0
        };
        let left_multiplyer = (((self.nr50 >> 4) & 0b0000_0111) as f32 + 1.0)/8.0;
        let right_multiplyer = (((self.nr50) & 0b0000_0111) as f32 + 1.0)/8.0;
        Some((left_sum * left_multiplyer, right_sum * right_multiplyer))   
    }
    
    pub fn tick(&mut self, cycles:u8) -> Option<(f32,f32)>{
        self.sequence_timer += cycles as u32;
        let mut ticked = false;
        if self.sequence_timer >= 8192{
            self.sequence_timer -= 8192;
            ticked = true;
        }
        if !self.enabled{
            return None;
        }
        self.channel2.tick(cycles);
        self.channel3.tick(cycles);
        self.channel1.tick(cycles);
        self.channel4.tick(cycles);
        if ticked{
            self.channel2.tick_frame_sequencer(self.frame_step);
            self.channel1.tick_frame_sequencer(self.frame_step);
            self.channel3.tick_frame_sequencer(self.frame_step);
            self.channel4.tick_frame_sequencer(self.frame_step);
            self.frame_step += 1;
            if self.frame_step == 8 {
                self.frame_step = 0;
            };
        }
        self.sample_timer += cycles as f32;
        if self.sample_timer >= self.cycles_per_sample{
            self.sample_timer -= self.cycles_per_sample;
            self.mixer(self.channel1.sample(), self.channel2.sample(), self.channel3.sample(),self.channel4.sample())
        }
        else{
            None
        }
    }

    pub fn read(&self, addr: u16) -> u8{
        if !self.enabled && (addr != 0xFF26) && !(0xFF30..=0xFF3F).contains(&addr) { return APU_READ_MASKS[(addr - 0xFF10) as usize]}
        let byte = match addr {
            0xFF15 => 0xFF,
            0xFF1F => 0xFF,
            0xFF24 => self.nr50,
            0xFF25 => self.nr51,
            0xFF26 => {
                let mut byte = (self.enabled as u8) << 7;
                byte |= (self.channel4.is_active() as u8) << 3;
                byte |= (self.channel3.is_active() as u8) << 2;
                byte |= (self.channel2.is_active() as u8) << 1;
                byte |= self.channel1.is_active() as u8;
                byte
            }
            0xFF27..=0xFF2F => 0xFF,
            0xFF30..=0xFF3F => self.channel3.read_wave(addr),
            0xFF10..=0xFF14 => self.channel1.read(addr),
            0xFF16..=0xFF19 => self.channel2.read(addr),
            0xFF1A..=0xFF1E => self.channel3.read(addr),
            0xFF20..=0xFF23 => self.channel4.read(addr),
            _ => 0xFF,
        };
        
        if (0xFF10_u16..=0xFF26).contains(&addr){
            byte | APU_READ_MASKS[(addr - 0xFF10) as usize]
        }else{
            byte
        }
    }

    pub fn write(&mut self, addr: u16, byte:u8){
        if (0xFF30_u16..=0xFF3F).contains(&addr){
            self.channel3.parse_input(addr, byte, self.frame_step);
            return;
        }else if addr == 0xFF26{
                if byte & 0b1000_0000 != 0{
                    if !self.enabled {
                        self.frame_step = 0;
                    }
                    self.enabled = true;
                }else{
                    self.enabled = false;
                    self.nr50 = 0;
                    self.nr51 = 0;
                    self.channel4.reset();
                    self.channel3.reset();
                    self.channel2.reset();
                    self.channel1.reset();
                }
            return;
        }
        if self.enabled {
            match addr {
                0xFF24 => self.nr50 = byte,
                0xFF25 => self.nr51 = byte,
                0xFF10..=0xFF14 => self.channel1.parse_input(addr, byte, self.frame_step, self.enabled),
                0xFF16..=0xFF19 => self.channel2.parse_input(addr, byte, self.frame_step, self.enabled),
                0xFF1A..=0xFF1E => self.channel3.parse_input(addr, byte, self.frame_step),
                0xFF20..=0xFF23 => self.channel4.parse_input(addr, byte, self.frame_step),
                _ => ()
            }
        }
        else{
            match addr {
                0xFF11 => self.channel1.parse_input(addr, byte, self.frame_step, self.enabled),
                0xFF16 => self.channel2.parse_input(addr, byte, self.frame_step, self.enabled),
                0xFF1B => self.channel3.parse_input(addr, byte, self.frame_step),
                0xFF20 => self.channel4.parse_input(addr, byte, self.frame_step),
                _ => ()
            }
        }
    }

}

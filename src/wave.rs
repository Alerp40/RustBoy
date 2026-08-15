pub struct WaveChannel{
    channel_enable: bool,
    dac_enable: bool,
    frequency: u16,
    timer: u16,
    length_timer: u16,
    length_enable: bool,
    volume_code: u8,
    wave_ram: [u8;16],
    sample_index: u8,
}

impl WaveChannel{
    pub fn new() -> Self{
        WaveChannel{
            channel_enable: false,
            dac_enable: false,
            frequency: 0,
            timer: 0,
            length_timer: 0,
            length_enable: false,
            volume_code: 0,
            wave_ram: [0;16],
            sample_index: 0,
        }
    }

    fn period(&self) -> u16{
        (2048 - self.frequency) * 2
    }

    pub fn tick(&mut self, cycles: u8){
        if !self.channel_enable{
            return;
        }
        let mut cycles = cycles as u32;
        while cycles > 0{
            if cycles >= self.timer as u32{
                cycles -= self.timer as u32;
                self.timer = self.period();
                self.sample_index = (self.sample_index +1) %32;
            }else{
                self.timer -= cycles as u16;
                cycles = 0;
            }
    }
    }

    pub fn tick_frame_sequencer(&mut self, step : u8){
        if step.is_multiple_of(2) && self.length_enable && (self.length_timer > 0){
            self.length_timer -= 1;
            if self.length_timer == 0{
                self.channel_enable = false;
            }
        }
    }

    pub fn read_wave(&self, addr: u16) -> u8{
        if self.channel_enable{
            if self.timer >= self.period().saturating_sub(2){
                self.wave_ram[(self.sample_index / 2) as usize]
            }else{
                0xFF
            }
        }else{
            self.wave_ram[(addr - 0xFF30) as usize]
        }
    }

    pub fn is_active(&self) -> bool{
        self.channel_enable
    }

    pub fn read(&self, addr : u16) -> u8{
        match addr {
            0xFF1A => (self.dac_enable as u8) << 7,
            0xFF1C => self.volume_code << 5,
            0xFF1E => (self.length_enable as u8) << 6,
            _ => 0x00,
        }
    }

    pub fn sample(&self) -> f32{
        if !self.dac_enable || !self.channel_enable {return 0.0};
        let byte_index = self.sample_index/2;
        let mut sample =self.wave_ram[byte_index as usize];
        if self.sample_index.is_multiple_of(2){
            sample >>= 4;
        }else{
            sample &= 0b0000_1111;
        }
        let centered_sample = (sample as f32 / 7.5) - 1.0;
        match self.volume_code{
            0 => 0.0,
            1 => centered_sample,
            2 => centered_sample * 0.5,
            3 => centered_sample * 0.25,
            _ => 0.0
        }
    }

    fn trigger(&mut self, frame_step: u8){
        if self.length_timer == 0{
            self.length_timer = 256;
            if !frame_step.is_multiple_of(2) && self.length_enable{
                self.length_timer -= 1;
            }
        }
        self.channel_enable = self.dac_enable;
        self.timer = self.period() + 6;
        self.sample_index = 0;
    }

    pub fn reset(&mut self){
        self.channel_enable = false;
        self.dac_enable = false;
        self.frequency = 0;
        self.timer = 0;
        self.length_enable = false;
        self.volume_code = 0;
        self.sample_index = 0;
    }

    pub fn parse_input(&mut self,addr: u16, byte: u8, frame_step: u8){
        match addr{
            0xFF1A => {
                self.dac_enable = (byte & 0b1000_0000) != 0;
                if !self.dac_enable{
                    self.channel_enable = false;
                }
            }
            0xFF1B => self.length_timer = 256 - (byte as u16),
            0xFF1C => self.volume_code = (byte & 0b0110_0000) >> 5,
            0xFF1D => self.frequency = (self.frequency & !0b1111_1111) | (byte as u16),
            0xFF1E => {
                let previous_length = self.length_enable;
                let triggered  = (byte & 0b1000_0000) != 0;
                self.frequency = (self.frequency & 0b1111_1111) | (((byte as u16) & 0b111) << 8);
                self.length_enable = (byte & 0b0100_0000) != 0 ;
                if !previous_length && self.length_enable && (!frame_step.is_multiple_of(2)) && (self.length_timer > 0){
                        self.length_timer -= 1;
                        if self.length_timer == 0{
                            self.channel_enable = false;
                    }
                }
                if triggered{
                    self.trigger(frame_step);
                }
            }
            0xFF30..=0xFF3F => {
                self.wave_ram[(addr - 0xFF30) as usize] = byte;
            }
            _ => ()
        }
    }
}

pub struct NoiseChannel{
    channel_enable: bool,
    dac_enable: bool,
    volume: u8,
    initial_volume: u8,
    envelope_period: u8,
    envelope_timer: u8,
    envelope_add: bool,
    length_timer: u8,
    length_enable: bool,
    lfsr: u16,
    timer: u32,
    clock_shift: u8,
    width_mode_7bit: bool,
    divisor_code: u8,
}

impl NoiseChannel{
    pub fn new() -> Self{
        NoiseChannel{
            channel_enable: false,
            dac_enable: false,
            volume: 0,
            initial_volume: 0,
            envelope_period: 0,
            envelope_timer: 0,
            envelope_add: false,
            length_timer: 0,
            length_enable: false,
            lfsr: 0b0111_1111_1111_1111,
            timer: 0,
            clock_shift: 0,
            width_mode_7bit: false,
            divisor_code: 0,
        }
    }

    fn period(&self) -> u32{
        let base_divisor = if self.divisor_code == 0 {8} else {(self.divisor_code as u32) * 16};
        base_divisor << self.clock_shift
    }

    fn trigger(&mut self, frame_step: u8){
        self.channel_enable = self.dac_enable;
        self.lfsr = 0b0111_1111_1111_1111;
        self.timer = self.period();
        self.envelope_timer = self.envelope_period;
        self.volume = self.initial_volume;
        if self.length_timer == 0{
            self.length_timer = 64;
            if !frame_step.is_multiple_of(2) && self.length_enable{
                self.length_timer -= 1;
            }
        }
    }

    pub fn tick(&mut self, cycles: u8){
        let mut cycles: u32 = cycles as u32;
        while cycles > 0{
            if cycles >= self.timer {
                cycles -= self.timer;
                self.timer = self.period();
                let xor_result = (self.lfsr & 0b0001) ^ ((self.lfsr >> 1) & 0b0001);
                self.lfsr >>= 1;
                self.lfsr = (self.lfsr & 0b0011_1111_1111_1111) | (xor_result << 14);
                if self.width_mode_7bit {
                    self.lfsr = (self.lfsr & 0b0111_1111_1011_1111) | (xor_result << 6);
                }
            }else{
                self.timer -= cycles;
                cycles = 0;
            }
        }
    }

    pub fn sample(&self) -> f32{
        if !self.channel_enable {return 0.0}
        if !self.dac_enable { return 0.0 }
        let amplitude = if(self.lfsr & 0b0001) == 0 {1.0} else {-1.0};
        amplitude * (self.volume as f32 / 15.0)
    }

    pub fn tick_frame_sequencer(&mut self, step: u8){
        if step == 7 && self.envelope_period > 0{
                self.envelope_timer = self.envelope_timer.saturating_sub(1);
                if self.envelope_timer == 0{
                    self.envelope_timer = self.envelope_period;
                    if self.envelope_add && (self.volume < 15){ self.volume += 1 }
                    else if !self.envelope_add && (self.volume > 0) { self.volume -= 1}
            }
        }
        if step.is_multiple_of(2) && self.length_enable && (self.length_timer > 0){
                self.length_timer -= 1;
                if self.length_timer == 0{
                    self.channel_enable = false;
                }
        }
    }

    pub fn reset(&mut self){
        self.channel_enable = false;
        self.dac_enable = false;
        self.volume = 0;
        self.initial_volume = 0;
        self.envelope_period = 0;
        self.envelope_timer = 0;
        self.envelope_add = false;
        self.length_enable = false;
        self.timer = 0;
        self.clock_shift = 0;
        self.width_mode_7bit = false;
        self.divisor_code = 0;
        self.lfsr = 0b0111_1111_1111_1111;
    }

    pub fn is_active(&self) -> bool{
        self.channel_enable
    }

    pub fn parse_input(&mut self, addr: u16, byte: u8, frame_step: u8){
        match addr {
            0xFF20 => {
                self.length_timer = 64 - (byte & 0b0011_1111)
            }
            0xFF21 => {
                self.dac_enable = (byte & 0b1111_1000) != 0;
                if !self.dac_enable{
                    self.channel_enable = false;
                }
                self.initial_volume = (byte >> 4) & 0b0000_1111;
                self.envelope_add = (byte & 0b0000_1000) != 0;
                self.envelope_period = byte & 0b0000_0111;
            }
            0xFF22 => {
                self.clock_shift = (byte >> 4) & 0b0000_1111;
                self.width_mode_7bit = (byte & 0b0000_1000) != 0;
                self.divisor_code = byte & 0b0000_0111;
            }
            0xFF23 => {
                let previous_length = self.length_enable;
                let triggered = (byte & 0b1000_0000) != 0;
                self.length_enable = (byte & 0b0100_0000) != 0;
                if !previous_length && self.length_enable && (!frame_step.is_multiple_of(2)){
                    if self.length_timer > 0{
                        self.length_timer -= 1;
                        if self.length_timer == 0{
                            self.channel_enable = false;
                        }
                    }
                }
                if triggered{
                    self.trigger(frame_step);
                }
            }

            _ => ()
            
        }
    }

    pub fn read(&self, addr : u16) -> u8{
        match addr {
            0xFF21 => (self.initial_volume << 4) | ((self.envelope_add as u8) << 3) | self.envelope_period,
            0xFF22 => (self.clock_shift << 4) | ((self.width_mode_7bit as u8) << 3) | self.divisor_code,
            0xFF23 => (self.length_enable as u8) << 6,
            _ => 0x00,
        }
    }
}

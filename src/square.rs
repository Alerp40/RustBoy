
const DUTY_REFERENCE: [[u8;8]; 4] = [
[0,0,0,0,0,0,0,1],
[1,0,0,0,0,0,0,1],
[1,0,0,0,0,1,1,1],
[0,1,1,1,1,1,1,0],
];

pub struct SquareChannel{
    negate_used: bool,
    has_sweep: bool,
    channel_enable: bool,
    dac_enable: bool,
    volume: u8,
    initial_volume: u8,
    envelope_period: u8,
    envelope_timer: u8,
    envelope_add: bool,
    length_timer: u8,
    length_enable: bool,
    timer : u32,
    frequency: u16,
    duty_step: u8,
    duty_pattern: u8,
    sweep_timer: u8,
    sweep_period: u8,
    sweep_shift: u8,
    sweep_decrease: bool,
    sweep_enabled: bool,
    shadow_frequency: u16,
}
impl SquareChannel{
    pub fn new(has_sweep: bool) -> Self{
        SquareChannel{
            negate_used: false,
            has_sweep,
            channel_enable: false,
            dac_enable: false,
            volume: 0,
            initial_volume: 0,
            envelope_period: 0,
            envelope_timer: 0,
            envelope_add: false,
            length_timer: 0,
            length_enable: false,
            timer: 0,
            frequency: 0,
            duty_step: 0,
            duty_pattern: 0,
            sweep_timer: 0,
            sweep_period: 0,
            sweep_shift: 0,
            sweep_decrease: false,
            sweep_enabled: false,
            shadow_frequency: 0,
        }
    }
    
    fn period(&self) -> u32{
        (2048 - self.frequency as u32) * 4
    }

    pub fn tick(&mut self, cycles: u8){
        let mut cycles: u32 = cycles as u32;
        while cycles > 0{
            if cycles >= self.timer {
                cycles -= self.timer;
                self.duty_step += 1;
                if self.duty_step == 8{
                    self.duty_step = 0;
                }
                self.timer = self.period()
            }else{
                self.timer -= cycles;
                cycles = 0;
            }
        }
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
        if ((step == 2) || (step == 6)) &&  self.has_sweep{
                self.sweep_timer = self.sweep_timer.saturating_sub(1);
                if self.sweep_timer == 0{
                    self.sweep_timer = if self.sweep_period == 0 {8} else {self.sweep_period};
                    if self.sweep_enabled && self.sweep_period > 0{
                        let new_freq = self.calculate_sweep_freq();
                        if new_freq > 2047 { self.channel_enable = false}
                        else if self.sweep_shift > 0 {
                            self.frequency = new_freq;
                            self.shadow_frequency = new_freq;
                            if self.calculate_sweep_freq() > 2047 { self.channel_enable = false }
                    }
                }
            }
        }
        if step.is_multiple_of(2) && self.length_enable && (self.length_timer > 0){
                self.length_timer -= 1;
                if self.length_timer == 0{
                    self.channel_enable = false;
                }
        }
    }

    pub fn sample(&self) -> f32{
        if !self.channel_enable {return 0.0}
        if !self.dac_enable { return 0.0 }
        let amplitude = DUTY_REFERENCE[self.duty_pattern as usize][self.duty_step as usize];
        let centered_wave = if amplitude == 1 {1.0} else {-1.0};
        centered_wave * (self.volume as f32 / 15.0)
    }

    fn calculate_sweep_freq(&mut self) -> u16{
        let offset = self.shadow_frequency >> self.sweep_shift;
        if self.sweep_decrease {self.negate_used = true}
        if self.sweep_decrease {self.shadow_frequency - offset} else {self.shadow_frequency + offset}
    }

    fn trigger(&mut self, frame_step: u8){
        self.negate_used = false;
        self.channel_enable = self.dac_enable;
        self.timer = self.period();
        if self.length_timer == 0{
            self.length_timer = 64;
            if !frame_step.is_multiple_of(2) && self.length_enable{
                self.length_timer -= 1;
            }
        }
        self.envelope_timer = self.envelope_period;
        self.volume = self.initial_volume;
        if self.has_sweep{
            self.shadow_frequency = self.frequency;
            self.sweep_timer = if self.sweep_period == 0 {8} else {self.sweep_period};
            self.sweep_enabled = (self.sweep_period > 0) || (self.sweep_shift > 0);
            if (self.sweep_shift > 0) && (self.calculate_sweep_freq() > 2047){
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
        self.frequency = 0;
        self.timer = 0;
        self.duty_step = 0;
        self.duty_pattern = 0;
        if self.has_sweep {
            self.sweep_shift = 0;
            self.sweep_decrease = false;
            self.sweep_period = 0;
            self.sweep_timer = 0;
            self.sweep_enabled = false;
            self.shadow_frequency = 0;
        }
    }

    pub fn is_active(&self) -> bool{
        self.channel_enable
    }

    pub fn parse_input(&mut self, addr: u16, byte: u8, frame_step: u8, apu_enabled: bool){
        match addr{
            0xFF10 => {
                self.sweep_period = (byte & 0b0111_0000) >> 4;
                self.sweep_decrease = (byte & 0b0000_1000) != 0;
                if self.negate_used & !self.sweep_decrease{
                    self.channel_enable = false;
                }
                self.sweep_shift = byte & 0b0000_0111
            }
            0xFF11 | 0xFF16 => {
                if apu_enabled {
                    self.duty_pattern = (byte & 0b1100_0000) >> 6;
                }
                self.length_timer = 64 - (byte & 0b0011_1111);
            }
            0xFF12 | 0xFF17 => {
                self.dac_enable = (byte & 0b1111_1000) != 0;
                if !self.dac_enable{
                    self.channel_enable = false;
                }
                self.initial_volume = (byte & 0b1111_0000) >> 4;
                self.envelope_add = (byte & 0b0000_1000) != 0;
                self.envelope_period = byte & 0b0000_0111;
            }
            0xFF13 | 0xFF18 => {
                self.frequency = (self.frequency & !0b1111_1111) | (byte as u16)
            }
            0xFF14 | 0xFF19 => {
                let previous_length = self.length_enable;
                let triggered = (byte & 0b1000_0000) != 0;
                self.frequency = (self.frequency & 0b1111_1111) | (((byte as u16) & 0b0000_0111) <<8);
                self.length_enable = (byte & 0b0100_0000) != 0;
                if !previous_length && self.length_enable && (!frame_step.is_multiple_of(2)){
                    if self.length_timer > 0{
                        self.length_timer -= 1;
                        if self.length_timer == 0{
                            self.channel_enable = false;
                        }
                    }
                }
                if triggered {
                    self.trigger(frame_step);
                }
            }

            _ => (),
        }
    }

    pub fn read(&self, addr : u16) -> u8{
        match addr {
            0xFF10 => (self.sweep_period << 4) | ((self.sweep_decrease as u8) << 3) | self.sweep_shift,
            0xFF11 | 0xFF16 => self.duty_pattern << 6,
            0xFF12 | 0xFF17 => (self.initial_volume << 4) | ((self.envelope_add as u8) << 3) | self.envelope_period,
            0xFF14 | 0xFF19 => (self.length_enable as u8) << 6,
            _ => 0x00,
        }
    }
}

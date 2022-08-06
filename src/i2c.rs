#![allow(dead_code)]
#![allow(unused_variables)]

use crate::{wait_exact_ns};
use crate::clock::*;
use crate::phys::pins::*;


#[derive(Clone, Copy)]
pub enum I2CSpeed {
    Fast400kHz = 1250,
    Normal100kHz = 2500,
}

pub struct I2C {
    sda_pin: usize,
    scl_pin: usize,
    speed: I2CSpeed,
}

impl I2C {
    pub fn begin(sda: usize, scl: usize) -> Self {

        pin_pad_config(sda, PadConfig { 
            hysterisis: true, 
            resistance: PullUpDown::PullUp22k, 
            pull_keep: PullKeep::Pull, 
            pull_keep_en: true, 
            open_drain: true, 
            speed: PinSpeed::Medium100MHz, 
            drive_strength: DriveStrength::MaxDiv3, 
            fast_slew_rate: true 
        });

        pin_pad_config(scl, PadConfig {
            hysterisis: true, 
            resistance: PullUpDown::PullUp22k, 
            pull_keep: PullKeep::Pull, 
            pull_keep_en: true, 
            open_drain: true, 
            speed: PinSpeed::Medium100MHz, 
            drive_strength: DriveStrength::MaxDiv3, 
            fast_slew_rate: true 
        });

        pin_mode(sda, Mode::Output);
        pin_mode(scl, Mode::Output);
        pin_out(sda, Power::High);
        pin_out(scl, Power::High);

        return I2C { 
            sda_pin: sda,
            scl_pin: scl,
            speed: I2CSpeed::Normal100kHz,
        };
    }

    pub fn begin_transmission(&self, address: u8, write_mode: bool) -> bool {
        // Start transmission
        i2c_start_condition(&self);

        // Address frame
        let mut mask = 0x1 << 6;
        for _ in 0 ..= 6 {
            let high = address & mask;
            i2c_write_bit(&self, high > 0);
            mask >>= 1;
        }

        // R/W bit
        if write_mode {
            i2c_write_bit(&self, false);
        } else {
            i2c_write_bit(&self, true);
        }
        // Ack bit
        let ack = i2c_read_bit(&self);
        if ack == false {
            // Success
            return true;
        } else {
            // Transmissino not acknowledged. Terminate.
            i2c_end_condition(&self);
            return false;
        }

    }

    pub fn end_transmission(&self, address: u8) {
        i2c_end_condition(&self);
    }

    pub fn write(&self, bytes: &[u8]) -> bool {
        for byte in bytes {
            let mut mask = 0x1 << 7;
            for _ in 0 ..= 7 {
                let high = byte & mask;
                i2c_write_bit(&self, high > 0);
                mask >>= 1;
            }
            let ack = i2c_read_bit(&self);
            if ack == false {
                // Success
            } else {
                // Not acknowledged
                i2c_end_condition(&self);
                return false;
            }
        }
        return true;
    }

    pub fn read(&self, ack: bool) -> u8 {
        let mut byte: u8 = 0;
        let mut mask = 0x1 << 7;

        for _ in 0 .. 8 {
            if i2c_read_bit(&self) {
                byte |= mask;
            }
            mask >>= 1;
        }

        if ack {
            // Send the ack bit
            i2c_write_bit(&self, false);
        }

        return byte;
    }

    pub fn set_speed(&mut self, speed: I2CSpeed) {
        self.speed = speed;
    }

}

fn clock_high(i2c: &I2C) {
    pin_mode(i2c.scl_pin, Mode::Output);
    pin_out(i2c.scl_pin, Power::High);
    wait_exact_ns(500);
}

fn clock_low(i2c: &I2C) {
    pin_mode(i2c.scl_pin, Mode::Output);
    pin_out(i2c.scl_pin, Power::Low);
    wait_exact_ns(500);
}

fn data_high(i2c: &I2C) {
    pin_mode(i2c.sda_pin, Mode::Output);
    pin_out(i2c.sda_pin, Power::High);
    wait_exact_ns(500);
}

fn data_low(i2c: &I2C) {
    pin_mode(i2c.sda_pin, Mode::Output);
    pin_out(i2c.sda_pin, Power::Low);
    wait_exact_ns(500);
}

fn data_release(i2c: &I2C) {
    pin_mode(i2c.sda_pin, Mode::Input);
    wait_exact_ns(500);
}

fn clock_release(i2c: &I2C) {
    pin_mode(i2c.scl_pin, Mode::Input);
    wait_exact_ns(500);
}

fn i2c_start_condition(i2c: &I2C) {
    data_low(&i2c);
    clock_low(&i2c);
}

#[no_mangle]
fn i2c_read_bit(i2c: &I2C) -> bool {
    clock_low(&i2c);
    data_release(&i2c);

    // **************
    // Pulse the clock
    // **************
    clock_release(&i2c);
    let timeout = nanos() + (i2c.speed as uNano * 4);
    let stretch_timeout = nanos() + (i2c.speed as uNano * 16);
    let mut res = true;

    loop {

        // Check for stretch condition
        let now = nanos();
        let clock_line = pin_read(i2c.scl_pin);
        let data_line = pin_read(i2c.sda_pin);

        if clock_line == 0 && now < stretch_timeout {
            // We are stretching the signal
            continue;
        } else if now > timeout {
            break;
        } else if data_line == 0 {
            res = false;
        }

        wait_exact_ns(500);
    }

    // Bring clock back down
    clock_low(&i2c);
    data_low(&i2c);

    return res;
}

fn i2c_write_bit(i2c: &I2C, high: bool) {
    if high {
        data_high(&i2c);
    } else {
        data_low(&i2c);
    }

    // Wait
    wait_exact_ns(i2c.speed as uNano);

    // **************
    // Pulse the clock
    // **************
    clock_release(&i2c);
    wait_exact_ns((i2c.speed as uNano) * 2);

    // Pull clock low
    clock_low(&i2c);
    wait_exact_ns(i2c.speed as uNano);
}

fn i2c_end_condition(i2c: &I2C) {
    clock_release(&i2c);
    wait_exact_ns(500);
    data_release(&i2c);
    wait_exact_ns(500);
}


//! This module provides the ability to designate any two
//! gpio pins as SDA/SCL which allows you to introduce i2c
//! capabilities into your project.
//! 
//! In order to use these pins, you must include a pull-up
//! resistor on both lines.
#![allow(dead_code, unused_imports)]
#![allow(unused_variables)]

use core::arch::asm;
use crate::debug::{debug_str, debug_binary, debug_u64, debug_hex};
use crate::{wait_exact_ns, assembly};
use crate::clock::*;
use crate::phys::pins::*;

const PAUSE: uNano = 1000;

#[derive(Clone, Copy)]
pub enum I2CSpeed {
    Fast400kHz = 1250,
    Normal100kHz = 2500,
}

/// Represents a two-wire i2c controller.
pub struct I2C {
    /// The pin (as described on the board itself) referencing the sda line.
    sda_pin: usize,
    /// The pin (as described on the board itself) referencing the scl line.
    scl_pin: usize,
    /// The speed at which to drive the clock signals.
    speed: I2CSpeed,
}

impl I2C {
    /// This method creates a new instance of an i2c controller.
    /// After specifying the pins on which sda and scl lines reside,
    /// the system will configure those pins as open-drain.
    /// 
    /// This means you must have a pull-up resistor for each
    /// line on your circuit.
    /// 
    /// ```
    /// let mut wire = I2C::Begin(19, 18);
    /// ```
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
        
        pin_mode(scl, Mode::Input);
        pin_mode(sda, Mode::Input);

        return I2C { 
            sda_pin: sda,
            scl_pin: scl,
            speed: I2CSpeed::Normal100kHz,
        };
    }

    /// This method begins a new i2c transmission by sending
    /// the start condition signal and then transmitting
    /// the device select packet.
    /// 
    /// If the write_mode parameter is true, the R/W bit will
    /// be 0, signalling to the downstream devices that
    /// a write operation will follow.
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
            // debug_str(b"ACK!");
            // Success
            return true;
        } else {
            debug_str(b"failed to receive ack");
            // Transmissino not acknowledged. Terminate.
            i2c_end_condition(&self);
            return false;
        }

    }

    /// This method terminates an existing i2c transmission by 
    /// sending the stop condition signal.
    pub fn end_transmission(&self) {
        i2c_end_condition(&self);
    }

    /// This method will write a series of bytes to
    /// the i2c bus. After each byte, the controller
    /// will expect an acknowledgement.
    /// 
    /// In order to use this method successfully,
    /// you must first have invoked `i2c.begin_transmission()`
    /// 
    /// ```
    /// let mut wire = I2C::begin(19, 18);
    /// wire.begin_transmission(0x50, true)
    /// wire.write(&[0, 0]);
    /// wire.write(b"hello");
    /// wire.end_transmission();
    /// ```
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
                debug_hex(bytes[0] as u32, b"failed write add");
                debug_hex(bytes[1] as u32, b"failed write value");
                // i2c_end_condition(&self);
                return false;
            }
        }
        return true;
    }


    /// This method will read a single byte
    /// from the downstream device.
    /// 
    /// If the ack parameter is true, after reading
    /// from the downstream device, the teensy will
    /// send an acknowledgement bit.
    /// 
    /// In order to use this method successfully,
    /// you must first have invoked `i2c.begin_transmission()`
    /// 
    /// ```
    /// let mut wire = I2C::begin(19, 18);
    /// wire.begin_transmission(0x50, true)
    /// let str = &[
    ///     wire.read(true),
    ///     wire.read(true),
    ///     wire.read(true),
    ///     wire.read(true),
    ///     wire.read(true),
    /// ];
    /// wire.end_transmission();
    /// ```
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
        } else {
            // Send the nack bit
            i2c_write_bit(&self, true);
        }

        // clock_release(&self);

        return byte;
    }

    pub fn read_burst<const T: usize>(&self) -> [u8; T] {
        let mut bytes = [0; T];

        for idx in 0 .. T {
            bytes[idx] = self.read(true);
        }
        self.read(false);
        return bytes;
    }

    /// This method will change the signal speed.
    /// By default, all signals are clocked at 100kHz
    /// but if you upgrade to fast mode, it'll be 400kHz.
    /// 
    /// ```
    /// let mut wire = I2C::Begin(19, 18);
    /// wire.set_speed(I2CSpeed::Fast400kHz);
    /// ```
    pub fn set_speed(&mut self, speed: I2CSpeed) {
        self.speed = speed;
    }

}

fn clock_high(i2c: &I2C) {
    pin_mode(i2c.scl_pin, Mode::Output);
    pin_out(i2c.scl_pin, Power::High);
    wait_exact_ns(PAUSE);
}

fn clock_low(i2c: &I2C) {
    pin_mode(i2c.scl_pin, Mode::Output);
    pin_out(i2c.scl_pin, Power::Low);
    wait_exact_ns(PAUSE);
}

fn data_high(i2c: &I2C) {
    pin_mode(i2c.sda_pin, Mode::Output);
    pin_out(i2c.sda_pin, Power::High);
    wait_exact_ns(PAUSE);
}

fn data_low(i2c: &I2C) {
    pin_mode(i2c.sda_pin, Mode::Output);
    pin_out(i2c.sda_pin, Power::Low);
    wait_exact_ns(PAUSE);
}

fn data_release(i2c: &I2C) {
    pin_mode(i2c.sda_pin, Mode::Input);
    wait_exact_ns(PAUSE);
}

fn clock_release(i2c: &I2C) {
    pin_mode(i2c.scl_pin, Mode::Input);
    wait_exact_ns(PAUSE);
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


        // if clock_line == 0 && now < stretch_timeout {
        //     // We are stretching the signal
        //     assembly!("nop");
        //     continue;
        // } else 
        if clock_line == 0 && now >= timeout && now < stretch_timeout {
            // We are stretching the signal
            assembly!("nop");
            continue;
        } else if data_line == 0 {
            res = false;
        }

        if now > timeout {
            break;
        }

        wait_exact_ns(PAUSE);
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
    wait_exact_ns(PAUSE);
    data_release(&i2c);
    wait_exact_ns(PAUSE);
}


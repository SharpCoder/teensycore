//! This module provides the ability to designate any two
//! gpio pins as SDA/SCL which allows you to introduce i2c
//! capabilities into your project.
//!
//! In order to use these pins, you must include a pull-up
//! resistor on both lines.
#![allow(dead_code, unused_imports)]
#![allow(unused_variables)]

use crate::clock::*;
use crate::debug::{debug_binary, debug_hex, debug_str, debug_u64};
use crate::phys::pins::*;
use crate::{assembly, wait_exact_ns};
use core::arch::asm;

const PAUSE: uNano = 1000;

#[derive(Clone, Copy)]
pub enum I2CSpeed {
    Fast400kHz = 625,
    Normal100kHz = 2500,
    Debug10kHz = 25000,
}

/// Represents a two-wire i2c controller.
pub struct I2C {
    /// The pin (as described on the board itself) referencing the sda line.
    sda_pin: usize,
    /// The pin (as described on the board itself) referencing the scl line.
    scl_pin: usize,
    /// The speed at which to drive the clock signals.
    speed: I2CSpeed,
    /// If true, debug messages will be written to SerioDebug
    pub debug: bool,
}

impl I2C {
    /// This method creates a new instance of an i2c controller.
    /// After specifying the pins on which sda and scl lines reside,
    /// the system will configure those pins as open-drain.
    ///
    /// This means you must have a pull-up resistor for each
    /// line on your circuit.
    ///
    /// ```no_run
    /// use teensycore::i2c::*;
    /// let mut wire = I2C::begin(19, 18);
    /// ```
    pub fn begin(sda: usize, scl: usize) -> Self {
        pin_pad_config(
            sda,
            PadConfig {
                hysterisis: false,
                resistance: PullUpDown::PullUp22k,
                pull_keep: PullKeep::Pull,
                pull_keep_en: true,
                open_drain: true,
                speed: PinSpeed::Max200MHz,
                drive_strength: DriveStrength::MaxDiv3,
                fast_slew_rate: true,
            },
        );

        pin_pad_config(
            scl,
            PadConfig {
                hysterisis: false,
                resistance: PullUpDown::PullUp22k,
                pull_keep: PullKeep::Pull,
                pull_keep_en: true,
                open_drain: true,
                speed: PinSpeed::Max200MHz,
                drive_strength: DriveStrength::MaxDiv3,
                fast_slew_rate: true,
            },
        );

        pin_mode(scl, Mode::Output);
        pin_out(scl, Power::Low);

        return I2C {
            sda_pin: sda,
            scl_pin: scl,
            speed: I2CSpeed::Normal100kHz,
            debug: false,
        };
    }

    /// This method creates a new instance of an i2c controller.
    /// After specifying the pins on which sda and scl lines reside,
    /// the system will configure those pins as open-drain.
    ///
    /// begin_with_external_power will assume the SDA/SCL lines are already
    /// configured with pull-up resistors and it will not provide
    /// any power to those pins. Useful for driving lower-powered
    /// devices.
    ///
    /// ```no_run
    /// use teensycore::i2c::*;
    /// let mut wire = I2C::begin_with_external_power(19, 18);
    /// ```
    pub fn begin_with_external_power(sda: usize, scl: usize) -> Self {
        pin_pad_config(
            sda,
            PadConfig {
                hysterisis: false,
                resistance: PullUpDown::PullUp22k,
                pull_keep: PullKeep::Keeper,
                pull_keep_en: true,
                open_drain: true,
                speed: PinSpeed::Max200MHz,
                drive_strength: DriveStrength::MaxDiv7,
                fast_slew_rate: true,
            },
        );

        pin_pad_config(
            scl,
            PadConfig {
                hysterisis: false,
                resistance: PullUpDown::PullUp22k,
                pull_keep: PullKeep::Keeper,
                pull_keep_en: true,
                open_drain: true,
                speed: PinSpeed::Max200MHz,
                drive_strength: DriveStrength::MaxDiv7,
                fast_slew_rate: true,
            },
        );

        pin_mode(scl, Mode::Output);
        pin_out(scl, Power::Low);
        pin_out(sda, Power::Low);

        return I2C {
            sda_pin: sda,
            scl_pin: scl,
            speed: I2CSpeed::Normal100kHz,
            debug: false,
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
        for _ in 0..=6 {
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
            if self.debug {
                debug_str(b"received ack!!!!");
            }
            // Success
            return true;
        } else {
            if self.debug {
                debug_str(b"failed to receive ack");
            }
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
    /// ```no_run
    /// use teensycore::i2c::*;
    /// let mut wire = I2C::begin(19, 18);
    /// wire.begin_transmission(0x50, true);
    /// wire.write(&[0, 0]);
    /// wire.write(b"hello");
    /// wire.end_transmission();
    /// ```
    pub fn write(&self, bytes: &[u8]) -> bool {
        for byte in bytes {
            let mut mask = 0x1 << 7;
            for _ in 0..=7 {
                let high = byte & mask;
                i2c_write_bit(&self, high > 0);
                mask >>= 1;
            }
            let ack = i2c_read_bit(&self);
            if ack == false {
                // Success
            } else {
                // Not acknowledged
                if self.debug {
                    debug_hex(bytes[0] as u32, b"[failed write] @ address");
                    debug_hex(bytes[1] as u32, b"[failed value]");
                }

                // return false;
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
    /// ```no_run
    /// use teensycore::i2c::*;
    /// let mut wire = I2C::begin(19, 18);
    /// wire.begin_transmission(0x50, true);
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

        for _ in 0..8 {
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

        return byte;
    }

    /// This method will read a series of bytes in rapid
    /// succession based from the currently open i2c device.
    ///
    /// ```no_run
    /// use teensycore::i2c::*;
    /// let mut wire = I2C::begin(19, 18);
    /// wire.begin_transmission(0xD, true);
    /// wire.write(&[0x3B]);
    /// wire.begin_transmission(0xD, false);
    ///
    /// let bytes = wire.read_burst::<14>();
    ///
    /// wire.end_transmission();
    /// ```
    pub fn read_burst<const T: usize>(&self) -> [u8; T] {
        let mut bytes = [0; T];

        for idx in 0..T {
            bytes[idx] = self.read(true);
        }
        self.read(false);
        return bytes;
    }

    /// This method will change the signal speed.
    /// By default, all signals are clocked at 100kHz
    /// but if you upgrade to fast mode, it'll be 400kHz.
    ///
    /// ```no_run
    /// use teensycore::i2c::*;
    /// let mut wire = I2C::begin(19, 18);
    /// wire.set_speed(I2CSpeed::Fast400kHz);
    /// ```
    pub fn set_speed(&mut self, speed: I2CSpeed) {
        self.speed = speed;
    }

    /// This method will change the debug setting
    /// for the i2d device. If true, debug information
    /// will be output to the SerioDebug Serial channel
    /// with some loose information about missed ACK
    /// and NACK messages.
    ///
    /// ```no_run
    /// use teensycore::i2c::*;
    /// let mut wire = I2C::begin(19, 18);
    /// wire.set_debug(true);
    /// ```
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }
}

fn clock_high(i2c: &I2C) {
    pin_mode(i2c.scl_pin, Mode::Input);
}

fn clock_low(i2c: &I2C) {
    pin_out(i2c.scl_pin, Power::Low);
    pin_mode(i2c.scl_pin, Mode::Output);
}

fn data_high(i2c: &I2C) {
    pin_mode(i2c.sda_pin, Mode::Input);
}

fn data_low(i2c: &I2C) {
    pin_out(i2c.sda_pin, Power::Low);
    pin_mode(i2c.sda_pin, Mode::Output);
}

fn data_release(i2c: &I2C) {
    pin_mode(i2c.sda_pin, Mode::Input);
}

fn clock_release(i2c: &I2C) {
    pin_mode(i2c.scl_pin, Mode::Input);
}

fn i2c_start_condition(i2c: &I2C) {
    clock_high(&i2c);
    wait_exact_ns(PAUSE);
    data_high(&i2c);
    wait_exact_ns(PAUSE);
    data_low(&i2c);
    wait_exact_ns(PAUSE);
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
    let mut res = true;

    loop {
        // Check for stretch condition
        let now = nanos();
        let clock_line = pin_read(i2c.scl_pin);
        let data_line = pin_read(i2c.sda_pin);

        if clock_line == 0 && now < timeout {
            // We are stretching the signal
            assembly!("nop");
            continue;
        }

        if data_line == 0 {
            res = false;
        }

        if now > timeout {
            break;
        }

        wait_exact_ns(PAUSE);
    }

    // Bring clock back down
    clock_low(&i2c);

    return res;
}

fn i2c_write_bit(i2c: &I2C, high: bool) {
    clock_low(&i2c);

    if high {
        data_high(&i2c);
    } else {
        data_low(&i2c);
    }

    // **************
    // Pulse the clock
    // **************
    clock_high(&i2c);
    wait_exact_ns((i2c.speed as uNano) * 3);

    // Pull clock low
    clock_low(&i2c);
}

fn i2c_end_condition(i2c: &I2C) {
    clock_low(&i2c);
    data_low(&i2c);
    wait_exact_ns(PAUSE);
    clock_high(&i2c);
    wait_exact_ns(PAUSE);
    data_high(&i2c);
    wait_exact_ns(PAUSE);
}

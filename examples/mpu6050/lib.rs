//! mpu6050
//!
//! This example demonstrates how to write a full driver for
//! the MPU-6050 6DoF sensor. It has been tested on physical
//! hardware and worked out pretty well.
//!
//! This code is not for production! It's got a lot of issues.
use teensycore::prelude::*;

const ADDR: u8 = 0x68;

pub const GRAVITY_EARTH: f32 = 9.80665f32;

pub struct SensorData {
    pub accel: Vector3D,
    pub gyro: Vector3D,
}

pub struct Vector3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

static mut ACCELEROMETER_RANGE: AccelerometerRange = AccelerometerRange::Normal8g;
static mut GYROSCOPE_RANGE: GyroscopeRange = GyroscopeRange::Low250;

#[derive(Clone, Copy)]
pub enum FilterBandwidth {
    Hz260 = 0, // Delay 0ms
    Hz184 = 1, // Delay 2ms
    Hz94 = 2,  // Delay 3ms
    Hz44 = 3,  // Delay 4.9ms
    Hz21 = 4,  // Delay 8.5ms
    Hz10 = 5,  // Delay 13.8ms
    Hz5 = 6,   // Delay 19ms
}

#[derive(Clone, Copy)]
pub enum GyroscopeRange {
    Low250 = 0,
    Med500 = 1,
    Normal1000 = 2,
    Extra2000 = 3,
}

#[derive(Copy, Clone)]
pub enum AccelerometerRange {
    Low2g = 0,
    Med4g = 1,
    Normal8g = 2,
    Extra16g = 3,
}

/// Perform a single byte write to a specific register address.
fn mpu6050_bus_write(i2c: &I2C, byte_addr: u8, byte: u8) {
    i2c.begin_transmission(ADDR, true);
    i2c.write(&[byte_addr, byte]);
    i2c.end_transmission();
}

/// Perform a single byte read from a specific register address.
fn mpu6050_bus_read(i2c: &I2C, byte_addr: u8) -> u8 {
    i2c.begin_transmission(ADDR, true);
    i2c.write(&[byte_addr]);
    i2c.begin_transmission(ADDR, false);
    let val = i2c.read(false);
    i2c.end_transmission();
    return val;
}

/// Initialize the MPU-6050 device.
pub fn mpu6050_init(i2c: &I2C) {
    // Find the device
    let mut found = false;
    for _ in 0..10 {
        if i2c.debug {
            debug_str(b"Searching for mpu6050");
        }
        i2c.begin_transmission(ADDR, true);
        i2c.write(&[0x75]);
        i2c.begin_transmission(ADDR, false);
        let addr = i2c.read(false);
        i2c.end_transmission();

        if addr == 0x68 {
            if i2c.debug {
                debug_u64(addr as u64, b"Found device!");
            }
            found = true;
            break;
        }
    }

    if !found {
        debug_str(b"Failed to connect to MPU6050");
        loop {}
    }

    // Device reset
    mpu6050_bus_write(&i2c, 0x6B, 0x80);
    wait_exact_ns(MS_TO_NANO * 50);
    if i2c.debug {
        debug_str(b"reset device");
    }

    loop {
        let status = mpu6050_bus_read(&i2c, 0x6B);
        if status & 0x80 == 0 {
            break;
        }
    }

    wait_exact_ns(MS_TO_NANO * 100);

    // MPU6050_SIGNAL_PATH_RESET
    mpu6050_bus_write(&i2c, 0x68, 0x7);
    wait_exact_ns(teensycore::MS_TO_NANO * 100);

    mpu6050_set_sample_divisor(&i2c, 0);
    mpu6050_set_filter_bandwidth(&i2c, FilterBandwidth::Hz10);
    mpu6050_set_gyroscope_range(&i2c, GyroscopeRange::Normal1000);
    mpu6050_set_accelerometer_range(&i2c, AccelerometerRange::Normal8g);

    // Make sure nothing is in standby mode
    mpu6050_bus_write(&i2c, 0x6C, 0x0);
    wait_exact_ns(teensycore::MS_TO_NANO * 100);

    // Disable all FIFO queues
    mpu6050_bus_write(&i2c, 0x23, 0);
    wait_exact_ns(teensycore::MS_TO_NANO * 300);

    // Write power management
    mpu6050_bus_write(&i2c, 0x6B, 0);
    wait_exact_ns(teensycore::MS_TO_NANO * 100);
}

/// Convert the raw gyro reading based on the configured full-scale range.
fn conv_raw_gyro(value: f32) -> f32 {
    let gyro_scale: f32 = match unsafe { GYROSCOPE_RANGE } {
        GyroscopeRange::Low250 => 250.0 / 32768.0,
        GyroscopeRange::Med500 => 500.0 / 32768.0,
        GyroscopeRange::Normal1000 => 1000.0 / 32768.0,
        GyroscopeRange::Extra2000 => 2000.0 / 32768.0,
    };

    return value * gyro_scale; // / gyro_scale;
}

/// Convert the raw accelerometer reading based on the configured full-scale range.
fn conv_raw_accel(value: f32) -> f32 {
    let accel_scale: f32 = match unsafe { ACCELEROMETER_RANGE } {
        AccelerometerRange::Low2g => 16384.0,
        AccelerometerRange::Med4g => 8192.0,
        AccelerometerRange::Normal8g => 4096.0,
        AccelerometerRange::Extra16g => 2048.0,
    };

    return (value / accel_scale) * GRAVITY_EARTH;
}

pub fn mpu6050_set_sample_divisor(i2c: &I2C, sample_rate: u8) {
    mpu6050_bus_write(&i2c, 0x19, sample_rate);
}

pub fn mpu6050_set_filter_bandwidth(i2c: &I2C, bandwidth: FilterBandwidth) {
    mpu6050_bus_write(&i2c, 0x1A, bandwidth as u8);
}

pub fn mpu6050_set_accelerometer_range(i2c: &I2C, range: AccelerometerRange) {
    unsafe {
        ACCELEROMETER_RANGE = range;
    }

    mpu6050_bus_write(&i2c, 0x1C, (range as u8) << 3);
}

pub fn mpu6050_set_gyroscope_range(i2c: &I2C, range: GyroscopeRange) {
    unsafe {
        GYROSCOPE_RANGE = range;
    }
    mpu6050_bus_write(&i2c, 0x1B, (range as u8) << 3);
}

/// Run the accelerometer self-test.
/// NOTE: This has never worked.
pub fn mpu6050_self_test_accelerometer(i2c: &I2C) -> bool {
    // calculate the value
    let value = (0b111 << 5) | ((unsafe { ACCELEROMETER_RANGE } as u8) << 3);
    wait_exact_ns(MS_TO_NANO * 60);

    // First, read the current values
    let untested_values = vector_read(&i2c, 0x3B);

    // set the range and activate the test
    mpu6050_bus_write(&i2c, 0x1C, value);
    wait_exact_ns(330 * MS_TO_NANO);

    let tested_values = vector_read(&i2c, 0x3B);

    // Disable self-test
    mpu6050_bus_write(&i2c, 0x1C, (unsafe { ACCELEROMETER_RANGE } as u8) << 3);

    // Evaluate the results
    let x_result = conv_raw_accel(tested_values.x) - conv_raw_accel(untested_values.x);
    let y_result = conv_raw_accel(tested_values.y) - conv_raw_accel(untested_values.y);
    let z_result = conv_raw_accel(tested_values.z) - conv_raw_accel(untested_values.z);

    return x_result != 0.0
        && y_result != 0.0
        && z_result != 0.0
        && x_result > -14.0
        && x_result < 14.0
        && y_result > -14.0
        && y_result < 14.0
        && z_result > -14.0
        && z_result < 14.0;
}

/// Run the gyroscope self-test.
/// NOTE: This has never worked.
pub fn mpu6050_self_test_gyroscope(i2c: &I2C) -> bool {
    // Calculate the value
    let value = (0b111 << 5) | ((unsafe { GYROSCOPE_RANGE } as u8) << 3);

    // First, read the current values
    let untested_values = vector_read(&i2c, 0x43);

    // Set the range and activate the gyroscope self_test.
    mpu6050_bus_write(&i2c, 0x1B, value);
    wait_exact_ns(33 * MS_TO_NANO);

    // Now evaluate the result
    let tested_values = vector_read(&i2c, 0x43);

    // Disable the self-test
    mpu6050_bus_write(&i2c, 0x1B, (unsafe { GYROSCOPE_RANGE } as u8) << 3);

    // Evaluate the result
    let x_result = conv_raw_gyro(tested_values.x) - conv_raw_gyro(untested_values.x);
    let y_result = conv_raw_gyro(tested_values.y) - conv_raw_gyro(untested_values.y);
    let z_result = conv_raw_gyro(tested_values.z) - conv_raw_gyro(untested_values.z);

    return x_result != 0.0
        && y_result != 0.0
        && z_result != 0.0
        && x_result > -14.0
        && x_result < 14.0
        && y_result > -14.0
        && y_result < 14.0
        && z_result > -14.0
        && z_result < 14.0;
}

/// Read all sensor data from the MPU-6050.
pub fn mpu6050_read_sensors(i2c: &I2C) -> SensorData {
    let bytes = [
        mpu6050_bus_read(&i2c, 0x3B),
        mpu6050_bus_read(&i2c, 0x3C),
        mpu6050_bus_read(&i2c, 0x3D),
        mpu6050_bus_read(&i2c, 0x3E),
        mpu6050_bus_read(&i2c, 0x3F),
        mpu6050_bus_read(&i2c, 0x40),
        mpu6050_bus_read(&i2c, 0x41),
        mpu6050_bus_read(&i2c, 0x42),
        mpu6050_bus_read(&i2c, 0x43),
        mpu6050_bus_read(&i2c, 0x44),
        mpu6050_bus_read(&i2c, 0x45),
        mpu6050_bus_read(&i2c, 0x46),
        mpu6050_bus_read(&i2c, 0x47),
        mpu6050_bus_read(&i2c, 0x48),
    ];

    // NOTE: This should work, but the sensor didn't
    // like it in practice.
    // i2c.begin_transmission(ADDR, true);
    // i2c.write(&[0x3B]); // ACCEL_XOUT_H register address
    // i2c.begin_transmission(ADDR, false);
    // let bytes = i2c.read_burst::<14>();
    // i2c.end_transmission();

    // Read all the data
    // Start with accelerometer
    let accel_x = f32_conv(&bytes, 0);
    let accel_y = f32_conv(&bytes, 2);
    let accel_z = f32_conv(&bytes, 4);

    // Read the gyroscope data
    let gyro_x = f32_conv(&bytes, 8);
    let gyro_y = f32_conv(&bytes, 10);
    let gyro_z = f32_conv(&bytes, 12);

    // Return the converted results.
    return SensorData {
        accel: Vector3D {
            x: conv_raw_accel(accel_x),
            y: conv_raw_accel(accel_y),
            z: conv_raw_accel(accel_z),
        },
        gyro: Vector3D {
            x: conv_raw_gyro(gyro_x),
            y: conv_raw_gyro(gyro_y),
            z: conv_raw_gyro(gyro_z),
        },
    };
}

/// Read 6 registers at once.
fn vector_read(i2c: &I2C, start_addr: u8) -> Vector3D {
    i2c.begin_transmission(ADDR, true);
    i2c.write(&[start_addr]);
    i2c.begin_transmission(ADDR, false);
    let bytes = i2c.read_burst::<6>();
    i2c.end_transmission();

    let x = f32_conv(&bytes, 0);
    let y = f32_conv(&bytes, 2);
    let z = f32_conv(&bytes, 4);

    return Vector3D { x: x, y: y, z: z };
}

/// Given an array, convert two u8 words into
/// a two's-complement f32 value.
fn f32_conv(bytes: &[u8], idx: usize) -> f32 {
    let high_bits = bytes[idx] as u16;
    let low_bits = bytes[idx + 1] as u16;
    let components = (high_bits << 8) | low_bits;
    return match (components & 0x8000) > 0 {
        true => -(!components as f32),
        false => components as f32,
    };
}

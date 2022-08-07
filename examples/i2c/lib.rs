//! i2c
//! 
//! This example demonstrates how to output i2c data over two wires.
//! You can use any two gpio pins for the SDA/SCL lines but you must
//! manually add a pull-up resistor to both lines in your circuit.
//! 
//! Following is a small example of writing and reading EEPROM
//! data from a 24LC512 chip.
#![feature(lang_items)]
#![crate_type = "staticlib"]
#![no_std]

teensycore::main!({
    use teensycore::*;

    let mut wire = I2C::begin(18, 19);
    wire.set_speed(I2CSpeed::Fast400kHz);

    // Begin i2c transaction
    wire.begin_transmission(0x50, true);
    // First two bytes are memory address
    wire.write(&[0, 0]);
    // Next is a sequential write of data
    wire.write(b"EARTH");
    wire.end_transmission();
    
    // Settle time for whole-page write. Per docs.
    wait_ns(250 * MS_TO_NANO);

    // Select the address we wish to read
    wire.begin_transmission(0x50, true);
    wire.write(&[0, 0]);
    wire.end_transmission();

    // Perform read request
    wire.begin_transmission(0x50, false);
    // Use the `debug_str` functionality to output this data to the
    // TX2 UART. Pin 8 on the teensy.
    debug_str(&[
        // Send 'true' as the second parameter to include an ack
        // This tells the chip we wish to do sequential reads
        // with automatic addr incrementation.
        wire.read(true),
        wire.read(true),
        wire.read(true),
        wire.read(true),
        wire.read(true),
    ]);
    wire.end_transmission();
});
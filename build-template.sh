#!/bin/sh
mkdir -p out/
rm -rf out/*.hex
rm -rf out/*.elf

# Download the linker file if it is not already present
if [ ! -f out/linker.ld ]:
then
    curl https://raw.githubusercontent.com/SharpCoder/teensycore/main/src/linker.ld > out/linker.ld
fi

# Build with cargo
RUSTFLAGS="-C panic=abort -C opt-level=2 -C no-redzone" cargo build --target thumbv7em-none-eabihf

# Extract all projects in the workspace
# and then build them into individual hex files
DIR=$(cargo metadata | jq '.target_directory' | tr -d '"')/thumbv7em-none-eabihf/debug
arr=$(ls $DIR/*.a)
for elf in "${arr}"
do :

    # Link the built file
    arm-none-eabi-ld \
        -T out/linker.ld \
        -strip-all \
        --gc-sections \
        $elf \
        -o out/kernel.elf

    # Extract the name, so the hex can have a pleasant file name
    lib=$(basename $elf .a)
    proj=$(echo $lib | sed 's/lib//g')

    # Use objcopy to generate the hex output
    arm-none-eabi-objcopy -O ihex out/kernel.elf out/$proj.hex
done

# Remove artifacts
rm -rf out/kernel.elf
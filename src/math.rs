use crate::system::str::*;

// Buffer used for doing general math stuff
static mut U64_BUF: [u8; 20] = [0; 20];

pub fn itoa(number: u64) -> Str {
    return itob(number, 10);
}

/// Integer to Base
pub fn itob(number: u64, radix: u64) -> Str {
    let mut temp = number;
    let mut size = 0;

    loop {
        let element = temp % radix;
        temp /= radix;
        unsafe { U64_BUF[size] = int_to_hex(element as u8); }
        size += 1;
        if temp == 0 {
            break;
        }

    }

    // Reverse
    let mut tail = size - 1;
    for idx in 0 .. size / 2 {        
        unsafe {
            let temp = U64_BUF[idx];
            U64_BUF[idx] = U64_BUF[tail];
            U64_BUF[tail] = temp;
            tail -= 1;
        }
    }

    return Str::with_content(unsafe { &U64_BUF[0..size] });
}

pub fn pow(base: u64, power: u64) -> u64 {
    if power == 0 {
        return 1;
    } else if power == 1{
        return base;
    }

    let mut result = 1;
    for _ in 0 .. power {
        result *= base;
    }

    return result;
}

/// Ascii to integer
pub fn atoi(input: Str) -> u64 {
    if input.len() == 0 {
        return 0;
    }

    let mut result: u64 = 0;
    let mut digits: u64 = 0;
    
    // Copy input into buffer
    let size = input.len();
    let mut idx = 0;
    let tail =  size - 1;
    for char in input {
        unsafe {
            U64_BUF[tail - idx] = char;
        }
        idx += 1;
    }
    
    for character in unsafe { &U64_BUF[0..size] } {
        if *character >= 48 && *character <= 57 {
            result += char_to_int(*character) as u64 * pow(10, digits);
        } else {
            continue;
        }

        digits += 1;
    }

    return result;
}

pub fn char_to_int(char: u8) -> u8 {
    return char - 48;
}

// Technically this supports up-to base 26 :P
pub fn int_to_hex(number: u8) -> u8 {
    if number < 10 {
        return number + 48;
    } else {
        return number - 10 + b'A';
    }
}

/// Return the minimum of two comparable items.
pub fn min<T: PartialOrd> (left: T, right: T) -> T {
    if left > right {
        return right;
    } else {
        return left;
    }
}

// Amazing prng XORSHIFT+
// https://en.wikipedia.org/wiki/Xorshift
// 128 bits is kinda overkill though.
static mut XORSHIFT_REGS: [u64;2] = [0xFAE0, 0xFFAA_FFDC];
pub fn rand() -> u64 {
    unsafe {
        let mut t = XORSHIFT_REGS[0];
        let s = XORSHIFT_REGS[1];
        XORSHIFT_REGS[0] = s;
        t ^= t << 23;
        t ^= t >> 18;
        t ^= s ^ (s >> 5);
        XORSHIFT_REGS[1] = t;
        return t + s;
    }
}

pub fn seed_rand(val: u64) {
    unsafe {
        XORSHIFT_REGS[0] = val;
    }
}

#[cfg(test)]
mod test {
    use crate::*;
    use super::*;

    fn sb_eq(left: Str, right: Str) {
        assert_eq!(left.len(), right.len());
        for idx in 0 .. left.len() {
            assert_eq!(left.char_at(idx), right.char_at(idx));
        }
    }

    #[test]
    fn test_itoa() {
        assert_eq!(int_to_hex(0), b'0');
        assert_eq!(int_to_hex(5), b'5');
        assert_eq!(int_to_hex(8), b'8');
        assert_eq!(int_to_hex(9), b'9');

        sb_eq(itoa(10345612345612345), str!(b"10345612345612345"));
        sb_eq(itoa(19), str!(b"19"));
        sb_eq(itoa(180), str!(b"180"));
        sb_eq(itoa(1), str!(b"1"));
        sb_eq(itoa(10), str!(b"10"));
        sb_eq(itoa(101), str!(b"101"));
        sb_eq(itoa(1010), str!(b"1010"));
        sb_eq(itoa(10000), str!(b"10000"));
        sb_eq(itoa(3000002), str!(b"3000002"));
        sb_eq(itoa(1028191), str!(b"1028191"));
        sb_eq(itoa(1220221), str!(b"1220221"));
        sb_eq(itoa(1234567890), str!(b"1234567890"));
        sb_eq(itoa(123456789), str!(b"123456789"));
        sb_eq(itoa(12345678), str!(b"12345678"));
        sb_eq(itoa(1234567), str!(b"1234567"));
        sb_eq(itoa(17), str!(b"17"));
        sb_eq(itoa(137), str!(b"137"));
        sb_eq(itoa(1337), str!(b"1337"));
    }

    #[test]
    fn test_atoi() {
        assert_eq!(char_to_int(b'4'), 4);
        assert_eq!(char_to_int(b'7'), 7);
        assert_eq!(char_to_int(b'0'), 0);
        assert_eq!(char_to_int(b'9'), 9);

        assert_eq!(atoi(str!(b"")), 0);
        assert_eq!(atoi(str!(b"45632190")), 45632190);
        assert_eq!(atoi(str!(b"1")), 1);
        assert_eq!(atoi(str!(b"12")), 12);
        assert_eq!(atoi(str!(b"103")), 103);
        assert_eq!(atoi(str!(b"     1990\n")), 1990);
    }
}
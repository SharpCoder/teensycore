/*
Author: fulmicoton
Original soruce: https://github.com/fulmicoton/fastdivide

zlib License
------------

This software is provided 'as-is', without any express or implied
warranty.  In no event will the authors be held liable for any damages
arising from the use of this software.

Permission is granted to anyone to use this software for any purpose,
including commercial applications, and to alter it and redistribute it
freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not
    claim that you wrote the original software. If you use this software
    in a product, an acknowledgment in the product documentation would be
    appreciated but is not required.
2. Altered source versions must be plainly marked as such, and must not be
    misrepresented as being the original software.
3. This notice may not be removed or altered from any source distribution.
*/

// ported from  libdivide.h by ridiculous_fish
//
//  This file is not the original library, it is an attempt to port part
//  of it to rust.
//
const LIBDIVIDE_ADD_MARKER: u8 = 0x40;
const LIBDIVIDE_U64_SHIFT_PATH: u8 = 0x80;
const LIBDIVIDE_64_SHIFT_MASK: u8 = 0x3F;

#[derive(Debug, Clone, Copy)]
pub struct DividerU64 {
    magic: u64,
    more: u8,
}

fn libdivide_mullhi_u64(x: u64, y: u64) -> u64 {
    let xl = x as u128;
    let yl = y as u128;
    ((xl * yl) >> 64) as u64
}

impl DividerU64 {
    pub fn divide_by(divisor: u64) -> DividerU64 {
        assert!(divisor > 0u64);
        let floor_log_2_d: u8 = 63u8 - (divisor.leading_zeros() as u8);
        if divisor & (divisor - 1) == 0 {
            DividerU64 {
                magic: 0u64,
                more: floor_log_2_d | LIBDIVIDE_U64_SHIFT_PATH,
            }
        } else {
            let u = 1u128 << (floor_log_2_d + 64);
            let mut proposed_m: u128 = u / divisor as u128;
            let reminder: u64 = (u - proposed_m * divisor as u128) as u64;
            assert!(reminder > 0 && reminder < divisor);
            let e: u64 = divisor - reminder;
            let more: u8 = if e < (1u64 << floor_log_2_d) {
                floor_log_2_d
            } else {
                proposed_m += proposed_m;
                let twice_rem = reminder * 2;
                if twice_rem >= divisor || twice_rem < reminder {
                    proposed_m += 1;
                }
                floor_log_2_d | LIBDIVIDE_ADD_MARKER
            };
            DividerU64 {
                more: more,
                magic: (proposed_m as u64) + 1u64,
            }
        }
    }

    #[allow(unknown_lints, inline_always)]
    #[inline(always)]
    pub fn divide(&self, n: u64) -> u64 {
        if self.more & LIBDIVIDE_U64_SHIFT_PATH != 0 {
            n >> (self.more & LIBDIVIDE_64_SHIFT_MASK)
        } else {
            let q = libdivide_mullhi_u64(self.magic, n);
            if self.more & LIBDIVIDE_ADD_MARKER != 0 {
                let t = ((n - q) >> 1).wrapping_add(q);
                t >> (self.more & LIBDIVIDE_64_SHIFT_MASK)
            } else {
                q >> self.more
            }
        }
    }
}

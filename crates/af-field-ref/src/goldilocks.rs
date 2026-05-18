// SPDX-License-Identifier: AGPL-3.0-or-later
//! Golden field arithmetic helpers.

pub const MODULUS: u64 = 0xFFFF_FFFF_0000_0001;

pub fn add_mod(a: u64, b: u64, modulus: u64) -> u64 {
    let sum = a as u128 + b as u128;
    let modv = modulus as u128;
    (sum % modv) as u64
}

pub fn sub_mod(a: u64, b: u64, modulus: u64) -> u64 {
    if a >= b {
        a - b
    } else {
        modulus - (b - a)
    }
}

pub fn mul_mod(a: u64, b: u64, modulus: u64) -> u64 {
    let prod = (a as u128) * (b as u128);
    (prod % (modulus as u128)) as u64
}

pub fn reduce(value: u128, modulus: u64) -> u64 {
    (value % (modulus as u128)) as u64
}

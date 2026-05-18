// SPDX-License-Identifier: AGPL-3.0-or-later
//! Goldilocks finite field reference implementation for af-mod-add.

pub mod goldilocks;

pub use goldilocks::{add_mod, mul_mod, reduce, sub_mod};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_mod_examples() {
        let m = goldilocks::MODULUS;
        assert_eq!(add_mod(0, 0, m), 0);
        assert_eq!(add_mod(1, 1, m), 2);
        assert_eq!(add_mod(m - 1, 1, m), 0);
        assert_eq!(add_mod(m - 1, m - 1, m), m - 2);
    }

    #[test]
    fn test_sub_mod() {
        let m = goldilocks::MODULUS;
        assert_eq!(sub_mod(0, 0, m), 0);
        assert_eq!(sub_mod(1, 2, m), m - 1);
        assert_eq!(sub_mod(m - 1, m - 1, m), 0);
    }

    #[test]
    fn test_reduce() {
        let m = goldilocks::MODULUS;
        assert_eq!(
            reduce((m as u128) * 3, m),
            ((3u128 * (m as u128)) % (m as u128)) as u64
        );
        assert_eq!(reduce((m as u128) * 3 + 7, m), 7);
    }
}

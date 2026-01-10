//! RX-LX Seed Schedule
//! 
//! Implements Monero-style seed rotation for RandomX-LUMENYX.
//! Seed changes every N blocks with D blocks delay for security.

use sp_core::H256;

/// Seed rotation period in blocks (~3 days @ 2.5s/block).
pub const SEED_PERIOD_N: u64 = 103_680;

/// Delay between key block and seed activation (~2 hours @ 2.5s/block).
pub const SEED_DELAY_D: u64 = 2_880;

/// Returns `true` if `height` is a height where the *active seed* changes.
///
/// This matches the RandomX recommended pattern:
/// change key when `blockHeight % N == D`.
///
/// Notes:
/// - At `height = D`, this returns true (first "change event") but the key block is still 0,
///   so the seed value may remain the genesis hash until `height = N + D`.
pub fn is_seed_change_height(height: u64) -> bool {
    height % SEED_PERIOD_N == SEED_DELAY_D
}

/// Returns the height of the key block whose hash is used as the seed for `height`.
///
/// Rules:
/// - If `height < D`, use genesis (0).
/// - Otherwise, compute `h = height - D` and snap down to the nearest multiple of `N`:
///   `key = h - (h % N)`.
///
/// This guarantees `key <= height` (seed is always derived from already-known chain data),
/// and `key % N == 0` (key block is aligned to epoch boundary).
pub fn seed_height(height: u64) -> u64 {
    if height < SEED_DELAY_D {
        return 0;
    }

    let h = height - SEED_DELAY_D;
    h - (h % SEED_PERIOD_N)
}

/// Returns how many blocks remain until the next seed change height.
///
/// Examples:
/// - If `height` is itself a change height, returns 0.
/// - Otherwise returns `next_change_height - height`.
pub fn blocks_until_seed_change(height: u64) -> u64 {
    let r = height % SEED_PERIOD_N;
    if r <= SEED_DELAY_D {
        SEED_DELAY_D - r
    } else {
        (SEED_PERIOD_N - r) + SEED_DELAY_D
    }
}

/// Returns the next height where seed will change.
pub fn next_seed_change_height(height: u64) -> u64 {
    height + blocks_until_seed_change(height)
}

/// Returns the RX-LX seed for a given `height`.
///
/// `get_block_hash` must return the canonical block hash at a given height.
/// The seed is defined as the hash of the key block at `seed_height(height)`.
pub fn get_seed<F>(height: u64, get_block_hash: F) -> H256
where
    F: Fn(u64) -> H256,
{
    let key_h = seed_height(height);
    get_block_hash(key_h)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_hash(h: u64) -> H256 {
        let mut b = [0u8; 32];
        b[24..].copy_from_slice(&h.to_be_bytes());
        H256::from(b)
    }

    #[test]
    fn test_seed_height_small_heights() {
        assert_eq!(seed_height(0), 0);
        assert_eq!(seed_height(1), 0);
        assert_eq!(seed_height(100), 0);
    }

    #[test]
    fn test_seed_height_around_delay() {
        let d = SEED_DELAY_D;
        assert_eq!(seed_height(d - 1), 0);
        assert_eq!(seed_height(d), 0);
        assert_eq!(seed_height(d + 1), 0);
    }

    #[test]
    fn test_seed_height_around_first_real_rotation() {
        let n = SEED_PERIOD_N;
        let d = SEED_DELAY_D;
        assert_eq!(seed_height(n + d - 1), 0);
        assert_eq!(seed_height(n + d), n);
        assert_eq!(seed_height(n + d + 1), n);
    }

    #[test]
    fn test_seed_height_second_rotation() {
        let n = SEED_PERIOD_N;
        let d = SEED_DELAY_D;
        assert_eq!(seed_height(2 * n + d), 2 * n);
    }

    #[test]
    fn test_is_seed_change_height() {
        let n = SEED_PERIOD_N;
        let d = SEED_DELAY_D;
        assert!(!is_seed_change_height(0));
        assert!(!is_seed_change_height(d - 1));
        assert!(is_seed_change_height(d));
        assert!(!is_seed_change_height(d + 1));
        assert!(is_seed_change_height(n + d));
        assert!(is_seed_change_height(2 * n + d));
    }

    #[test]
    fn test_blocks_until_seed_change() {
        let n = SEED_PERIOD_N;
        let d = SEED_DELAY_D;
        assert_eq!(blocks_until_seed_change(d), 0);
        assert_eq!(blocks_until_seed_change(n + d), 0);
        assert_eq!(blocks_until_seed_change(d - 1), 1);
        assert_eq!(blocks_until_seed_change(0), d);
    }

    #[test]
    fn test_get_seed_matches_key_block_hash() {
        let n = SEED_PERIOD_N;
        let d = SEED_DELAY_D;
        assert_eq!(get_seed(0, fake_hash), fake_hash(0));
        assert_eq!(get_seed(d - 1, fake_hash), fake_hash(0));
        assert_eq!(get_seed(d, fake_hash), fake_hash(0));
        assert_eq!(get_seed(n + d, fake_hash), fake_hash(n));
        assert_eq!(get_seed(2 * n + d, fake_hash), fake_hash(2 * n));
    }
}

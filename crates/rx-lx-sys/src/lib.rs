//! RX-LX-SYS: FFI bindings for RandomX-LUMENYX
//!
//! Low-level unsafe bindings to the RandomX C API.
//! This crate provides the foundation for RX-LX PoW algorithm.

use libc::{c_void, size_t, c_ulong};

/// Opaque type for RandomX cache
#[repr(C)]
pub struct randomx_cache {
    _private: [u8; 0],
}

/// Opaque type for RandomX dataset
#[repr(C)]
pub struct randomx_dataset {
    _private: [u8; 0],
}

/// Opaque type for RandomX virtual machine
#[repr(C)]
pub struct randomx_vm {
    _private: [u8; 0],
}

/// RandomX flags for configuration
pub type randomx_flags = u32;

// Flag constants from randomx.h
pub const RANDOMX_FLAG_DEFAULT: randomx_flags = 0;
pub const RANDOMX_FLAG_LARGE_PAGES: randomx_flags = 1;
pub const RANDOMX_FLAG_HARD_AES: randomx_flags = 2;
pub const RANDOMX_FLAG_FULL_MEM: randomx_flags = 4;
pub const RANDOMX_FLAG_JIT: randomx_flags = 8;
pub const RANDOMX_FLAG_SECURE: randomx_flags = 16;
pub const RANDOMX_FLAG_ARGON2_SSSE3: randomx_flags = 32;
pub const RANDOMX_FLAG_ARGON2_AVX2: randomx_flags = 64;
pub const RANDOMX_FLAG_ARGON2: randomx_flags = 96;

extern "C" {
    /// Get recommended flags for the current CPU
    pub fn randomx_get_flags() -> randomx_flags;

    /// Allocate cache memory
    pub fn randomx_alloc_cache(flags: randomx_flags) -> *mut randomx_cache;

    /// Initialize cache with a key
    pub fn randomx_init_cache(
        cache: *mut randomx_cache,
        key: *const c_void,
        key_size: size_t,
    );

    /// Release cache memory
    pub fn randomx_release_cache(cache: *mut randomx_cache);

    /// Allocate dataset memory
    pub fn randomx_alloc_dataset(flags: randomx_flags) -> *mut randomx_dataset;

    /// Get number of items in dataset
    pub fn randomx_dataset_item_count() -> c_ulong;

    /// Initialize dataset from cache
    pub fn randomx_init_dataset(
        dataset: *mut randomx_dataset,
        cache: *mut randomx_cache,
        start_item: c_ulong,
        item_count: c_ulong,
    );

    /// Release dataset memory
    pub fn randomx_release_dataset(dataset: *mut randomx_dataset);

    /// Create a virtual machine
    pub fn randomx_create_vm(
        flags: randomx_flags,
        cache: *mut randomx_cache,
        dataset: *mut randomx_dataset,
    ) -> *mut randomx_vm;

    /// Set VM cache (for light mode)
    pub fn randomx_vm_set_cache(
        machine: *mut randomx_vm,
        cache: *mut randomx_cache,
    );

    /// Set VM dataset (for fast mode)
    pub fn randomx_vm_set_dataset(
        machine: *mut randomx_vm,
        dataset: *mut randomx_dataset,
    );

    /// Destroy a virtual machine
    pub fn randomx_destroy_vm(machine: *mut randomx_vm);

    /// Calculate hash (single input)
    pub fn randomx_calculate_hash(
        machine: *mut randomx_vm,
        input: *const c_void,
        input_size: size_t,
        output: *mut c_void,
    );

    /// Calculate hash (first in batch)
    pub fn randomx_calculate_hash_first(
        machine: *mut randomx_vm,
        input: *const c_void,
        input_size: size_t,
    );

    /// Calculate hash (next in batch)
    pub fn randomx_calculate_hash_next(
        machine: *mut randomx_vm,
        next_input: *const c_void,
        next_input_size: size_t,
        output: *mut c_void,
    );

    /// Calculate hash (last in batch)
    pub fn randomx_calculate_hash_last(
        machine: *mut randomx_vm,
        output: *mut c_void,
    );
}

/// Hash output size in bytes (256 bits)
pub const RANDOMX_HASH_SIZE: usize = 32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_flags() {
        unsafe {
            let flags = randomx_get_flags();
            // Should return some valid flags
            println!("RandomX flags: {}", flags);
            assert!(flags >= 0);
        }
    }
}

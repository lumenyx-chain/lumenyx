//! RX-LX: Safe Rust wrapper for RandomX-LUMENYX
//!
//! This crate provides a safe, ergonomic API for the RX-LX PoW algorithm.
//! It wraps the unsafe FFI bindings with proper RAII semantics.

use rx_lx_sys::*;
use std::ptr;

/// Error types for RX-LX operations
#[derive(Debug, thiserror::Error)]
pub enum RxLxError {
    #[error("Failed to allocate cache")]
    CacheAllocationFailed,
    #[error("Failed to allocate dataset")]
    DatasetAllocationFailed,
    #[error("Failed to create VM")]
    VmCreationFailed,
    #[error("Cache not initialized")]
    CacheNotInitialized,
}

pub type Result<T> = std::result::Result<T, RxLxError>;

/// RandomX flags wrapper
#[derive(Debug, Clone, Copy)]
pub struct Flags(randomx_flags);

impl Flags {
    /// Get recommended flags for current CPU
    pub fn recommended() -> Self {
        // RX-LX: Force soft AES to use custom SBOX
        unsafe { Flags(randomx_get_flags() & !RANDOMX_FLAG_HARD_AES) }
    }

    /// Default flags (no optimizations)
    pub fn default_flags() -> Self {
        Flags(RANDOMX_FLAG_DEFAULT)
    }

    /// Add JIT compilation support
    pub fn with_jit(self) -> Self {
        Flags(self.0 | RANDOMX_FLAG_JIT)
    }

    /// Add hardware AES support
    // RX-LX: Remove hardware AES to force custom SBOX
    pub fn without_hard_aes(self) -> Self {
        Flags(self.0 & !RANDOMX_FLAG_HARD_AES)
    }

    /// Add hardware AES support (NOT recommended for RX-LX - breaks custom SBOX)
    pub fn with_hard_aes(self) -> Self {
        Flags(self.0 | RANDOMX_FLAG_HARD_AES)
    }

    /// Add full memory (dataset) mode
    pub fn with_full_mem(self) -> Self {
        Flags(self.0 | RANDOMX_FLAG_FULL_MEM)
    }

    /// Add large pages support
    pub fn with_large_pages(self) -> Self {
        Flags(self.0 | RANDOMX_FLAG_LARGE_PAGES)
    }

    /// Get raw flags value
    pub fn raw(&self) -> randomx_flags {
        self.0
    }
}

/// RandomX Cache (for light mode verification)
pub struct Cache {
    ptr: *mut randomx_cache,
    flags: Flags,
}

unsafe impl Send for Cache {}

impl Cache {
    /// Allocate a new cache
    pub fn alloc(flags: Flags) -> Result<Self> {
        let ptr = unsafe { randomx_alloc_cache(flags.raw()) };
        if ptr.is_null() {
            return Err(RxLxError::CacheAllocationFailed);
        }
        Ok(Cache { ptr, flags })
    }

    /// Initialize cache with a key (seed)
    pub fn init(&mut self, key: &[u8]) {
        unsafe {
            randomx_init_cache(
                self.ptr,
                key.as_ptr() as *const _,
                key.len(),
            );
        }
    }

    /// Get raw pointer (for internal use)
    pub(crate) fn as_ptr(&self) -> *mut randomx_cache {
        self.ptr
    }

    /// Get flags used for this cache
    pub fn flags(&self) -> Flags {
        self.flags
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { randomx_release_cache(self.ptr) };
        }
    }
}

/// RandomX Dataset (for fast mode mining)
pub struct Dataset {
    ptr: *mut randomx_dataset,
    flags: Flags,
}

unsafe impl Send for Dataset {}

impl Dataset {
    /// Allocate a new dataset
    pub fn alloc(flags: Flags) -> Result<Self> {
        let ptr = unsafe { randomx_alloc_dataset(flags.raw()) };
        if ptr.is_null() {
            return Err(RxLxError::DatasetAllocationFailed);
        }
        Ok(Dataset { ptr, flags })
    }

    /// Get the number of items in the dataset
    pub fn item_count() -> u64 {
        unsafe { randomx_dataset_item_count() as u64 }
    }

    /// Initialize dataset from cache (single-threaded)
    pub fn init(&mut self, cache: &Cache) {
        let count = Self::item_count();
        unsafe {
            randomx_init_dataset(self.ptr, cache.as_ptr(), 0, count as _);
        }
    }

    /// Initialize dataset from cache (multi-threaded, specific range)
    pub fn init_range(&mut self, cache: &Cache, start: u64, count: u64) {
        unsafe {
            randomx_init_dataset(self.ptr, cache.as_ptr(), start as _, count as _);
        }
    }

    /// Get raw pointer (for internal use)
    pub(crate) fn as_ptr(&self) -> *mut randomx_dataset {
        self.ptr
    }

    /// Get flags used for this dataset
    pub fn flags(&self) -> Flags {
        self.flags
    }
}

impl Drop for Dataset {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { randomx_release_dataset(self.ptr) };
        }
    }
}

/// RandomX Virtual Machine
pub struct Vm {
    ptr: *mut randomx_vm,
}

unsafe impl Send for Vm {}

impl Vm {
    /// Create a VM for light mode (verification) - uses cache only
    pub fn light(flags: Flags, cache: &Cache) -> Result<Self> {
        let ptr = unsafe {
            randomx_create_vm(flags.raw(), cache.as_ptr(), ptr::null_mut())
        };
        if ptr.is_null() {
            return Err(RxLxError::VmCreationFailed);
        }
        Ok(Vm { ptr })
    }

    /// Create a VM for fast mode (mining) - uses dataset
    pub fn fast(flags: Flags, dataset: &Dataset) -> Result<Self> {
        let ptr = unsafe {
            randomx_create_vm(
                flags.raw() | RANDOMX_FLAG_FULL_MEM,
                ptr::null_mut(),
                dataset.as_ptr(),
            )
        };
        if ptr.is_null() {
            return Err(RxLxError::VmCreationFailed);
        }
        Ok(Vm { ptr })
    }

    /// Calculate hash of input data
    pub fn hash(&self, input: &[u8]) -> [u8; 32] {
        let mut output = [0u8; 32];
        unsafe {
            randomx_calculate_hash(
                self.ptr,
                input.as_ptr() as *const _,
                input.len(),
                output.as_mut_ptr() as *mut _,
            );
        }
        output
    }
}

impl Drop for Vm {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { randomx_destroy_vm(self.ptr) };
        }
    }
}

/// High-level hasher for light mode (verification)
pub struct LightHasher {
    cache: Cache,
    vm: Vm,
}

impl LightHasher {
    /// Create a new light hasher with the given seed
    pub fn new(seed: &[u8]) -> Result<Self> {
        let flags = Flags::recommended();
        let mut cache = Cache::alloc(flags)?;
        cache.init(seed);
        let vm = Vm::light(flags, &cache)?;
        Ok(LightHasher { cache, vm })
    }

    /// Calculate hash
    pub fn hash(&self, input: &[u8]) -> [u8; 32] {
        self.vm.hash(input)
    }

    /// Reinitialize with new seed
    pub fn set_seed(&mut self, seed: &[u8]) {
        self.cache.init(seed);
    }
}

/// High-level hasher for fast mode (mining)
pub struct FastHasher {
    #[allow(dead_code)]
    cache: Cache,
    #[allow(dead_code)]
    dataset: Dataset,
    vm: Vm,
}

impl FastHasher {
    /// Create a new fast hasher with the given seed
    pub fn new(seed: &[u8]) -> Result<Self> {
        let flags = Flags::recommended();
        let mut cache = Cache::alloc(flags)?;
        cache.init(seed);
        let mut dataset = Dataset::alloc(flags)?;
        dataset.init(&cache);
        let vm = Vm::fast(flags, &dataset)?;
        Ok(FastHasher { cache, dataset, vm })
    }

    /// Calculate hash
    pub fn hash(&self, input: &[u8]) -> [u8; 32] {
        self.vm.hash(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_recommended() {
        let flags = Flags::recommended();
        println!("Recommended flags: {}", flags.raw());
    }

    #[test]
    fn test_cache_alloc() {
        let flags = Flags::recommended();
        let cache = Cache::alloc(flags);
        assert!(cache.is_ok());
    }

    #[test]
    fn test_cache_init() {
        let flags = Flags::recommended();
        let mut cache = Cache::alloc(flags).unwrap();
        let seed = b"test seed for randomx";
        cache.init(seed);
    }

    #[test]
    fn test_light_hasher() {
        let seed = b"LUMENYX genesis seed";
        let hasher = LightHasher::new(seed).unwrap();
        let input = b"Hello, LUMENYX!";
        let hash = hasher.hash(input);
        println!("Hash: {}", hex::encode(&hash));
        let hash2 = hasher.hash(input);
        assert_eq!(hash, hash2);
        let hash3 = hasher.hash(b"Different input");
        assert_ne!(hash, hash3);
    }
}

#[cfg(test)]
mod golden_tests {
    use super::*;

    /// Test vector from RandomX reference implementation (api-example1.c)
    #[test]
    fn test_golden_vector_reference() {
        let key = b"RandomX example key\0";
        let input = b"RandomX example input\0";
        let expected = "7e030b0a83df80ca090d7e79f433989759fc241f4b7fc9aa07695226e96e53d0";

        let hasher = LightHasher::new(key).unwrap();
        let hash = hasher.hash(input);
        let hash_hex = hex::encode(&hash);

        println!("Expected: {}", expected);
        println!("Got:      {}", hash_hex);

        assert_eq!(hash_hex, expected, "Hash mismatch with RandomX reference!");
    }

    /// Test determinism
    #[test]
    fn test_determinism() {
        let seed = b"LUMENYX RX-LX seed v1";
        let input = b"block header data here";

        let hasher1 = LightHasher::new(seed).unwrap();
        let hasher2 = LightHasher::new(seed).unwrap();

        let hash1 = hasher1.hash(input);
        let hash2 = hasher2.hash(input);

        assert_eq!(hash1, hash2, "RandomX must be deterministic!");
    }

    /// Test different seeds produce different hashes
    #[test]
    fn test_different_seeds() {
        let input = b"same input";

        let hasher1 = LightHasher::new(b"seed one").unwrap();
        let hasher2 = LightHasher::new(b"seed two").unwrap();

        let hash1 = hasher1.hash(input);
        let hash2 = hasher2.hash(input);

        assert_ne!(hash1, hash2, "Different seeds must produce different hashes!");
    }
}

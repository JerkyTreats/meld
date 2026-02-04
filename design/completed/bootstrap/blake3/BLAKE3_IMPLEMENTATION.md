# BLAKE3 Implementation Guide

## Overview

This document outlines how we safely and properly use BLAKE3 in the Merkle filesystem state management system. We use the official `blake3` crate (version 1.5+), which is the reference Rust implementation maintained by the BLAKE3 team.

## Why We Use the Official Implementation

1. **Security**: The official implementation is cryptographically secure and well-tested
2. **Performance**: Optimized with SIMD and parallel processing
3. **Maintenance**: Actively maintained by the BLAKE3 team
4. **Correctness**: Verified against official test vectors
5. **No Reimplementation Risk**: Avoids bugs and security vulnerabilities from naive implementations

## Implementation Strategy

### ✅ What We Do

1. **Use the Official Crate**: We use `blake3 = "1.5"` from crates.io
2. **Standard API**: We use `blake3::Hasher` with standard `update()` and `finalize()` methods
3. **No Custom Crypto**: We never implement cryptographic primitives ourselves
4. **Test Against Vectors**: We verify our usage against official test vectors
5. **Deterministic Usage**: We ensure our hashing patterns are deterministic

### ❌ What We Don't Do

1. **No Custom BLAKE3 Implementation**: We never reimplement BLAKE3
2. **No Modified Versions**: We use the standard crate, not forks
3. **No Cryptographic Shortcuts**: We follow standard cryptographic practices
4. **No Undocumented Features**: We only use documented, stable APIs

## Best Practices

### 1. Standard Hasher Usage

```rust
use blake3::Hasher;

// Create hasher
let mut hasher = Hasher::new();

// Update with data (can be called multiple times)
hasher.update(data1);
hasher.update(data2);

// Finalize to get 32-byte hash
let hash = hasher.finalize();
let hash_bytes: [u8; 32] = *hash.as_bytes();
```

### 2. Deterministic Hashing Patterns

For our use case, we need deterministic hashing. We ensure this by:

- **Fixed Order**: Always hash components in the same order
- **Canonical Representation**: Normalize paths and data before hashing
- **Big-Endian Encoding**: Use big-endian for numeric values (cross-platform consistency)
- **Sorted Collections**: Sort metadata and children before hashing

### 3. Content Hashing

For file content, we use a simple pattern:

```rust
pub fn compute_content_hash(content: &[u8]) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(content);
    *hasher.finalize().as_bytes()
}
```

This is safe because:
- BLAKE3 handles arbitrary-length inputs correctly
- No need for chunking (BLAKE3 handles it internally)
- Deterministic for same input

### 4. Structured Data Hashing

For structured data (NodeIDs, FrameIDs), we use a structured approach:

```rust
let mut hasher = Hasher::new();

// Type discriminator (prevents collisions)
hasher.update(b"file");

// Length prefix (big-endian for determinism)
hasher.update(&(data.len() as u64).to_be_bytes());

// Actual data
hasher.update(data);

// Additional fields
hasher.update(additional_data);

let hash = *hasher.finalize().as_bytes();
```

### 5. Incremental Hashing

BLAKE3 supports incremental hashing (multiple `update()` calls):

```rust
let mut hasher = Hasher::new();
hasher.update(part1);
hasher.update(part2);
hasher.update(part3);
let hash = hasher.finalize();
```

This is equivalent to:
```rust
let mut hasher = Hasher::new();
hasher.update(&[part1, part2, part3].concat());
let hash = hasher.finalize();
```

## Verification and Testing

### 1. Official Test Vectors

We verify our implementation against official BLAKE3 test vectors:

- Empty input: `af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262`
- Standard test vectors from BLAKE3 specification

See `tests/integration/blake3_verification.rs` for test vector tests.

### 2. Determinism Tests

We verify that:
- Same input → same output (always)
- Different input → different output (with cryptographic probability)
- Incremental hashing produces same result as single update

### 3. Edge Case Tests

We test:
- Empty input
- Single byte input
- Large inputs (1MB+)
- Unicode content
- Special characters

### 4. Integration Tests

We verify that our wrapper functions (`compute_content_hash`, `compute_file_node_id`, etc.) correctly use BLAKE3 and maintain determinism.

## Security Considerations

### 1. No Timing Attacks

BLAKE3 is designed to be constant-time, but we ensure:
- No data-dependent branching in our code
- No secret-dependent memory access patterns
- Standard usage patterns (no custom optimizations)

### 2. No Information Leakage

- Hash outputs are 32 bytes (256 bits) - sufficient for collision resistance
- No truncation of hash outputs
- No reuse of hasher state between unrelated operations

### 3. Proper Initialization

- Always create new `Hasher` instances for each hash computation
- Never reuse hasher state (each hash gets fresh hasher)
- Clear hasher state after use (Rust's ownership ensures this)

## Performance Considerations

### 1. BLAKE3 is Fast

- SIMD-optimized by default
- Parallel processing for large inputs
- Faster than SHA-256 in most cases

### 2. Our Usage Patterns

- Small inputs (< 1KB): Very fast (< 1µs)
- Medium inputs (1KB - 1MB): Fast (< 1ms)
- Large inputs (> 1MB): Efficient with parallel processing

### 3. No Premature Optimization

- We use standard BLAKE3 API
- No custom chunking or batching
- Let BLAKE3 handle optimizations internally

## Cross-Platform Consistency

### 1. Determinism Across Platforms

BLAKE3 produces the same output for the same input across:
- Different operating systems (Linux, macOS, Windows)
- Different architectures (x86, ARM, etc.)
- Different Rust versions (as long as blake3 crate version is same)

### 2. Our Additional Guarantees

- Big-endian encoding for numeric values (platform-independent)
- Unicode normalization (NFC) for text
- Path canonicalization for filesystem paths

## Version Management

### 1. Crate Version

We use `blake3 = "1.5"` (or compatible version). This ensures:
- Stable API
- Security updates
- Performance improvements

### 2. Version Pinning

For production, consider:
- Pinning exact version: `blake3 = "=1.5.0"`
- Or using version range: `blake3 = "1.5"` (allows patch updates)

### 3. Updating

When updating the blake3 crate:
- Review changelog for breaking changes
- Re-run all test vectors
- Verify determinism tests still pass
- Check performance benchmarks

## References

- [BLAKE3 Specification](https://github.com/BLAKE3-team/BLAKE3-specs)
- [BLAKE3 Rust Crate](https://crates.io/crates/blake3)
- [BLAKE3 Official Repository](https://github.com/BLAKE3-team/BLAKE3)
- [BLAKE3 Test Vectors](https://github.com/BLAKE3-team/BLAKE3/blob/main/test_vectors/test_vectors.json)

## Checklist for BLAKE3 Usage

- [x] Use official `blake3` crate
- [x] Use standard `Hasher` API
- [x] Test against official test vectors
- [x] Verify determinism
- [x] Test edge cases
- [x] Document usage patterns
- [x] No custom cryptographic code
- [x] Follow security best practices

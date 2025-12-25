# BLAKE3 Testing Strategy

## Overview

This document describes our comprehensive testing strategy for ensuring we're using BLAKE3 correctly and safely. Our tests verify both the correctness of the official BLAKE3 implementation and our usage patterns.

## Test Categories

### 1. Official Test Vector Verification

**Location**: `tests/integration/blake3_verification.rs`

**Purpose**: Verify that we're using the official BLAKE3 implementation correctly by testing against known-good outputs.

**Test Vectors**:
- Empty input hash (official BLAKE3 hash)
- Standard test inputs (verified for determinism)

**Key Tests**:
- `test_official_blake3_vectors()`: Verifies known test vectors
- `test_empty_input()`: Verifies empty input produces correct hash

### 2. Determinism Verification

**Location**: `tests/integration/blake3_verification.rs`

**Purpose**: Ensure BLAKE3 produces deterministic outputs.

**Key Tests**:
- `test_determinism()`: Same input → same output
- `test_incremental_hashing()`: Incremental updates produce same result
- `test_update_order_independence()`: Same data, different update patterns → same hash

### 3. Avalanche Effect Verification

**Location**: `tests/integration/blake3_verification.rs`

**Purpose**: Verify that small input changes produce completely different hashes.

**Key Tests**:
- `test_avalanche_effect()`: Different inputs → different hashes

### 4. Edge Case Testing

**Location**: `tests/integration/blake3_verification.rs`

**Purpose**: Test behavior with edge cases.

**Key Tests**:
- `test_large_input()`: Large inputs (1MB+) handled correctly
- `test_hash_output_size()`: Output is always 32 bytes

### 5. Our Implementation Verification

**Location**: `tests/integration/hasher_verification.rs`

**Purpose**: Verify that our wrapper functions correctly use BLAKE3.

**Key Tests**:
- `test_content_hash_matches_blake3()`: Our wrapper matches direct BLAKE3
- `test_file_node_id_determinism()`: File NodeID computation is deterministic
- `test_directory_node_id_determinism()`: Directory NodeID computation is deterministic
- `test_file_node_id_content_sensitivity()`: Content changes → NodeID changes
- `test_file_node_id_path_sensitivity()`: Path changes → NodeID changes
- `test_file_node_id_metadata_sensitivity()`: Metadata changes → NodeID changes

### 6. Property-Based Testing

**Location**: `tests/property/determinism.rs`

**Purpose**: Verify determinism properties across wide range of inputs.

**Key Tests**:
- `test_nodeid_determinism_property()`: Property-based NodeID determinism
- `test_frameid_determinism_property()`: Property-based FrameID determinism

## Running the Tests

### Run All BLAKE3 Tests

```bash
cargo test --test blake3_verification
cargo test --test hasher_verification
cargo test --test determinism
```

### Run Specific Test Category

```bash
# Official test vectors
cargo test test_official_blake3_vectors

# Determinism
cargo test test_determinism

# Our implementation
cargo test test_content_hash_matches_blake3
```

### Run with Verbose Output

```bash
cargo test --test blake3_verification -- --nocapture
```

## Test Coverage

### ✅ Covered

- [x] Official test vector verification
- [x] Determinism verification
- [x] Incremental hashing correctness
- [x] Large input handling
- [x] Edge cases (empty, single byte, Unicode)
- [x] Our wrapper function correctness
- [x] NodeID computation correctness
- [x] FrameID computation correctness
- [x] Property-based testing

### 🔄 Continuous Verification

- Run tests in CI/CD pipeline
- Verify against new BLAKE3 crate versions
- Update test vectors if official vectors change

## Expected Test Results

All tests should pass. If any test fails:

1. **Official test vector failure**: May indicate BLAKE3 crate version issue
2. **Determinism failure**: Critical bug - must fix immediately
3. **Implementation verification failure**: Bug in our wrapper functions
4. **Property test failure**: May indicate edge case bug

## Adding New Tests

When adding new BLAKE3 usage:

1. Add test to verify correctness
2. Add test to verify determinism
3. Add test for edge cases
4. Update this document

## References

- [BLAKE3 Test Vectors](https://github.com/BLAKE3-team/BLAKE3/blob/main/test_vectors/test_vectors.json)
- [BLAKE3 Specification](https://github.com/BLAKE3-team/BLAKE3-specs)
- [BLAKE3 Rust Crate](https://crates.io/crates/blake3)

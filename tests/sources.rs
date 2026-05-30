//! Integration tests for the source adapters.
//!
//! Each source is exercised against a `wiremock` server serving canned registry
//! responses — no live network in CI. Filled in alongside M1/M2.

// M1: crates.io parsing via wiremock.
// M2: each remaining source + dedup.

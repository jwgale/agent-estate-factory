//! Reserved one-way feed seam. Append scrubbed traces. Never auto-promote.
//!
//! Day 61–90: traces from frontier + local can be *materialized* into a pack
//! manifest in the enrich-packs drop zone. Jason still has to edit the estate
//! by hand. `promote` always fails.

include!("feed_body_0.rs");
include!("feed_body_1.rs");
include!("feed_body_2.rs");
include!("feed_body_3.rs");

#[cfg(test)]
mod feed_test_support;
#[cfg(test)]
mod feed_tests_0;
#[cfg(test)]
mod feed_tests_1;
#[cfg(test)]
mod feed_tests_2;

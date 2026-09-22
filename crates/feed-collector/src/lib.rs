//! Reserved one-way feed seam. Append scrubbed traces. Never auto-promote.
//!
//! Day 61–90: traces from frontier + local can be *materialized* into a pack
//! manifest in the enrich-packs drop zone. Jason still has to edit the estate
//! by hand. `promote` always fails.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FeedError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(String),
    #[error("no auto-promote: Jason curates enrich packs by hand (policy=manual)")]
    NoAutoPromote,
    #[error("pack id '{0}' encodes a hardware SKU; hardware is a driver choice")]
    SkuBanned(String),
    #[error("pack id '{0}' must match [a-z][a-z0-9_-]{{0,63}}")]
    BadId(String),
    #[error("pack schema must be cell-one.pack.v0 or cell-one.specialist-pack.v0 (got {0})")]
    BadSchema(String),
    #[error("pack host_class '{0}' must be consumer-nvidia|apple-silicon|rented-nvidia|any")]
    BadHostClass(String),
    #[error("refuse:missing-pack: no pack '{0}' in drop or accepted")]
    MissingPack(String),
    #[error("refuse:no-auto-apply: enrich proposals are curator-only; Jason reviews and edits the estate")]
    NoAutoApply,
    #[error("refuse:raw-secret: pack must not store raw secrets")]
    RawSecret,
    #[error("refuse:model-hint: '{0}' must be a slug and must not encode a hardware SKU")]
    BadModelHint(String),
    #[error("refuse:source-path: '{0}' is not a relative path (or encodes a SKU)")]
    BadSourcePath(String),
    #[error("refuse:source-driver: '{0}'")]
    BadSourceDriver(String),
    #[error(
        "refuse:frontier-invent: no frontier binding; will not invent a frontier source_driver"
    )]
    FrontierInvent,
    #[error("refuse:curator: curator '{provided}' does not match locked curator '{want}'")]
    WrongCurator { provided: String, want: String },
}

/// Overnight lock. Jason curates. Do not reopen.
pub const LOCKED_CURATOR: &str = "jason";

pub fn refuse_curator(provided: &str, estate_curator: &str) -> Result<(), FeedError> {
    let have = provided.trim().to_ascii_lowercase();
    let estate = estate_curator.trim().to_ascii_lowercase();
    if have != LOCKED_CURATOR {
        return Err(FeedError::WrongCurator {
            provided: provided.to_string(),
            want: LOCKED_CURATOR.into(),
        });
    }
    if !estate.is_empty() && estate != LOCKED_CURATOR {
        return Err(FeedError::WrongCurator {
            provided: estate_curator.to_string(),
            want: LOCKED_CURATOR.into(),
        });
    }
    Ok(())
}

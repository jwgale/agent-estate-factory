use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

static N: AtomicU64 = AtomicU64::new(0);

pub(crate) fn tmp() -> std::path::PathBuf {
    let n = N.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("cell-one-feed-{n}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

pub(crate) fn bound_estate() -> estate_schema::Estate {
    estate_schema::load_estate_str(include_str!("../../../examples/estate.yaml")).unwrap()
}

pub(crate) struct Boom;
impl Serialize for Boom {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error;
        Err(S::Error::custom("boom"))
    }
}

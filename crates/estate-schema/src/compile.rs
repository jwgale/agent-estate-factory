use crate::hash::estate_hash;
use crate::types::{Effect, Estate, IntentionKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledIntention {
    pub subject_agent: String,
    pub object: String,
    pub kind: IntentionKind,
    pub effect: Effect,
    pub source_estate_hash: String,
}

pub fn compile_intentions(estate: &Estate) -> Vec<CompiledIntention> {
    let hash = estate_hash(estate);
    estate
        .intentions
        .iter()
        .map(|i| CompiledIntention {
            subject_agent: i.subject_agent.clone(),
            object: i.object.clone(),
            kind: i.kind,
            effect: i.effect,
            source_estate_hash: hash.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_estate_str;

    #[test]
    fn empty_intentions_compile_empty() {
        let e = load_estate_str(crate::tests::example_yaml()).unwrap();
        assert!(compile_intentions(&e).is_empty());
    }
}

use crate::sacred::normalize_name;
use crate::types::{Effect, Estate, IntentionKind, ModelClass, ObjectRef};
use std::path::{Component, Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRequest<'a> {
    pub subject_agent: &'a str,
    pub kind: IntentionKind,
    pub object: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow { reason: String },
    Deny(Deny),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deny {
    pub reason: String,
}

impl Decision {
    pub fn is_allow(&self) -> bool {
        matches!(self, Decision::Allow { .. })
    }

    pub fn reason(&self) -> &str {
        match self {
            Decision::Allow { reason } => reason,
            Decision::Deny(d) => &d.reason,
        }
    }
}

/// Fail-closed authorization used by conveyor-proxy and memory-firewall tests.
pub fn authorize(estate: &Estate, req: &AccessRequest<'_>) -> Decision {
    if estate.agent(req.subject_agent).is_none() {
        return deny(format!("unknown subject agent '{}'", req.subject_agent));
    }
    if estate.is_sacred(req.object) {
        return deny(format!(
            "sacred exclusion '{}' is not readable or mountable from the estate",
            req.object
        ));
    }

    // Model, Tool, Mcp, Mount, and Agent decide coverage inside their own path.
    // An early explicit match would hide deny-default when no intention covers
    // the object.
    if !matches!(
        req.kind,
        IntentionKind::Model
            | IntentionKind::Tool
            | IntentionKind::Mcp
            | IntentionKind::Mount
            | IntentionKind::Agent
    ) {
        if let Some(decision) = explicit_intention(estate, req) {
            return decision;
        }
    }

    match req.kind {
        IntentionKind::MemoryRead => authorize_memory(estate, req),
        IntentionKind::Tool | IntentionKind::Mcp | IntentionKind::Mount => {
            authorize_declared_named(estate, req)
        }
        IntentionKind::Model => authorize_model(estate, req),
        IntentionKind::Agent => authorize_agent(estate, req),
    }
}

/// Who may call whom. The object is an estate agent (bare id or `agent:`).
/// The subject must declare that call. Missing coverage is deny-default.
/// An explicit deny wins. Sacred ids are not agents. This is not a gateway.
fn authorize_agent(estate: &Estate, req: &AccessRequest<'_>) -> Decision {
    let Some(agent) = estate.agent(req.subject_agent) else {
        return deny(format!("unknown subject agent '{}'", req.subject_agent));
    };
    let Some(name) = agent_object_name(req.object) else {
        return deny(format!(
            "agent '{}' is not an estate agent (deny-default)",
            req.object
        ));
    };
    if estate.is_sacred(&name) {
        return deny(format!(
            "sacred exclusion '{name}' is not an agent call target"
        ));
    }
    if name.is_empty() || estate.agent(&name).is_none() {
        return deny(format!(
            "agent '{name}' is not an estate agent (deny-default)"
        ));
    }
    if !agent.has_call(&name) {
        return deny(format!(
            "agent '{name}' undeclared for agent '{}' (deny-default)",
            req.subject_agent
        ));
    }
    match named_intention_effect(estate, req.subject_agent, IntentionKind::Agent, &name) {
        Some(Effect::Deny) => deny(format!(
            "explicit deny intention: {} agent {name}",
            req.subject_agent
        )),
        Some(Effect::Allow) => allow(format!(
            "allow intention: {} agent {name}",
            req.subject_agent
        )),
        None => deny(format!(
            "agent '{name}' is not covered by an allow Agent intention for agent '{}' (deny-default)",
            req.subject_agent
        )),
    }
}

fn agent_object_name(object: &str) -> Option<String> {
    match ObjectRef::parse(object) {
        ObjectRef::Agent(name) | ObjectRef::Bare(name) => Some(normalize_name(&name)),
        _ => None,
    }
}

/// Frontier and local share this rule. A named agent may use a binding only
/// when `model_bindings` declares that binding's class and an allow Model
/// intention covers the class or the binding. Missing coverage is deny-default.
/// An explicit deny wins. `models:` on the agent stays required.
fn authorize_model(estate: &Estate, req: &AccessRequest<'_>) -> Decision {
    let Some(agent) = estate.agent(req.subject_agent) else {
        return deny(format!("unknown subject agent '{}'", req.subject_agent));
    };
    let parsed = ObjectRef::parse(req.object);
    let name = parsed.name();
    let Some(binding) = estate
        .model_bindings
        .iter()
        .find(|b| normalize_name(&b.id) == normalize_name(name))
    else {
        return deny(format!(
            "model binding '{name}' is not declared on model_bindings (deny-default)"
        ));
    };
    if !agent.has_model(name) {
        return deny(format!(
            "model '{name}' undeclared for agent '{}' (deny-default)",
            req.subject_agent
        ));
    }
    match model_intention_effect(estate, req.subject_agent, name, binding.class) {
        Some(Effect::Deny) => deny(format!(
            "explicit deny intention: {} model {name} class {}",
            req.subject_agent,
            binding.class.as_str()
        )),
        Some(Effect::Allow) => allow(format!(
            "allow intention: {} model {name} class {}",
            req.subject_agent,
            binding.class.as_str()
        )),
        None => deny(format!(
            "model class '{}' is not covered by an allow Model intention for agent '{}' (deny-default)",
            binding.class.as_str(),
            req.subject_agent
        )),
    }
}

fn model_object_token(raw: &str) -> String {
    let raw = raw.trim();
    let bare = raw
        .strip_prefix("binding:")
        .or_else(|| raw.strip_prefix("class:"))
        .or_else(|| raw.strip_prefix("model:"))
        .unwrap_or(raw);
    normalize_name(bare)
}

fn intention_covers_model(object: &str, binding_id: &str, class: ModelClass) -> bool {
    let token = model_object_token(object);
    token == normalize_name(binding_id) || token == class.as_str()
}

fn model_intention_effect(
    estate: &Estate,
    agent_id: &str,
    binding_id: &str,
    class: ModelClass,
) -> Option<Effect> {
    let subject = normalize_name(agent_id);
    let mut deny = false;
    let mut allow = false;
    for intention in &estate.intentions {
        if normalize_name(&intention.subject_agent) != subject
            || intention.kind != IntentionKind::Model
        {
            continue;
        }
        if !intention_covers_model(&intention.object, binding_id, class) {
            continue;
        }
        match intention.effect {
            Effect::Deny => deny = true,
            Effect::Allow => allow = true,
        }
    }
    if deny {
        Some(Effect::Deny)
    } else if allow {
        Some(Effect::Allow)
    } else {
        None
    }
}

/// One coverage fact. `agent_id` is the filter key. `line` is the display.
/// An empty `agent_id` is an estate-wide row (an empty hop population).
/// Hop rows also carry `hop_id`, `word`, and `capability` so convey does not parse `line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageRow {
    pub agent_id: String,
    pub line: String,
    /// Placement id when this row is hop coverage. Empty otherwise.
    pub hop_id: String,
    /// `allow`, `deny`, or `deny-default` for hop rows. Empty otherwise.
    pub word: &'static str,
    /// Placement-derived hop capability (`lane-tool` on box, `mesh-stub` on cloud).
    /// Empty on fact rows.
    pub capability: String,
}

impl CoverageRow {
    fn fact(agent_id: impl Into<String>, line: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            line: line.into(),
            hop_id: String::new(),
            word: "",
            capability: String::new(),
        }
    }

    fn hop(
        agent_id: impl Into<String>,
        hop_id: impl Into<String>,
        word: &'static str,
        capability: impl Into<String>,
        line: impl Into<String>,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            hop_id: hop_id.into(),
            word,
            capability: capability.into(),
            line: line.into(),
        }
    }
}

/// Rows whose `agent_id` matches. Display text is not parsed.
pub fn coverage_for_agent<'a>(
    rows: &'a [CoverageRow],
    agent_id: &str,
) -> impl Iterator<Item = &'a CoverageRow> {
    let want = normalize_name(agent_id);
    rows.iter()
        .filter(move |row| !row.agent_id.is_empty() && normalize_name(&row.agent_id) == want)
}

fn join_coverage(rows: &[CoverageRow], empty: &str) -> String {
    if rows.is_empty() {
        empty.into()
    } else {
        rows.iter().map(|row| row.line.as_str()).collect::<Vec<_>>().join("\n")
    }
}

pub fn coverage_word(decision: &Decision) -> &'static str {
    coverage_word_for_reason(decision.is_allow(), decision.reason())
}

/// Same words as [`coverage_word`]. Explicit deny stays `deny`. A reason that
/// says `deny-default` or `no intention` is `deny-default`.
pub fn coverage_word_for_reason(allow: bool, reason: &str) -> &'static str {
    if allow {
        "allow"
    } else if reason.contains("explicit deny") {
        "deny"
    } else if reason.contains("deny-default") || reason.contains("no intention") {
        "deny-default"
    } else {
        "deny"
    }
}

/// One row per agent `models:` entry. Frontier and local use the same words.
pub fn model_class_coverage_rows(estate: &Estate) -> Vec<CoverageRow> {
    let mut rows = Vec::new();
    for agent in &estate.agents {
        for model in &agent.models {
            let class = estate
                .model_bindings
                .iter()
                .find(|b| normalize_name(&b.id) == normalize_name(&model.id))
                .map(|b| b.class.as_str())
                .unwrap_or("undeclared");
            let covered = estate
                .model_bindings
                .iter()
                .find(|b| normalize_name(&b.id) == normalize_name(&model.id))
                .and_then(|b| model_intention_effect(estate, &agent.id, &model.id, b.class))
                .map(|effect| effect.as_str())
                .unwrap_or("deny-default");
            rows.push(CoverageRow::fact(
                agent.id.clone(),
                format!("{} {} {}: {covered}", agent.id, class, model.id),
            ));
        }
    }
    rows
}

/// One line per agent `models:` entry. Frontier and local use the same words.
pub fn describe_model_class_coverage(estate: &Estate) -> String {
    join_coverage(&model_class_coverage_rows(estate), "(no agent model uses)")
}

/// A declared tool, MCP, or mount is not enough. An allow intention of that
/// kind must cover the object (bare id, `tool:`, `mcp:`, or `mount:`). These
/// kinds have no class. Missing coverage is deny-default. An explicit deny wins.
fn authorize_declared_named(estate: &Estate, req: &AccessRequest<'_>) -> Decision {
    let Some(agent) = estate.agent(req.subject_agent) else {
        return deny(format!("unknown subject agent '{}'", req.subject_agent));
    };
    let name = ObjectRef::parse(req.object).name().to_string();
    let declared = match req.kind {
        IntentionKind::Tool => agent.has_tool(&name),
        IntentionKind::Mcp => agent.has_mcp(&name),
        IntentionKind::Mount => agent.has_mount(&name),
        _ => false,
    };
    let label = req.kind.as_str();
    if !declared {
        return deny(format!(
            "{label} '{name}' undeclared for agent '{}' (deny-default)",
            req.subject_agent
        ));
    }
    match named_intention_effect(estate, req.subject_agent, req.kind, &name) {
        Some(Effect::Deny) => deny(format!(
            "explicit deny intention: {} {label} {name}",
            req.subject_agent
        )),
        Some(Effect::Allow) => allow(format!(
            "allow intention: {} {label} {name}",
            req.subject_agent
        )),
        None => deny(format!(
            "{label} '{name}' is not covered by an allow {} intention for agent '{}' (deny-default)",
            kind_title(req.kind),
            req.subject_agent
        )),
    }
}

fn kind_title(kind: IntentionKind) -> &'static str {
    match kind {
        IntentionKind::Tool => "Tool",
        IntentionKind::Mcp => "Mcp",
        IntentionKind::Model => "Model",
        IntentionKind::Mount => "Mount",
        IntentionKind::MemoryRead => "Memory",
        IntentionKind::Agent => "Agent",
    }
}

fn intention_covers_named(object: &str, name: &str) -> bool {
    normalize_name(ObjectRef::parse(object).name()) == normalize_name(name)
}

fn named_intention_effect(
    estate: &Estate,
    agent_id: &str,
    kind: IntentionKind,
    name: &str,
) -> Option<Effect> {
    let subject = normalize_name(agent_id);
    let mut deny = false;
    let mut allow = false;
    for intention in &estate.intentions {
        if normalize_name(&intention.subject_agent) != subject || intention.kind != kind {
            continue;
        }
        if !intention_covers_named(&intention.object, name) {
            continue;
        }
        match intention.effect {
            Effect::Deny => deny = true,
            Effect::Allow => allow = true,
        }
    }
    if deny {
        Some(Effect::Deny)
    } else if allow {
        Some(Effect::Allow)
    } else {
        None
    }
}

/// One row per agent tool, MCP, and mount. These kinds have no class.
pub fn declared_coverage_rows(estate: &Estate) -> Vec<CoverageRow> {
    let mut rows = Vec::new();
    for agent in &estate.agents {
        for tool in &agent.tools {
            let covered = named_intention_effect(estate, &agent.id, IntentionKind::Tool, &tool.id)
                .map(|effect| effect.as_str())
                .unwrap_or("deny-default");
            rows.push(CoverageRow::fact(
                agent.id.clone(),
                format!("{} tool {}: {covered}", agent.id, tool.id),
            ));
        }
        for mcp in &agent.mcp {
            let covered = named_intention_effect(estate, &agent.id, IntentionKind::Mcp, &mcp.id)
                .map(|effect| effect.as_str())
                .unwrap_or("deny-default");
            rows.push(CoverageRow::fact(
                agent.id.clone(),
                format!("{} mcp {}: {covered}", agent.id, mcp.id),
            ));
        }
        for mount in &agent.mounts {
            let covered =
                named_intention_effect(estate, &agent.id, IntentionKind::Mount, &mount.id)
                    .map(|effect| effect.as_str())
                    .unwrap_or("deny-default");
            rows.push(CoverageRow::fact(
                agent.id.clone(),
                format!("{} mount {}: {covered}", agent.id, mount.id),
            ));
        }
    }
    rows
}

/// One line per agent tool, MCP, and mount. These kinds have no class.
pub fn describe_declared_coverage(estate: &Estate) -> String {
    join_coverage(
        &declared_coverage_rows(estate),
        "(no agent tool, mcp, or mount uses)",
    )
}

/// One row per ordered pair of estate agents. `own` is the same agent.
/// `peer` is another agent. Missing coverage is deny-default.
pub fn agent_edge_coverage_rows(estate: &Estate) -> Vec<CoverageRow> {
    let mut rows = Vec::new();
    for subject in &estate.agents {
        for target in &estate.agents {
            let object = format!("agent:{}", target.id);
            let decision = authorize(
                estate,
                &AccessRequest {
                    subject_agent: &subject.id,
                    kind: IntentionKind::Agent,
                    object: &object,
                },
            );
            let edge = if normalize_name(&subject.id) == normalize_name(&target.id) {
                "own"
            } else {
                "peer"
            };
            rows.push(CoverageRow::fact(
                subject.id.clone(),
                format!(
                    "{} agent {}: {} ({edge})",
                    subject.id,
                    target.id,
                    coverage_word(&decision)
                ),
            ));
        }
    }
    rows
}

/// Who may call whom. Plan, drift, and doctor print this cite.
pub fn describe_agent_edge_coverage(estate: &Estate) -> String {
    join_coverage(
        &agent_edge_coverage_rows(estate),
        "(no agent call edges)",
    )
}

/// One declared call on apply. `line` is the same `own` / `peer` row
/// [`describe_agent_edge_coverage`] prints. Allow is omitted (quiet).
/// Deny and deny-default stay visible. Agent-call rows have no
/// capability-mismatch class, so `fail` is false for those words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCallCoverageCite {
    pub fail: bool,
    pub line: String,
}

/// Declared `calls` only. Undeclared pairs stay on the plan, drift, and
/// doctor describe lines and are not apply findings. Does not read a mesh
/// and does not write.
pub fn agent_call_coverage_cites(estate: &Estate) -> Vec<AgentCallCoverageCite> {
    let rows = agent_edge_coverage_rows(estate);
    let mut cites = Vec::new();
    for subject in &estate.agents {
        for call in &subject.calls {
            let Some(target) = estate.agent(&call.id) else {
                cites.push(AgentCallCoverageCite {
                    fail: true,
                    line: format!(
                        "refuse:agent-call: {} agent {}: deny-default (not an estate agent)",
                        subject.id, call.id
                    ),
                });
                continue;
            };
            let prefix = format!("{} agent {}:", subject.id, target.id);
            let Some(row) = rows.iter().find(|row| {
                normalize_name(&row.agent_id) == normalize_name(&subject.id)
                    && row.line.starts_with(&prefix)
            }) else {
                cites.push(AgentCallCoverageCite {
                    fail: true,
                    line: format!("refuse:agent-call: {prefix} deny-default (missing edge)"),
                });
                continue;
            };
            if row.line.contains(": allow (") {
                continue;
            }
            // Deny and deny-default are the coverage words. They are not a
            // capability mismatch. An unrecognized word fails closed.
            let known = row.line.contains(": deny-default (") || row.line.contains(": deny (");
            cites.push(AgentCallCoverageCite {
                fail: !known,
                line: row.line.clone(),
            });
        }
    }
    cites
}

/// Own-lane memory is allow. Cross-lane memory is deny-default unless an
/// intention covers it. An explicit deny wins. Each compiled intention is
/// also a row so allow and deny are visible without reading the estate file.
pub fn intention_coverage_rows(estate: &Estate) -> Vec<CoverageRow> {
    let mut rows = Vec::new();
    for agent in &estate.agents {
        for lane in &estate.lanes {
            let object = format!("lane:{}", lane.id);
            let decision = authorize(
                estate,
                &AccessRequest {
                    subject_agent: &agent.id,
                    kind: IntentionKind::MemoryRead,
                    object: &object,
                },
            );
            let edge = if estate.owns_lane(&agent.id, &lane.id) {
                "own-lane"
            } else {
                "cross-lane"
            };
            rows.push(CoverageRow::fact(
                agent.id.clone(),
                format!(
                    "{} memory_read {object}: {} ({edge})",
                    agent.id,
                    coverage_word(&decision)
                ),
            ));
        }
        for intention in estate
            .intentions
            .iter()
            .filter(|i| normalize_name(&i.subject_agent) == normalize_name(&agent.id))
        {
            rows.push(CoverageRow::fact(
                agent.id.clone(),
                format!(
                    "{} intention {} {}: {}",
                    agent.id,
                    intention.kind.as_str(),
                    intention.object,
                    intention.effect.as_str()
                ),
            ));
        }
    }
    rows
}

pub fn describe_intention_coverage(estate: &Estate) -> String {
    join_coverage(&intention_coverage_rows(estate), "(no intention coverage)")
}

/// Placement-derived hop facts. Plan does not read a lease file and does not
/// spawn. Box capability is `lane-tool`. Cloud capability is `mesh-stub`.
/// A cloud hop stays `deny`. The printed line also says
/// `(declared, not spawned)` so plan, drift, and doctor do not read that
/// deny as an intention deny. The word stays `deny` so convey gates and
/// the capability-mismatch cite are unchanged. Cloud kinds stay out of
/// that cite: a cloud hop is not a capability mismatch, it is not spawned.
/// An empty population is deny-default (not a grant). A box hop uses the
/// same allow / deny / deny-default words as a declared capability of that name.
pub fn hop_coverage_rows(estate: &Estate) -> Vec<CoverageRow> {
    let mut rows = Vec::new();
    for place in &estate.placements {
        let cloud = place.kind == crate::PlacementKind::CloudAgent;
        let capability = if cloud { "mesh-stub" } else { "lane-tool" };
        if place.agents.is_empty() {
            let word = if cloud { "deny" } else { "deny-default" };
            rows.push(CoverageRow::hop(
                String::new(),
                place.id.clone(),
                word,
                capability,
                hop_coverage_line("", &place.id, capability, word, cloud),
            ));
            continue;
        }
        for agent_id in &place.agents {
            let word = hop_coverage_word(estate, agent_id, capability, cloud);
            rows.push(CoverageRow::hop(
                agent_id.clone(),
                place.id.clone(),
                word,
                capability,
                hop_coverage_line(agent_id, &place.id, capability, word, cloud),
            ));
        }
    }
    rows
}

/// Display line for one hop row. `word` is unchanged. Cloud rows name
/// declared-not-spawned so the graph is not a silent omit.
fn hop_coverage_line(
    agent_id: &str,
    place_id: &str,
    capability: &str,
    word: &str,
    cloud: bool,
) -> String {
    let line = if agent_id.is_empty() {
        format!("{place_id} hop {capability}: {word}")
    } else {
        format!("{agent_id} hop {place_id} {capability}: {word}")
    };
    if cloud {
        format!("{line} (declared, not spawned)")
    } else {
        line
    }
}

fn hop_coverage_word(
    estate: &Estate,
    agent_id: &str,
    capability: &str,
    cloud: bool,
) -> &'static str {
    if cloud {
        return "deny";
    }
    if estate.agent(agent_id).is_none() {
        return "deny-default";
    }
    // Agent is who-may-call-whom. It is not a hop grant. A call id that
    // matches the reserved hop capability must not allow or scramble this word.
    let kinds: Vec<IntentionKind> = capability_kinds(estate, agent_id, capability)
        .into_iter()
        .filter(|kind| *kind != IntentionKind::Agent)
        .collect();
    match kinds.as_slice() {
        [kind] => coverage_word(&authorize(
            estate,
            &AccessRequest {
                subject_agent: agent_id,
                kind: *kind,
                object: capability,
            },
        )),
        _ => "deny-default",
    }
}

pub fn describe_hop_coverage(estate: &Estate) -> String {
    join_coverage(&hop_coverage_rows(estate), "(no hop coverage)")
}

/// Same order as conveyor-proxy `resolve_intention_kind`: `lane:` is
/// MemoryRead before a declared tool, MCP, mount, or model. One hit infers.
/// Zero or many hits stay unresolved so the caller can name the reason.
fn infer_convey_intention_kind(
    capability: &str,
    hits: &[IntentionKind],
) -> Option<IntentionKind> {
    if capability.starts_with("lane:") {
        return Some(IntentionKind::MemoryRead);
    }
    match hits {
        [one] => Some(*one),
        _ => None,
    }
}

fn capability_kinds(estate: &Estate, agent_id: &str, capability: &str) -> Vec<IntentionKind> {
    let Some(agent) = estate.agent(agent_id) else {
        return Vec::new();
    };
    let mut hits = Vec::new();
    if agent.has_tool(capability) {
        hits.push(IntentionKind::Tool);
    }
    if agent.has_mcp(capability) {
        hits.push(IntentionKind::Mcp);
    }
    if agent.has_mount(capability) {
        hits.push(IntentionKind::Mount);
    }
    if agent.has_model(capability) {
        hits.push(IntentionKind::Model);
    }
    if declared_agent_call(agent, capability) {
        hits.push(IntentionKind::Agent);
    }
    hits
}

fn hop_reserved_capability(capability: &str) -> bool {
    matches!(
        normalize_name(capability).as_str(),
        "lane-tool" | "mesh-stub"
    )
}

fn declared_agent_call(agent: &crate::types::Agent, capability: &str) -> bool {
    // Box hops are `lane-tool`. Cloud hops are `mesh-stub`. Those strings
    // stay tool/hop capabilities. A peer call uses `agent:` or `--kind agent`.
    if hop_reserved_capability(capability) {
        return false;
    }
    let name = capability
        .strip_prefix("agent:")
        .unwrap_or(capability)
        .trim();
    !name.is_empty() && agent.has_call(name)
}

/// Named-agent intention for one convey capability. `Ok` is allow and the
/// caller continues to hop coverage. `Err` is deny or deny-default.
/// Kind inference matches `resolve_intention_kind`: an explicit kind wins,
/// `lane:` is MemoryRead, and one declared tool, MCP, mount, model, or
/// agent call is that kind. A missing intention is deny-default. More than one declared
/// kind is still deny-default. Call says `ambiguous capability; pass kind`.
/// Hop says `pass --intention-kind` so that hint is not hop `--kind`.
/// An explicit deny wins. An unresolved `mesh-stub` on a cloud-agent
/// placement stays the hop-coverage gate. A box hop does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentionCoverageGate {
    pub word: &'static str,
    pub line: String,
}

pub fn convey_intention_coverage(
    estate: &Estate,
    hop_id: &str,
    agent_id: &str,
    capability: &str,
    kind: Option<IntentionKind>,
    on_hop: bool,
) -> Result<(), IntentionCoverageGate> {
    if estate.agent(agent_id).is_none() {
        return Err(IntentionCoverageGate {
            word: "deny-default",
            line: format!(
                "{agent_id} intention {capability}: deny-default (unknown agent; not a grant)"
            ),
        });
    }
    let hits = capability_kinds(estate, agent_id, capability);
    let resolved = kind.or_else(|| infer_convey_intention_kind(capability, &hits));
    let Some(kind) = resolved else {
        // Cloud mesh-stub only. A box row whose word is deny (explicit
        // lane-tool deny) must not hide an unresolved capability.
        if capability == "mesh-stub"
            && estate.placements.iter().any(|place| {
                normalize_name(&place.id) == normalize_name(hop_id)
                    && place.kind == crate::PlacementKind::CloudAgent
            })
        {
            return Ok(());
        }
        let why = if hits.len() > 1 {
            if on_hop {
                "ambiguous capability; pass --intention-kind".to_string()
            } else {
                "ambiguous capability; pass kind".to_string()
            }
        } else {
            format!("undeclared for agent '{agent_id}' (deny-default)")
        };
        return Err(IntentionCoverageGate {
            word: "deny-default",
            line: format!("{agent_id} intention {capability}: deny-default ({why})"),
        });
    };
    let object = if kind == IntentionKind::MemoryRead && !capability.contains(':') {
        format!("lane:{capability}")
    } else {
        capability.to_string()
    };
    let decision = authorize(
        estate,
        &AccessRequest {
            subject_agent: agent_id,
            kind,
            object: &object,
        },
    );
    let word = coverage_word(&decision);
    if word == "allow" {
        return Ok(());
    }
    Err(IntentionCoverageGate {
        word,
        line: format!(
            "{agent_id} intention {} {capability}: {word}",
            kind.as_str()
        ),
    })
}

/// One placement-derived hop row that a convey path would use.
/// `word` is `allow`, `deny`, or `deny-default` — the same tokens
/// [`hop_coverage_rows`] prints. Plan, doctor, and drift stay print-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HopCoverageGate {
    pub word: &'static str,
    pub line: String,
    /// Placement-derived capability from [`hop_coverage_rows`]. Empty when
    /// the hop id is not a placement.
    pub capability: String,
}

/// Coverage for a convey hop target. `Ok(None)` means this hop id is not a
/// placement, so the lease stub is unchanged. `Ok(Some)` is allow. `Err` is
/// deny or deny-default and names which. Empty population is not a grant.
/// A cloud-agent hop is deny. Does not spawn and does not write.
pub fn convey_hop_coverage(
    estate: &Estate,
    hop_id: &str,
    agent_id: Option<&str>,
) -> Result<Option<HopCoverageGate>, HopCoverageGate> {
    let want_hop = normalize_name(hop_id);
    let Some(place) = estate
        .placements
        .iter()
        .find(|place| normalize_name(&place.id) == want_hop)
    else {
        return Ok(None);
    };
    let rows = hop_coverage_rows(estate);
    let mut hits: Vec<&CoverageRow> = rows
        .iter()
        .filter(|row| !row.hop_id.is_empty() && normalize_name(&row.hop_id) == want_hop)
        .collect();
    if hits.is_empty() {
        // Placement exists. Missing metadata is not "not a placement".
        return Err(HopCoverageGate {
            word: "deny-default",
            line: format!(
                "{} hop: deny-default (placement coverage missing; not a grant)",
                place.id
            ),
            capability: String::new(),
        });
    }
    if let Some(agent) = agent_id {
        let want = normalize_name(agent);
        let named: Vec<&CoverageRow> = hits
            .iter()
            .copied()
            .filter(|row| !row.agent_id.is_empty() && normalize_name(&row.agent_id) == want)
            .collect();
        if named.is_empty() {
            // Only an empty-population cloud row (no agent id, word deny)
            // is deny for a named outsider. A sibling's explicit deny stays
            // that sibling's row; the outsider is deny-default, not on the
            // population.
            if let Some(row) = hits
                .iter()
                .find(|row| row.agent_id.is_empty() && row.word == "deny")
            {
                return Err(HopCoverageGate {
                    word: "deny",
                    line: row.line.clone(),
                    capability: row.capability.clone(),
                });
            }
            return Err(HopCoverageGate {
                word: "deny-default",
                line: format!(
                    "{agent} hop {hop_id}: deny-default (not on the hop population; not a grant)"
                ),
                capability: String::new(),
            });
        }
        hits = named;
    }
    if let Some(gate) = hop_gate_from_hits(&hits) {
        return Err(gate);
    }
    let line = hits
        .first()
        .map(|row| row.line.clone())
        .unwrap_or_else(|| format!("{hop_id}: allow"));
    let capability = hits
        .first()
        .map(|row| row.capability.clone())
        .unwrap_or_default();
    Ok(Some(HopCoverageGate {
        word: "allow",
        line,
        capability,
    }))
}

/// Allow continues only when `declared` equals the placement-derived
/// capability on the allow row (`lane-tool` on box, `mesh-stub` on cloud,
/// or whatever [`hop_coverage_rows`] stored). Deny and deny-default return
/// unchanged. A hop id that is not a placement stays `Ok(None)`.
pub fn convey_hop_declared_capability(
    estate: &Estate,
    hop_id: &str,
    agent_id: Option<&str>,
    declared: &str,
) -> Result<Option<HopCoverageGate>, HopCoverageGate> {
    let gate = convey_hop_coverage(estate, hop_id, agent_id)?;
    let Some(gate) = gate else {
        return Ok(None);
    };
    if gate.word != "allow" {
        return Err(gate);
    }
    let want = gate.capability.as_str();
    if declared == want {
        return Ok(Some(gate));
    }
    Err(HopCoverageGate {
        word: "mismatch",
        line: format!(
            "capability '{declared}' does not match hop coverage capability '{want}'"
        ),
        capability: want.to_string(),
    })
}

fn hop_gate_from_hits(hits: &[&CoverageRow]) -> Option<HopCoverageGate> {
    if let Some(row) = hits.iter().find(|row| row.word == "deny") {
        return Some(HopCoverageGate {
            word: "deny",
            line: row.line.clone(),
            capability: row.capability.clone(),
        });
    }
    if let Some(row) = hits.iter().find(|row| row.word == "deny-default") {
        return Some(HopCoverageGate {
            word: "deny-default",
            line: row.line.clone(),
            capability: row.capability.clone(),
        });
    }
    None
}

fn authorize_memory(estate: &Estate, req: &AccessRequest<'_>) -> Decision {
    let object = ObjectRef::parse(req.object);
    let lane_id = object.name();
    if estate.owns_lane(req.subject_agent, lane_id) {
        return allow(format!(
            "agent '{}' owns lane '{}'",
            req.subject_agent, lane_id
        ));
    }
    deny(format!(
        "cross-lane memory read denied: {} -> {} (no intention)",
        req.subject_agent, req.object
    ))
}

fn explicit_intention(estate: &Estate, req: &AccessRequest<'_>) -> Option<Decision> {
    let subject = normalize_name(req.subject_agent);
    let object = normalize_name(ObjectRef::parse(req.object).name());
    let mut matched: Vec<Effect> = estate
        .intentions
        .iter()
        .filter(|i| {
            normalize_name(&i.subject_agent) == subject
                && i.kind == req.kind
                && normalize_name(ObjectRef::parse(&i.object).name()) == object
        })
        .map(|i| i.effect)
        .collect();
    if matched.is_empty() {
        return None;
    }
    if matched.iter().any(|e| matches!(e, Effect::Deny)) {
        return Some(deny(format!(
            "explicit deny intention: {} {} {}",
            req.subject_agent,
            req.kind.as_str(),
            req.object
        )));
    }
    if matched.iter().any(|e| matches!(e, Effect::Allow)) {
        return Some(allow(format!(
            "allow intention: {} {} {}",
            req.subject_agent,
            req.kind.as_str(),
            req.object
        )));
    }
    let _ = matched.pop();
    None
}

/// Read a file under a lane root only after authorize() allows it.
pub fn read_lane_file(
    estate: &Estate,
    roots_base: &Path,
    subject: &str,
    lane_id: &str,
    rel: &Path,
) -> Result<Vec<u8>, Deny> {
    let object = format!("lane:{lane_id}");
    match authorize(
        estate,
        &AccessRequest {
            subject_agent: subject,
            kind: IntentionKind::MemoryRead,
            object: &object,
        },
    ) {
        Decision::Allow { .. } => {}
        Decision::Deny(d) => return Err(d),
    }
    if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(Deny {
            reason: "path escapes lane".into(),
        });
    }
    let lane = estate.lane(lane_id).ok_or_else(|| Deny {
        reason: format!("unknown lane '{lane_id}'"),
    })?;
    let path = roots_base.join(&lane.root_path).join(rel);
    std::fs::read(&path).map_err(|e| Deny {
        reason: format!("lane read io: {e}"),
    })
}

fn allow(reason: impl Into<String>) -> Decision {
    Decision::Allow {
        reason: reason.into(),
    }
}

fn deny(reason: impl Into<String>) -> Decision {
    Decision::Deny(Deny {
        reason: reason.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load_estate_str, Effect, Intention};

    fn estate() -> Estate {
        load_estate_str(crate::tests::example_yaml()).unwrap()
    }

    fn req<'a>(agent: &'a str, kind: IntentionKind, object: &'a str) -> AccessRequest<'a> {
        AccessRequest {
            subject_agent: agent,
            kind,
            object,
        }
    }

    #[test]
    fn own_lane_memory_allowed() {
        let e = estate();
        assert!(authorize(
            &e,
            &req("horizon", IntentionKind::MemoryRead, "lane:horizon")
        )
        .is_allow());
    }

    #[test]
    fn cross_lane_memory_denied() {
        let e = estate();
        let d = authorize(
            &e,
            &req("horizon", IntentionKind::MemoryRead, "lane:research"),
        );
        assert!(!d.is_allow());
        assert!(d.reason().contains("cross-lane"));
    }

    #[test]
    fn sacred_cyera_denied() {
        let e = estate();
        for object in [
            "cyera-ci",
            "cyera",
            "exclusion:cyera-ci",
            "lane:rust-classroom",
        ] {
            let d = authorize(&e, &req("horizon", IntentionKind::MemoryRead, object));
            assert!(!d.is_allow(), "{object} should be denied");
            assert!(d.reason().contains("sacred"));
        }
    }

    #[test]
    fn undeclared_tool_denied() {
        let e = estate();
        let d = authorize(&e, &req("horizon", IntentionKind::Tool, "shell"));
        assert!(!d.is_allow());
        assert!(d.reason().contains("undeclared"));
    }

    fn grant(
        estate: &mut Estate,
        agent: &str,
        kind: IntentionKind,
        object: &str,
        effect: Effect,
    ) {
        estate.intentions.push(Intention {
            subject_agent: agent.into(),
            object: object.into(),
            kind,
            effect,
            note: None,
        });
    }

    #[test]
    fn declared_tool_without_allow_is_deny_default() {
        let e = estate();
        let d = authorize(&e, &req("research", IntentionKind::Tool, "notes-append"));
        assert!(!d.is_allow());
        assert!(d.reason().contains("not covered by an allow Tool intention"));
        assert!(d.reason().contains("deny-default"));
        assert!(!d.reason().contains("class"));
    }

    fn declare_call(estate: &mut Estate, agent_id: &str, target: &str) {
        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == agent_id)
            .unwrap()
            .calls
            .push(crate::CallDecl {
                id: target.into(),
                description: None,
            });
    }

    #[test]
    fn agent_call_allow_deny_and_deny_default() {
        let mut allowed = estate();
        declare_call(&mut allowed, "horizon", "research");
        grant(
            &mut allowed,
            "horizon",
            IntentionKind::Agent,
            "agent:research",
            Effect::Allow,
        );
        let ok = authorize(&allowed, &req("horizon", IntentionKind::Agent, "research"));
        assert!(ok.is_allow(), "{}", ok.reason());
        assert!(authorize(&allowed, &req("horizon", IntentionKind::Agent, "agent:research")).is_allow());

        let mut denied = estate();
        declare_call(&mut denied, "horizon", "research");
        grant(
            &mut denied,
            "horizon",
            IntentionKind::Agent,
            "research",
            Effect::Deny,
        );
        let explicit = authorize(&denied, &req("horizon", IntentionKind::Agent, "agent:research"));
        assert!(!explicit.is_allow());
        assert!(explicit.reason().contains("explicit deny"), "{}", explicit.reason());

        let mut bare = estate();
        declare_call(&mut bare, "horizon", "research");
        let missing = authorize(&bare, &req("horizon", IntentionKind::Agent, "research"));
        assert!(!missing.is_allow());
        assert!(missing.reason().contains("deny-default"), "{}", missing.reason());
        assert!(missing.reason().contains("not covered"), "{}", missing.reason());

        let undeclared = authorize(&estate(), &req("horizon", IntentionKind::Agent, "research"));
        assert!(!undeclared.is_allow());
        assert!(undeclared.reason().contains("undeclared"), "{}", undeclared.reason());

        let allow_gate = convey_intention_coverage(
            &allowed,
            "cell-one-box",
            "horizon",
            "agent:research",
            None,
            false,
        );
        assert!(allow_gate.is_ok(), "{allow_gate:?}");
        let deny_gate = convey_intention_coverage(
            &denied,
            "cell-one-box",
            "horizon",
            "research",
            Some(IntentionKind::Agent),
            true,
        )
        .unwrap_err();
        assert_eq!(deny_gate.word, "deny");
        assert!(deny_gate.line.contains("agent"), "{}", deny_gate.line);
        let default_gate = convey_intention_coverage(
            &bare,
            "cell-one-box",
            "horizon",
            "agent:research",
            None,
            false,
        )
        .unwrap_err();
        assert_eq!(default_gate.word, "deny-default");
    }

    #[test]
    fn agent_call_unknown_and_sacred_refuse() {
        let unknown = authorize(&estate(), &req("horizon", IntentionKind::Agent, "ghost"));
        assert!(!unknown.is_allow());
        assert!(unknown.reason().contains("not an estate agent"), "{}", unknown.reason());

        let sacred = authorize(&estate(), &req("horizon", IntentionKind::Agent, "cyera-ci"));
        assert!(!sacred.is_allow());
        assert!(sacred.reason().contains("sacred"), "{}", sacred.reason());
        let prefixed = authorize(
            &estate(),
            &req("horizon", IntentionKind::Agent, "agent:rust-classroom"),
        );
        assert!(!prefixed.is_allow());
        assert!(prefixed.reason().contains("sacred"), "{}", prefixed.reason());
    }

    #[test]
    fn agent_edge_coverage_names_own_and_peer() {
        let mut allowed = estate();
        declare_call(&mut allowed, "horizon", "horizon");
        declare_call(&mut allowed, "horizon", "research");
        grant(
            &mut allowed,
            "horizon",
            IntentionKind::Agent,
            "agent:research",
            Effect::Allow,
        );
        grant(
            &mut allowed,
            "horizon",
            IntentionKind::Agent,
            "horizon",
            Effect::Deny,
        );
        let text = describe_agent_edge_coverage(&allowed);
        assert!(text.contains("horizon agent horizon: deny (own)"), "{text}");
        assert!(text.contains("horizon agent research: allow (peer)"), "{text}");
        assert!(text.contains("horizon agent sanctum: deny-default (peer)"), "{text}");
        let rows = agent_edge_coverage_rows(&estate());
        assert!(rows.iter().any(|row| row.line.contains("(own)")));
        assert!(rows.iter().any(|row| row.line.contains("(peer)")));

        let cites = agent_call_coverage_cites(&allowed);
        assert_eq!(cites.len(), 1, "{cites:?}");
        assert!(cites.iter().all(|cite| !cite.fail), "{cites:?}");
        assert!(
            cites.iter().any(|cite| cite.line == "horizon agent horizon: deny (own)"),
            "{cites:?}"
        );
        assert!(
            cites.iter().all(|cite| cite.line != "horizon agent research: allow (peer)"),
            "{cites:?}"
        );
        let bare = {
            let mut estate = estate();
            declare_call(&mut estate, "horizon", "research");
            estate
        };
        let defaults = agent_call_coverage_cites(&bare);
        assert_eq!(defaults.len(), 1, "{defaults:?}");
        assert!(!defaults[0].fail);
        assert_eq!(defaults[0].line, "horizon agent research: deny-default (peer)");
        assert!(agent_call_coverage_cites(&estate()).is_empty());
        let mut ghost = estate();
        declare_call(&mut ghost, "horizon", "ghost");
        let missing = agent_call_coverage_cites(&ghost);
        assert_eq!(missing.len(), 1, "{missing:?}");
        assert!(missing[0].fail, "{missing:?}");
        assert!(
            missing[0].line.contains("refuse:agent-call")
                && missing[0].line.contains("not an estate agent"),
            "{}",
            missing[0].line
        );
    }

    #[test]
    fn hop_coverage_does_not_grant_on_agent_call_named_lane_tool() {
        let mut estate = estate();
        let mut peer = estate
            .agents
            .iter()
            .find(|agent| agent.id == "research")
            .unwrap()
            .clone();
        peer.id = "lane-tool".into();
        peer.display_name = "Lane Tool".into();
        peer.calls.clear();
        estate.agents.push(peer);
        declare_call(&mut estate, "research", "lane-tool");
        grant(
            &mut estate,
            "research",
            IntentionKind::Agent,
            "lane-tool",
            Effect::Allow,
        );
        let hops = describe_hop_coverage(&estate);
        assert!(
            hops.contains("research hop cell-one-box lane-tool: deny-default"),
            "{hops}"
        );
        assert!(!hops.contains("research hop cell-one-box lane-tool: allow"), "{hops}");
        let inferred = convey_intention_coverage(
            &estate,
            "cell-one-box",
            "research",
            "lane-tool",
            None,
            true,
        )
        .unwrap_err();
        assert_eq!(inferred.word, "deny-default");
        assert!(
            inferred.line.contains("undeclared"),
            "{}",
            inferred.line
        );
        let explicit = convey_intention_coverage(
            &estate,
            "cell-one-box",
            "research",
            "lane-tool",
            Some(IntentionKind::Agent),
            true,
        );
        assert!(explicit.is_ok(), "{explicit:?}");

        estate
            .agents
            .iter_mut()
            .find(|agent| agent.id == "research")
            .unwrap()
            .tools
            .push(crate::ToolDecl {
                id: "lane-tool".into(),
                description: None,
            });
        grant(
            &mut estate,
            "research",
            IntentionKind::Tool,
            "lane-tool",
            Effect::Allow,
        );
        let both = describe_hop_coverage(&estate);
        assert!(
            both.contains("research hop cell-one-box lane-tool: allow"),
            "{both}"
        );
        assert!(
            convey_intention_coverage(
                &estate,
                "cell-one-box",
                "research",
                "lane-tool",
                None,
                true,
            )
            .is_ok()
        );
    }

    #[test]
    fn tool_allow_deny_and_prefix_are_symmetric() {
        let raw = estate();
        let mut allowed = raw.clone();
        grant(
            &mut allowed,
            "research",
            IntentionKind::Tool,
            "tool:notes-append",
            Effect::Allow,
        );
        let ok = authorize(&allowed, &req("research", IntentionKind::Tool, "notes-append"));
        assert!(ok.is_allow(), "{}", ok.reason());
        assert!(ok.reason().contains("allow intention"));
        let mut bare = raw.clone();
        grant(
            &mut bare,
            "research",
            IntentionKind::Tool,
            "notes-append",
            Effect::Allow,
        );
        assert!(authorize(&bare, &req("research", IntentionKind::Tool, "tool:notes-append")).is_allow());

        let mut denied = allowed;
        grant(
            &mut denied,
            "research",
            IntentionKind::Tool,
            "notes-append",
            Effect::Deny,
        );
        let explicit = authorize(&denied, &req("research", IntentionKind::Tool, "notes-append"));
        assert!(!explicit.is_allow());
        assert!(explicit.reason().contains("explicit deny"));
    }

    #[test]
    fn mcp_allow_deny_and_missing_are_symmetric() {
        let mut raw = estate();
        raw.agents
            .iter_mut()
            .find(|a| a.id == "research")
            .unwrap()
            .mcp
            .push(crate::McpDecl {
                id: "docs".into(),
                description: None,
            });
        let missing = authorize(&raw, &req("research", IntentionKind::Mcp, "docs"));
        assert!(!missing.is_allow());
        assert!(missing.reason().contains("not covered by an allow Mcp intention"));
        assert!(missing.reason().contains("deny-default"));
        assert!(!missing.reason().contains("class"));
        let undeclared = authorize(&raw, &req("research", IntentionKind::Mcp, "other"));
        assert!(undeclared.reason().contains("undeclared"));

        let mut allowed = raw.clone();
        grant(&mut allowed, "research", IntentionKind::Mcp, "mcp:docs", Effect::Allow);
        let ok = authorize(&allowed, &req("research", IntentionKind::Mcp, "docs"));
        assert!(ok.is_allow(), "{}", ok.reason());
        let mut denied = allowed;
        grant(&mut denied, "research", IntentionKind::Mcp, "docs", Effect::Deny);
        let explicit = authorize(&denied, &req("research", IntentionKind::Mcp, "mcp:docs"));
        assert!(!explicit.is_allow());
        assert!(explicit.reason().contains("explicit deny"));
    }

    #[test]
    fn tool_mcp_coverage_names_allow_deny_and_default() {
        let mut e = estate();
        e.agents
            .iter_mut()
            .find(|a| a.id == "horizon")
            .unwrap()
            .mcp
            .push(crate::McpDecl {
                id: "docs".into(),
                description: None,
            });
        grant(&mut e, "horizon", IntentionKind::Mcp, "docs", Effect::Deny);
        let text = describe_declared_coverage(&e);
        assert!(text.contains("research tool notes-append: deny-default"));
        assert!(text.contains("horizon mcp docs: deny"));
        assert!(text.contains("research mount notes: deny-default"));
        grant(&mut e, "research", IntentionKind::Tool, "notes-append", Effect::Allow);
        grant(&mut e, "research", IntentionKind::Mount, "notes", Effect::Deny);
        let text = describe_declared_coverage(&e);
        assert!(text.contains("research tool notes-append: allow"));
        assert!(text.contains("research mount notes: deny"));
    }

    #[test]
    fn declared_mount_without_allow_is_deny_default() {
        let e = estate();
        let d = authorize(&e, &req("research", IntentionKind::Mount, "notes"));
        assert!(!d.is_allow());
        assert!(d.reason().contains("not covered by an allow Mount intention"));
        assert!(d.reason().contains("deny-default"));
        assert!(!d.reason().contains("class"));
        let undeclared = authorize(&e, &req("research", IntentionKind::Mount, "secrets"));
        assert!(!undeclared.is_allow());
        assert!(undeclared.reason().contains("undeclared"));
    }

    #[test]
    fn mount_allow_deny_and_prefix_are_symmetric() {
        let raw = estate();
        let mut allowed = raw.clone();
        grant(
            &mut allowed,
            "research",
            IntentionKind::Mount,
            "mount:notes",
            Effect::Allow,
        );
        let ok = authorize(&allowed, &req("research", IntentionKind::Mount, "notes"));
        assert!(ok.is_allow(), "{}", ok.reason());
        assert!(ok.reason().contains("allow intention"));
        let mut bare = raw.clone();
        grant(
            &mut bare,
            "research",
            IntentionKind::Mount,
            "notes",
            Effect::Allow,
        );
        assert!(authorize(&bare, &req("research", IntentionKind::Mount, "mount:notes")).is_allow());

        let mut denied = allowed;
        grant(
            &mut denied,
            "research",
            IntentionKind::Mount,
            "notes",
            Effect::Deny,
        );
        let explicit = authorize(&denied, &req("research", IntentionKind::Mount, "mount:notes"));
        assert!(!explicit.is_allow());
        assert!(explicit.reason().contains("explicit deny"));
    }

    #[test]
    fn intention_can_allow_cross_lane() {
        let mut e = estate();
        e.intentions.push(Intention {
            subject_agent: "horizon".into(),
            object: "lane:research".into(),
            kind: IntentionKind::MemoryRead,
            effect: Effect::Allow,
            note: Some("test grant".into()),
        });
        assert!(authorize(
            &e,
            &req("horizon", IntentionKind::MemoryRead, "lane:research")
        )
        .is_allow());
        // still deny the other way
        assert!(!authorize(
            &e,
            &req("research", IntentionKind::MemoryRead, "lane:horizon")
        )
        .is_allow());
    }

    fn allow_class(estate: &mut Estate, agent: &str, class: &str) {
        estate.intentions.push(Intention {
            subject_agent: agent.into(),
            object: format!("class:{class}"),
            kind: IntentionKind::Model,
            effect: Effect::Allow,
            note: None,
        });
    }

    #[test]
    fn model_class_allow_and_deny_are_symmetric() {
        let raw = estate();
        for (agent, binding, class) in [
            ("horizon", "xai_grok", "frontier"),
            ("horizon", "local_slm", "local"),
            ("research", "local_slm", "local"),
        ] {
            let denied = authorize(&raw, &req(agent, IntentionKind::Model, binding));
            assert!(!denied.is_allow(), "{agent} {binding}");
            assert!(
                denied.reason().contains(&format!("model class '{class}'")),
                "{}",
                denied.reason()
            );
            assert!(
                denied.reason().contains("deny-default"),
                "{}",
                denied.reason()
            );
        }

        let mut frontier_only = raw.clone();
        allow_class(&mut frontier_only, "horizon", "frontier");
        let frontier = authorize(
            &frontier_only,
            &req("horizon", IntentionKind::Model, "xai_grok"),
        );
        assert!(frontier.is_allow(), "{}", frontier.reason());
        assert!(frontier.reason().contains("class frontier"));
        let local_still = authorize(
            &frontier_only,
            &req("horizon", IntentionKind::Model, "local_slm"),
        );
        assert!(!local_still.is_allow(), "{}", local_still.reason());
        assert!(local_still.reason().contains("model class 'local'"));

        let mut local_only = raw.clone();
        allow_class(&mut local_only, "horizon", "local");
        allow_class(&mut local_only, "research", "local");
        let local = authorize(
            &local_only,
            &req("horizon", IntentionKind::Model, "local_slm"),
        );
        assert!(local.is_allow(), "{}", local.reason());
        assert!(local.reason().contains("class local"));
        let research = authorize(
            &local_only,
            &req("research", IntentionKind::Model, "local_slm"),
        );
        assert!(research.is_allow(), "{}", research.reason());
        let frontier_still = authorize(
            &local_only,
            &req("horizon", IntentionKind::Model, "xai_grok"),
        );
        assert!(!frontier_still.is_allow());
        assert!(frontier_still.reason().contains("model class 'frontier'"));

        let missing = authorize(&raw, &req("horizon", IntentionKind::Model, "other_slm"));
        assert!(!missing.is_allow());
        assert!(missing.reason().contains("not declared on model_bindings"));
        let sanctum = authorize(&raw, &req("sanctum", IntentionKind::Model, "xai_grok"));
        assert!(!sanctum.is_allow());
        assert!(sanctum.reason().contains("undeclared"));

        let mut denied_class = frontier_only;
        denied_class.intentions.push(Intention {
            subject_agent: "horizon".into(),
            object: "class:frontier".into(),
            kind: IntentionKind::Model,
            effect: Effect::Deny,
            note: None,
        });
        let explicit = authorize(
            &denied_class,
            &req("horizon", IntentionKind::Model, "xai_grok"),
        );
        assert!(!explicit.is_allow());
        assert!(explicit.reason().contains("explicit deny"));
        assert!(explicit.reason().contains("class frontier"));
    }

    #[test]
    fn intention_and_hop_coverage_name_allow_deny_and_default() {
        let raw = estate();
        let memory = describe_intention_coverage(&raw);
        assert!(memory.contains("horizon memory_read lane:horizon: allow (own-lane)"));
        assert!(memory.contains("horizon memory_read lane:research: deny-default (cross-lane)"));
        assert!(memory.contains("research memory_read lane:sanctum: deny-default (cross-lane)"));
        assert!(!memory.contains("intention "));
        let hops = describe_hop_coverage(&raw);
        assert!(hops.contains("horizon hop cell-one-box lane-tool: deny-default"));
        assert!(hops.contains("research hop cell-one-box lane-tool: deny-default"));
        assert!(hops.contains("sanctum hop cell-one-box lane-tool: deny-default"));
        assert!(hops.contains("cursor-cloud hop mesh-stub: deny (declared, not spawned)"));
        assert!(!hops.contains("cell-one-box lane-tool: deny-default (declared, not spawned)"));

        let mut granted = raw.clone();
        grant(&mut granted, "horizon", IntentionKind::MemoryRead, "lane:research", Effect::Allow);
        let allowed = describe_intention_coverage(&granted);
        assert!(allowed.contains("horizon memory_read lane:research: allow (cross-lane)"));
        assert!(allowed.contains("horizon intention memory_read lane:research: allow"));
        let mut denied = granted;
        grant(&mut denied, "horizon", IntentionKind::MemoryRead, "lane:research", Effect::Deny);
        let explicit = describe_intention_coverage(&denied);
        assert!(explicit.contains("horizon memory_read lane:research: deny (cross-lane)"));
        assert!(!explicit.contains("horizon memory_read lane:research: allow"));
        assert!(explicit.contains("horizon intention memory_read lane:research: deny"));

        let rows = intention_coverage_rows(&denied);
        let horizon: Vec<_> = coverage_for_agent(&rows, "horizon").map(|row| row.line.as_str()).collect();
        assert!(horizon.iter().any(|line| line.contains("memory_read lane:research: deny")));
        assert!(coverage_for_agent(&rows, "research").all(|row| row.agent_id == "research"));

        let mut hopped = raw.clone();
        hopped.agents.iter_mut().find(|a| a.id == "research").unwrap().tools.push(crate::ToolDecl {
            id: "lane-tool".into(),
            description: None,
        });
        assert!(describe_hop_coverage(&hopped).contains("research hop cell-one-box lane-tool: deny-default"));
        grant(&mut hopped, "research", IntentionKind::Tool, "lane-tool", Effect::Allow);
        assert!(describe_hop_coverage(&hopped).contains("research hop cell-one-box lane-tool: allow"));
        grant(&mut hopped, "research", IntentionKind::Tool, "lane-tool", Effect::Deny);
        assert!(describe_hop_coverage(&hopped).contains("research hop cell-one-box lane-tool: deny"));
        assert!(!describe_hop_coverage(&hopped).contains("lane-tool: deny (declared, not spawned)"));
        hopped.placements.iter_mut().find(|p| p.id == "cursor-cloud").unwrap().agents = vec!["sanctum".into()];
        let cloud = describe_hop_coverage(&hopped);
        assert!(cloud.contains("sanctum hop cursor-cloud mesh-stub: deny (declared, not spawned)"));
        let cloud_row = hop_coverage_rows(&hopped)
            .into_iter()
            .find(|row| row.hop_id == "cursor-cloud")
            .expect("cloud hop row");
        assert_eq!(cloud_row.word, "deny");
        assert!(!cloud.contains("cursor-cloud hop mesh-stub:"));
    }

    #[test]
    fn convey_hop_coverage_refuses_deny_and_default_allows_only_allow() {
        let raw = estate();
        let empty_cloud = convey_hop_coverage(&raw, "cursor-cloud", None).unwrap_err();
        assert_eq!(empty_cloud.word, "deny");
        assert!(empty_cloud.line.contains("deny (declared, not spawned)"));
        assert!(!empty_cloud.line.contains("deny-default"));

        let populated = convey_hop_coverage(&raw, "cell-one-box", None).unwrap_err();
        assert_eq!(populated.word, "deny-default");
        assert!(populated.line.contains("deny-default"));

        let mut empty_box = raw.clone();
        empty_box
            .placements
            .iter_mut()
            .find(|p| p.id == "cell-one-box")
            .unwrap()
            .agents
            .clear();
        let empty = convey_hop_coverage(&empty_box, "cell-one-box", None).unwrap_err();
        assert_eq!(empty.word, "deny-default");
        assert!(empty.line.contains("cell-one-box hop lane-tool: deny-default"));

        let mut allowed = raw.clone();
        allowed
            .agents
            .iter_mut()
            .find(|a| a.id == "research")
            .unwrap()
            .tools
            .push(crate::ToolDecl {
                id: "lane-tool".into(),
                description: None,
            });
        grant(&mut allowed, "research", IntentionKind::Tool, "lane-tool", Effect::Allow);
        let allow = convey_hop_coverage(&allowed, "cell-one-box", Some("research")).unwrap();
        let allow = allow.unwrap();
        assert_eq!(allow.word, "allow");
        assert_eq!(allow.capability, "lane-tool");
        let still = convey_hop_coverage(&allowed, "cell-one-box", None).unwrap_err();
        assert_eq!(still.word, "deny-default");

        grant(&mut allowed, "research", IntentionKind::Tool, "lane-tool", Effect::Deny);
        let explicit = convey_hop_coverage(&allowed, "cell-one-box", Some("research")).unwrap_err();
        assert_eq!(explicit.word, "deny");
        assert!(explicit.line.contains(": deny"));
        assert!(!explicit.line.contains("deny-default"));

        let horizon = convey_hop_coverage(&allowed, "cell-one-box", Some("horizon")).unwrap_err();
        assert_eq!(horizon.word, "deny-default");
        let outsider = convey_hop_coverage(&allowed, "cell-one-box", Some("not-placed")).unwrap_err();
        assert_eq!(outsider.word, "deny-default");
        assert!(
            outsider.line.contains("not on the hop population"),
            "{}",
            outsider.line
        );
        assert!(
            !outsider.line.contains("research"),
            "outsider must not inherit research deny, got {}",
            outsider.line
        );

        let cloud_agent = convey_hop_coverage(&allowed, "cursor-cloud", None).unwrap_err();
        assert_eq!(cloud_agent.word, "deny");

        let named_cloud = convey_hop_coverage(&raw, "cursor-cloud", Some("research")).unwrap_err();
        assert_eq!(named_cloud.word, "deny");
        assert!(named_cloud.line.contains("deny"));
        assert!(
            !named_cloud.line.contains("deny-default"),
            "{}",
            named_cloud.line
        );

        let off_box = convey_hop_coverage(&raw, "cell-one-box", Some("not-placed")).unwrap_err();
        assert_eq!(off_box.word, "deny-default");
        assert!(off_box.line.contains("not on the hop population"));

        assert!(convey_hop_coverage(&raw, "ttl-hop", None).unwrap().is_none());
    }

    #[test]
    fn convey_hop_declared_capability_matches_allow_and_refuses_mismatch() {
        let mut allowed = estate();
        allowed
            .agents
            .iter_mut()
            .find(|a| a.id == "research")
            .unwrap()
            .tools
            .push(crate::ToolDecl {
                id: "lane-tool".into(),
                description: None,
            });
        grant(
            &mut allowed,
            "research",
            IntentionKind::Tool,
            "lane-tool",
            Effect::Allow,
        );
        let matched = convey_hop_declared_capability(
            &allowed,
            "cell-one-box",
            Some("research"),
            "lane-tool",
        )
        .unwrap()
        .unwrap();
        assert_eq!(matched.word, "allow");
        assert_eq!(matched.capability, "lane-tool");

        let mismatch = convey_hop_declared_capability(
            &allowed,
            "cell-one-box",
            Some("research"),
            "notes-append",
        )
        .unwrap_err();
        assert_eq!(mismatch.word, "mismatch");
        assert_eq!(mismatch.capability, "lane-tool");
        assert!(
            mismatch.line.contains(
                "capability 'notes-append' does not match hop coverage capability 'lane-tool'"
            ),
            "{}",
            mismatch.line
        );

        let rows = hop_coverage_rows(&allowed);
        let cloud = rows.iter().find(|row| row.hop_id == "cursor-cloud").unwrap();
        assert_eq!(cloud.capability, "mesh-stub");
        let denied =
            convey_hop_declared_capability(&allowed, "cursor-cloud", None, "lane-tool").unwrap_err();
        assert_eq!(denied.word, "deny");
        assert!(!denied.line.contains("does not match"));

        let still =
            convey_hop_declared_capability(&allowed, "cell-one-box", None, "notes-append")
                .unwrap_err();
        assert_eq!(still.word, "deny-default");
        assert!(!still.line.contains("does not match"));

        assert!(convey_hop_declared_capability(&allowed, "ttl-hop", None, "other")
            .unwrap()
            .is_none());
    }

    #[test]
    fn convey_intention_coverage_refuses_deny_and_default_allows_only_allow() {
        let mut estate = estate();
        estate
            .agents
            .iter_mut()
            .find(|a| a.id == "research")
            .unwrap()
            .tools
            .push(crate::ToolDecl {
                id: "lane-tool".into(),
                description: None,
            });
        let missing = convey_intention_coverage(&estate, "cell-one-box", "research", "lane-tool", None, false)
            .unwrap_err();
        assert_eq!(missing.word, "deny-default");
        assert!(missing.line.contains("intention"));

        grant(&mut estate, "research", IntentionKind::Tool, "lane-tool", Effect::Deny);
        let denied = convey_intention_coverage(&estate, "cell-one-box", "research", "lane-tool", None, false)
            .unwrap_err();
        assert_eq!(denied.word, "deny");
        assert!(!denied.line.contains("deny-default"));

        estate.intentions.clear();
        grant(&mut estate, "research", IntentionKind::Tool, "lane-tool", Effect::Allow);
        assert!(convey_intention_coverage(&estate, "cell-one-box", "research", "lane-tool", None, false).is_ok());

        let cloud = convey_intention_coverage(&estate, "cursor-cloud", "research", "mesh-stub", None, false);
        assert!(cloud.is_ok(), "cloud deny stays the hop-coverage gate");
        let off = convey_intention_coverage(&estate, "ttl-box", "research", "not-a-tool", None, false).unwrap_err();
        assert_eq!(off.word, "deny-default");
        assert!(
            off.line.contains("undeclared for agent 'research' (deny-default)"),
            "{}",
            off.line
        );
    }

    #[test]
    fn box_deny_does_not_hide_unresolved_intention() {
        let mut estate = estate();
        let research = estate.agents.iter_mut().find(|a| a.id == "research").unwrap();
        research.tools.push(crate::ToolDecl {
            id: "lane-tool".into(),
            description: None,
        });
        research.mcp.push(crate::McpDecl {
            id: "notes-append".into(),
            description: None,
        });
        grant(&mut estate, "research", IntentionKind::Tool, "lane-tool", Effect::Deny);
        let hop = convey_hop_coverage(&estate, "cell-one-box", Some("research")).unwrap_err();
        assert_eq!(hop.word, "deny");

        let undeclared = convey_intention_coverage(
            &estate,
            "cell-one-box",
            "research",
            "not-a-tool",
            None,
            true,
        )
        .unwrap_err();
        assert_eq!(undeclared.word, "deny-default");
        assert!(
            undeclared
                .line
                .contains("undeclared for agent 'research' (deny-default)"),
            "{}",
            undeclared.line
        );

        let ambiguous = convey_intention_coverage(
            &estate,
            "cell-one-box",
            "research",
            "notes-append",
            None,
            true,
        )
        .unwrap_err();
        assert!(
            ambiguous
                .line
                .contains("ambiguous capability; pass --intention-kind"),
            "{}",
            ambiguous.line
        );
        assert!(
            !ambiguous.line.contains("refuse:hop-coverage"),
            "{}",
            ambiguous.line
        );
    }

    #[test]
    fn convey_intention_coverage_lane_prefix_is_memory_read() {
        let estate = estate();
        let own = convey_intention_coverage(&estate, "ttl-box", "horizon", "lane:horizon", None, false);
        assert!(own.is_ok(), "own-lane MemoryRead continues past the intention gate");

        let crossed =
            convey_intention_coverage(&estate, "ttl-box", "horizon", "lane:research", None, false)
                .unwrap_err();
        assert_eq!(crossed.word, "deny-default");
        assert!(
            crossed.line.contains("memory_read"),
            "lane: infers MemoryRead, got {}",
            crossed.line
        );
        assert!(
            !crossed.line.contains("missing intention"),
            "cross-lane is an authorize deny, got {}",
            crossed.line
        );

        let mut allowed = estate.clone();
        grant(
            &mut allowed,
            "horizon",
            IntentionKind::MemoryRead,
            "lane:research",
            Effect::Allow,
        );
        assert!(
            convey_intention_coverage(&allowed, "ttl-box", "horizon", "lane:research", None, false).is_ok(),
            "allow memory intention continues past the intention gate"
        );

        grant(
            &mut allowed,
            "horizon",
            IntentionKind::MemoryRead,
            "lane:research",
            Effect::Deny,
        );
        let denied =
            convey_intention_coverage(&allowed, "ttl-box", "horizon", "lane:research", None, false)
                .unwrap_err();
        assert_eq!(denied.word, "deny");
        assert!(denied.line.contains("memory_read"), "{}", denied.line);
    }

    #[test]
    fn convey_intention_coverage_multi_kind_names_ambiguous() {
        let mut estate = estate();
        let research = estate.agents.iter_mut().find(|a| a.id == "research").unwrap();
        research.mcp.push(crate::McpDecl {
            id: "notes-append".into(),
            description: None,
        });
        let ambiguous =
            convey_intention_coverage(&estate, "ttl-box", "research", "notes-append", None, false)
                .unwrap_err();
        assert_eq!(ambiguous.word, "deny-default");
        assert!(
            ambiguous.line.contains("ambiguous capability; pass kind"),
            "{}",
            ambiguous.line
        );
        assert!(
            !ambiguous.line.contains("missing intention"),
            "{}",
            ambiguous.line
        );
        assert!(
            convey_intention_coverage(
                &estate,
                "ttl-box",
                "research",
                "notes-append",
                Some(IntentionKind::Tool),
                false,
            )
            .is_err(),
            "passing kind still fail-closes without an allow intention"
        );

        let hop_ambiguous = convey_intention_coverage(
            &estate,
            "ttl-box",
            "research",
            "notes-append",
            None,
            true,
        )
        .unwrap_err();
        assert!(
            hop_ambiguous
                .line
                .contains("ambiguous capability; pass --intention-kind"),
            "{}",
            hop_ambiguous.line
        );
        grant(
            &mut estate,
            "research",
            IntentionKind::Tool,
            "notes-append",
            Effect::Allow,
        );
        assert!(
            convey_intention_coverage(
                &estate,
                "ttl-box",
                "research",
                "notes-append",
                Some(IntentionKind::Tool),
                true,
            )
            .is_ok(),
            "hop --intention-kind tool allows when that intention allows"
        );
        let mcp = convey_intention_coverage(
            &estate,
            "ttl-box",
            "research",
            "notes-append",
            Some(IntentionKind::Mcp),
            true,
        )
        .unwrap_err();
        assert_eq!(mcp.word, "deny-default");
        assert!(mcp.line.contains("mcp"), "{}", mcp.line);
    }

    #[test]
    fn coverage_for_agent_ignores_display_prefix() {
        let rows = vec![
            CoverageRow::fact("horizon", "not-the-id model class: allow"),
            CoverageRow::hop(
                String::new(),
                "cursor-cloud",
                "deny",
                "mesh-stub",
                "horizon hop mesh-stub: deny",
            ),
        ];
        let matched: Vec<_> = coverage_for_agent(&rows, "horizon").collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].line, "not-the-id model class: allow");
        assert!(coverage_for_agent(&rows, "research").next().is_none());
    }

    #[test]
    fn rust_classroom_not_an_agent() {
        let e = estate();
        assert!(e.agent("rust-classroom").is_none());
        assert!(e.agent("cyera-ci").is_none());
        assert!(e.is_sacred("rust-classroom"));
    }

    #[test]
    fn cross_lane_file_read_fails_closed() {
        let e = estate();
        let tmp = std::env::temp_dir().join(format!("cell-one-fw-{}", std::process::id()));
        let research = tmp.join("lanes/research");
        std::fs::create_dir_all(&research).unwrap();
        std::fs::write(research.join("secret.txt"), "research-only").unwrap();
        let err =
            read_lane_file(&e, &tmp, "horizon", "research", Path::new("secret.txt")).unwrap_err();
        assert!(err.reason.contains("cross-lane"));
        let own =
            read_lane_file(&e, &tmp, "research", "research", Path::new("secret.txt")).unwrap();
        assert_eq!(own, b"research-only");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

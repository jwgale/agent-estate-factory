use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "estate",
    about = "Cell One estate-control: validate, plan, apply, drift. Specialist complete is a thin HttpLocal delegate, not a gateway.",
    after_help = "Day-90 topics: estate help status | plan | apply | reconcile | feed-loop | backup | frontier | day90-mixed | north-star | charter | enrich | models\nEntrypoint: make gate-90   Live boxes: docs/DAY90-PLUS.md (parked, not green)",
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    /// Dual-layer sacred file. Missing = hardcoded defaults only.
    #[arg(long, global = true, default_value = "policy/sacred.yaml")]
    pub(crate) sacred: PathBuf,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Day-90 operator topic pages. `estate help status`.
    Help {
        /// Topic: status, plan, apply, reconcile, feed-loop, backup, frontier, day90-mixed, north-star, charter, models. Omit to list.
        #[arg(value_name = "TOPIC")]
        topic: Option<String>,
    },
    /// Fail closed if the estate file is invalid.
    Validate {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// Human-readable blast-radius plan; append-only write to plans/.
    /// A placement hop capability that disagrees with hop coverage is
    /// `refuse:hop-coverage` (mismatch) and fails this command. Deny and
    /// deny-default are cited and do not fail plan by themselves. The
    /// check does not write the mesh, the leases, or the estate. A present
    /// mesh file that does not parse fails with the mesh error and is not
    /// rewritten. A missing mesh is not a failure. After the cites, an
    /// Authority section prints the same file check as `estate convey authority`
    /// (`would-allow`, `would-deny`, `not-enforced`). A `not-enforced reasons:`
    /// line counts those rows by class and omits zeros. A missing
    /// conveyor-mesh.json cites that the file is absent. It does not claim mediation.
    /// Examples: `estate help plan`
    Plan {
        #[command(subcommand)]
        action: Option<PlanAction>,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        /// Previous estate YAML to diff against. Default: last apply snapshot, else greenfield.
        #[arg(long)]
        against: Option<PathBuf>,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Copy the new plan into plans/reviewed/ for a human PR.
        #[arg(long, default_value_t = false)]
        reviewed: bool,
        #[arg(long, default_value = "plans/reviewed")]
        reviewed_dir: PathBuf,
    },
    /// Converge isolation (Control→Data apply seam). Stretch: included, thin.
    /// A placement hop capability that disagrees with hop coverage is
    /// `refuse:hop-coverage` (mismatch) and fails this command. Deny and
    /// deny-default are cited and do not fail apply by themselves. The
    /// check does not write the mesh. A present mesh file that does not
    /// parse fails with the mesh error and is not rewritten. A missing
    /// mesh is not a failure. After the cites, an Authority section prints
    /// the same file check as `estate plan`, `estate drift`, and
    /// `estate convey authority` (`would-allow`, `would-deny`,
    /// `not-enforced`), including on `--dry-run`. A `not-enforced reasons:`
    /// line counts those rows by class and omits zeros. A missing
    /// conveyor-mesh.json cites that the file is absent. It does not claim
    /// mediation. `--dry-run` writes nothing.
    /// Examples: `estate help apply`
    Apply {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        /// Fail if no plan on disk covers this estate hash (gated apply).
        #[arg(long, default_value_t = false)]
        require_plan: bool,
        /// Explicit pack import during apply (never silent; does not edit estate.yaml).
        #[arg(long)]
        import_pack: Option<String>,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        /// Fail if covering plan against_hash does not match last apply.
        #[arg(long, default_value_t = false)]
        require_fresh_plan: bool,
        /// Print blast radius + reconcile preview. Does not write leases.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
        /// Import gate. Must match locked curator `jason`.
        #[arg(long, default_value = "jason")]
        curator: String,
        /// Reconverge when desired hash matches but actual drifted. Or re-bind when unchanged.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Compare desired estate to regenerable actual-state.
    /// A placement hop capability that disagrees with hop coverage is
    /// `refuse:hop-coverage` (mismatch) and fails this command. Deny and
    /// deny-default are cited and do not fail drift by themselves. The
    /// check does not write the mesh and does not change floor or models
    /// `in_sync`. A present mesh file that does not parse fails with the
    /// mesh error and is not rewritten. A missing mesh is not a failure.
    /// After the cites, an Authority section prints the same file check as
    /// `estate plan` and `estate convey authority` (`would-allow`,
    /// `would-deny`, `not-enforced`). A `not-enforced reasons:` line counts
    /// those rows by class and omits zeros. A missing conveyor-mesh.json cites
    /// that the file is absent. It does not claim mediation.
    Drift {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
    /// List equal-class bindings. Does not invoke them.
    /// After bindings, readiness, and per-binding ping lines, prints the
    /// shared honesty stack (`honesty_stack`): the same Agents section as
    /// status, doctor, reconcile, and `estate audits`
    /// (`describe_agents_section`), then hop coverage cites
    /// (`hop_coverage_cites` / `render_hop_coverage_cites`: `FAIL` on
    /// mismatch, `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`: `would-allow`,
    /// `would-deny`, `not-enforced`, and `not-enforced reasons:`). Those
    /// cites do not fail this command. A match stays quiet. A missing mesh
    /// is an empty cite list and stays not-enforced (`missing-mesh`) and
    /// cites that conveyor-mesh.json is absent. The shared stack reads
    /// placement-actual for mesh interpretation and Authority. A present
    /// mesh that does not parse, a bad host_class on that file,
    /// `refuse:agent-unplaced`, or a placement-actual parse failure, refuses
    /// before those sections. The models body is already printed. A
    /// placement-actual SKU host_class still continues: Agents and hop cites
    /// print and Authority rows are omitted. There is no later mesh reader
    /// after the stack. The command does not refuse after the stack. A
    /// missing or unreadable estate refuses before the models body. Does not
    /// spawn. Does not write the mesh, the leases, the estate, or the apply
    /// audit. Does not claim mediation. Examples: `estate help models`
    Models {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        /// Directory the honesty stack reads for mesh and placement-actual.
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Append-only plan history (human control surface).
    Plans {
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
    },
    /// Drop runtime; write durable lifecycle=suspended. Lane roots stay.
    Suspend {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Re-apply from estate files; lifecycle=running.
    Resume {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
    /// Persisted estate + durable lifecycle + disposable runtime.
    /// After the hop expired count and the cloud-agent line, prints the
    /// shared honesty stack (`honesty_stack`): the same Agents section as
    /// plan, drift, apply, doctor, and convey authority
    /// (`describe_agents_section`), then hop coverage cites
    /// (`render_hop_coverage_cites` / `hop_coverage_cites`: `FAIL` on
    /// mismatch, `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`:
    /// `would-allow`, `would-deny`, `not-enforced`). Those cites do not
    /// fail status. A match stays quiet. Deny and deny-default on the
    /// Agents section are notes and do not fail status. A
    /// `not-enforced reasons:` line counts those rows by class and omits
    /// zeros. It does not claim mediation. A missing mesh is an empty cite
    /// list and stays not-enforced (`missing-mesh`), cites that
    /// conveyor-mesh.json is absent, and adds no hop coverage cite. A mesh
    /// that does not parse, a bad host_class on that file, or
    /// `refuse:agent-unplaced` does not invent cites or Authority rows.
    /// Estate load, lifecycle parse,
    /// `refuse_lease_host_classes`, and model-actual stay before the page
    /// header. Catalog disagree and spawned cloud bail after that header
    /// has started and before Agents, before the honesty stack. A
    /// placement-actual SKU
    /// host_class does not reach the stack: `refuse_lease_host_classes`
    /// refuses before the page, and `list_expired_hop_leases` would also
    /// refuse that SKU (`load_interpreted_mesh` slim-parses
    /// placement-actual) before the page. The page does not print. There
    /// is no later mesh reader after the stack. The command does not
    /// succeed with Authority omitted. That diverges from `estate convey
    /// authority`, from `estate leases` and `estate convey sync`, and from
    /// `estate audits` and `estate history`. Does not write. Does not spawn.
    /// Examples: `estate help status`
    Status {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Feed plane: materialize packs, list drop zone, refuse promote.
    Feed {
        #[command(subcommand)]
        command: FeedCommand,
    },
    /// Dump portable local catalog (file SoT). Does not invoke models.
    Catalog {
        #[arg(long, default_value = ".cell/catalog.json")]
        out: PathBuf,
    },
    /// Print durable placement leases. Cloud-agent must stay unspawned.
    /// After the estate loads, prints the same Agents section as status,
    /// doctor, reconcile, and convey authority (`describe_agents_section`),
    /// then the same hop coverage cites (`hop_coverage_cites`: `FAIL` on
    /// mismatch, `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`), before the
    /// placement lease list. Those cites do not fail this command. A match
    /// stays quiet. A missing mesh is an empty cite list and stays
    /// not-enforced. A present mesh that does not parse, or a bad host_class
    /// on that file, refuses before those sections and before the lease list.
    /// A placement-actual SKU host_class still continues: Agents and hop cites
    /// print and Authority rows are omitted. The lease reader still refuses
    /// that SKU before the placement JSON. A mesh population ahead of
    /// placement-actual is `refuse:agent-unplaced` before those sections and
    /// before the lease list. A spawned cloud-agent lease still refuses
    /// before the JSON. Does not spawn. Does not write the mesh, the leases,
    /// the estate, or the apply audit. Does not claim mediation.
    Leases {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print apply-audit.jsonl (gated apply history).
    /// After the estate loads, prints the same Agents section as status,
    /// doctor, reconcile, and `estate leases` (`describe_agents_section`),
    /// then the same hop coverage cites (`hop_coverage_cites`: `FAIL` on
    /// mismatch, `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`: `would-allow`,
    /// `would-deny`, `not-enforced`, and `not-enforced reasons:`), before
    /// the apply-audit list. Those cites do not fail this command. A match
    /// stays quiet. A missing mesh is an empty cite list and stays
    /// not-enforced. The shared stack reads placement-actual for mesh
    /// interpretation and Authority. A present mesh that does not parse, a
    /// bad host_class on that file, `refuse:agent-unplaced`, or any other
    /// mesh error including a placement-actual parse failure, refuses before
    /// those sections and before the apply-audit list. A placement-actual SKU
    /// host_class still continues: Agents and hop cites print and Authority
    /// rows are omitted, then the apply-audit list still prints. There is no
    /// second placement refuse before that list. Does not spawn. Does not
    /// write the mesh, the leases, the estate, or the apply audit. Does not
    /// claim mediation.
    Audits {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Desired vs actual placement reconcile. Sacred-id deny stays.
    /// After the estate loads, prints the same Agents section as status,
    /// doctor, and convey authority (`describe_agents_section`), then the
    /// same hop coverage cites (`hop_coverage_cites`: `FAIL` on mismatch,
    /// `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`), before
    /// reconcile.json, a suggest patch, or this report. Those cites do not
    /// fail this command. A match stays quiet. A missing mesh is an empty
    /// cite list and stays not-enforced. A present mesh that does not parse,
    /// or a bad host_class on that file, refuses before those sections. A
    /// placement-actual SKU host_class still reaches the placement report.
    /// A mesh population ahead of placement-actual is `refuse:agent-unplaced`
    /// before those sections. Placement drift
    /// still fails closed. `--suggest` writes a patch file and does not
    /// rewrite leases. Does not spawn. Does not rewrite the estate or the mesh.
    /// Examples: `estate help reconcile`
    Reconcile {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Write a suggested patch file. Never auto-applies.
        #[arg(long, default_value_t = false)]
        suggest: bool,
    },
    /// Bundle local review artifacts. Not a remote upload.
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
    /// Print append-only lifecycle.jsonl (suspend/resume/apply).
    /// After the estate loads, prints the same Agents section as status,
    /// doctor, reconcile, and `estate audits` (`describe_agents_section`),
    /// then the same hop coverage cites (`hop_coverage_cites`: `FAIL` on
    /// mismatch, `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`: `would-allow`,
    /// `would-deny`, `not-enforced`, and `not-enforced reasons:`), before
    /// the lifecycle history list. Those cites do not fail this command. A match
    /// stays quiet. A missing mesh is an empty cite list and stays
    /// not-enforced. The shared stack reads placement-actual for mesh
    /// interpretation and Authority. A present mesh that does not parse, a
    /// bad host_class on that file, `refuse:agent-unplaced`, or any other
    /// mesh error including a placement-actual parse failure, refuses before
    /// those sections and before the lifecycle history list. A placement-actual SKU
    /// host_class still continues: Agents and hop cites print and Authority
    /// rows are omitted, then the history body still prints. There is no
    /// second placement refuse before that list. Does not spawn. Does not
    /// write the mesh, the leases, the estate, or the apply audit. Does not
    /// claim mediation.
    History {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Catalog-level driver probes. Does not invoke models.
    Probes {
        /// Optional live HTTP. SKIP when endpoints are unset. Never required in CI.
        #[arg(long, default_value_t = false)]
        live: bool,
    },
    /// Env-gated data-plane chat. Local: HttpLocal. Frontier: grok-4.7. Not a gateway.
    Specialist {
        /// Override. Local: CELL_LOCAL_ENDPOINT. Frontier: CELL_FRONTIER_ENDPOINT or https://api.x.ai/v1.
        #[arg(long)]
        endpoint: Option<String>,
        /// ollama | llama.cpp | http-remote | frontier (grok-4.7). Visible alias: --runtime.
        #[arg(long, visible_alias = "runtime", default_value = "ollama")]
        driver: String,
        /// complete (default) | chat | policy-precheck | redact
        #[arg(long, default_value = "complete")]
        job: String,
        #[arg(long, default_value = "cli")]
        agent: String,
        #[arg(long, default_value = "model")]
        kind: String,
        #[arg(long)]
        prompt: Option<String>,
        /// Alias for `--prompt`.
        #[arg(long)]
        text: Option<String>,
    },
    /// Lease-bound hop stub. Declare a hop, then a lease-bound call.
    Convey {
        #[command(subcommand)]
        command: ConveyCommand,
    },
    /// Intention check. Same `authorize` as `conveyor-proxy check`.
    /// After the estate loads, a bad `{state-dir}/decision-select.json` is
    /// `refuse:decision-select` before the check and before the receipt.
    /// A resolved allow or deny appends one versioned receipt at
    /// `{state-dir}/decisions/receipts.jsonl` (`cell-one.decision-receipt.v0`,
    /// `surface` `authorize`) before the allow or deny JSON. `hop_id` on
    /// that line is the intention kind and `capability` is the object.
    /// Eligible candidates are opaque model-binding ids. The selector
    /// chooses one id or abstains. Zero eligible ids and two or more
    /// eligible ids abstain, so frontier and local stay equal class. The
    /// host re-validates (`ok`, `stale`, `ineligible`, `expired`) and may
    /// record a fallback. The selector does not grant permission. Authorize
    /// still decides allow or deny. A fallback is recorded and is not
    /// applied. This path has no hop lease, so lease expiry stays on
    /// `estate convey call`. Success prints one `decision receipt:` cite.
    /// A journal write that fails after authorize has committed prints
    /// `decision receipt: journal write failed after authorize commit` and
    /// still prints the allow or deny JSON. The authorize exit stands.
    /// `{state-dir}/decision-select.json` is an optional hint and is not a
    /// grant. No honesty stack. No promote. No auto-apply. Does not spawn.
    Authorize {
        #[arg(long)]
        agent: String,
        /// tool | mcp | mount | model | memory_read | agent.
        /// `agent` is who-may-call-whom. Aliases: agent_call, agent-call.
        #[arg(long)]
        kind: String,
        #[arg(long)]
        object: String,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Optional proxy audit directory. A failed append refuses before the receipt.
        #[arg(long)]
        feed_dir: Option<PathBuf>,
    },
    /// Host → select → authorize → data-plane complete → receipt.
    /// After the estate loads, a bad `{state-dir}/decision-select.json` is
    /// `refuse:decision-select` before the check and before the receipt.
    /// The host prepares eligible model-binding ids. A selector chooses
    /// one id or abstains. Zero or two or more eligible ids abstain, so
    /// frontier and local stay equal class. Omit `--object` when that
    /// selector chooses one id with validation `ok`. Name `--object` to
    /// complete a specific binding; the selector still does not grant.
    /// Authorize still decides allow or deny. On allow, complete runs
    /// through the selected binding's local or frontier driver
    /// (`complete_via_binding`). Control does not invent the text.
    /// Missing `CELL_LOCAL_ENDPOINT` or `XAI_API_KEY` fail-closes.
    /// A resolved allow, deny, or fail-closed complete appends one
    /// versioned receipt (`cell-one.decision-receipt.v0`, `surface`
    /// `complete`) at `{state-dir}/decisions/receipts.jsonl`. `hop_id`
    /// is `model` and `capability` is the binding complete targeted.
    /// Success prints one `decision receipt:` cite, then the completion
    /// JSON. A journal write that fails after complete has committed
    /// prints `decision receipt: journal write failed after complete
    /// commit` and still prints the completion. `--mock` uses in-process
    /// drivers. `{state-dir}/decision-select.json` is an optional hint
    /// and is not a grant. No honesty stack. No hop lease. No promote.
    /// No auto-apply. Does not spawn. Does not invent a live PASS.
    Complete {
        #[arg(long)]
        agent: String,
        #[arg(long)]
        prompt: Option<String>,
        /// Alias for `--prompt`.
        #[arg(long)]
        text: Option<String>,
        /// Binding to complete. Omit when the selector chooses one eligible id.
        #[arg(long)]
        object: Option<String>,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Optional proxy audit directory. A failed append refuses before the receipt.
        #[arg(long)]
        feed_dir: Option<PathBuf>,
        /// Override. Local: CELL_LOCAL_ENDPOINT. Frontier: CELL_FRONTIER_ENDPOINT.
        #[arg(long)]
        endpoint: Option<String>,
        /// In-process MockLocal / MockFrontier. Not a live generate.
        #[arg(long, default_value_t = false)]
        mock: bool,
    },
    /// Decision journal written by `estate convey call`, `estate authorize`,
    /// and `estate complete`. `export` writes JSONL replay cases. `report`
    /// counts stage, validation, and fallback. Selectors do not grant
    /// permission. No promote. No auto-apply.
    Decisions {
        #[command(subcommand)]
        command: DecisionsCommand,
    },
    /// Pack curator path: list / import / refuse promote + INDEX.
    Packs {
        #[command(subcommand)]
        command: PacksCommand,
    },
    /// Prepare train/enrich artifacts. Does not train, promote, or rewrite the estate.
    Enrich {
        #[command(subcommand)]
        command: EnrichCommand,
    },
    /// one-letter one-letter classify loop. Prepare does not train. Eval does not record a live PASS.
    Classify {
        #[command(subcommand)]
        command: ClassifyCommand,
    },
    /// List expired placement leases. Apply/resume refuse them.
    /// After the estate loads, prints the same Agents section as status,
    /// doctor, reconcile, and `estate history` (`describe_agents_section`),
    /// then the same hop coverage cites (`hop_coverage_cites`: `FAIL` on
    /// mismatch, `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`: `would-allow`,
    /// `would-deny`, `not-enforced`, and `not-enforced reasons:`), before
    /// the expired placement lease list and before `--forget` writes. Those
    /// cites do not fail this command. A match stays quiet. A missing mesh
    /// is an empty cite list and stays not-enforced. The shared stack reads
    /// placement-actual for mesh interpretation and Authority. A present
    /// mesh that does not parse, a bad host_class on that file,
    /// `refuse:agent-unplaced`, or any other mesh error including a
    /// placement-actual parse failure, refuses before those sections, before
    /// the expired list, and before `--forget` writes. A placement-actual
    /// SKU host_class still continues: Agents and hop cites print and
    /// Authority rows are omitted, then the expired placement list still
    /// prints. There is no second placement refuse before that list, same
    /// as `estate audits` and `estate history`. `list_expired_leases` and
    /// `forget_expired_leases` load placement-actual with `load_placements`
    /// and do not call `load_interpreted_mesh`. `--forget` drops expired
    /// placement rows after that list prints. An empty expired list does
    /// not rewrite. An expired spawned cloud-agent lease still refuses
    /// before the list and before the rewrite. Without `--forget` this
    /// command does not write the mesh, the leases, the estate, or the
    /// apply audit. A missing or unreadable estate refuses before any
    /// section and before the list. Does not spawn. Does not claim
    /// mediation.
    Expire {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Drop expired rows so a later apply can record fresh leases. Does not spawn.
        #[arg(long, default_value_t = false)]
        forget: bool,
    },
    /// One-page health: .cell layout, schema files, compile-only CI present.
    /// After the hop describe lines, the same Agents section as plan, drift,
    /// apply, status, and convey authority (`describe_agents_section`) prints before hop
    /// coverage cites and an Authority section. That Authority section is
    /// `describe_authority_section` over `authority_report`, the same file
    /// check as `estate plan`, `estate drift`, `estate apply`,
    /// `estate status`, and `estate convey authority` (`would-allow`,
    /// `would-deny`, `not-enforced`). Deny and deny-default on the Agents
    /// section and on hop cites are notes and do not fail doctor, including
    /// `--strict` on the locked example. A `not-enforced reasons:` line
    /// counts those rows by class and omits zeros. It does not claim
    /// mediation. A missing conveyor-mesh.json cites that the file is
    /// absent. A hop-coverage mismatch still fails this command after those
    /// sections. A mesh that does not parse is FAIL and does not invent
    /// Agents or Authority rows. Does not write. Does not spawn.
    Doctor {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Pre-merge operator checks. Local only. No live Mac / GPU.
        #[arg(long, default_value_t = false)]
        strict: bool,
        /// Fail when a declared tool, MCP, mount, or model is deny-default.
        /// Not part of `--strict`. Not used by `make smoke` or `make gate-90`.
        #[arg(long, default_value_t = false)]
        strict_intentions: bool,
    },
    /// Append-only `.cell/sessions.jsonl` (spawn/unspawn/suspend/resume).
    Sessions {
        #[command(subcommand)]
        command: SessionsCommand,
    },
    /// Timestamped local archive of durable `.cell/` files.
    Backup {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = "backups")]
        out: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
        /// Keep only the newest N `cell-backup-*` dirs after writing. Omit = keep all.
        #[arg(long)]
        prune: Option<usize>,
    },
    /// Restore a cell archive. Refuses sacred mismatch. `--dry-run` writes nothing.
    Restore {
        #[arg(long)]
        from: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
    },
    /// Check a policy pack against a known action.
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
    /// Apply → suspend → drop sessions → resume. Leases stay on disk.
    PauseProof {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
}

#[derive(Subcommand)]
pub(crate) enum DecisionsCommand {
    /// Write JSONL replay cases from the decision journal. Missing journal writes an empty file.
    Export {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Replay JSONL path. Does not train, promote, or apply.
        #[arg(long)]
        out: PathBuf,
    },
    /// Print counts by stage, validation, and fallback.
    Report {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
}

#[derive(Subcommand)]
pub(crate) enum PolicyCommand {
    Check {
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
        #[arg(long, default_value = "apply")]
        action: String,
        #[arg(long)]
        hop: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum ConveyCommand {
    /// Declare a hop and write a durable lease.
    /// After intention, hop-coverage, and agent-unbound refuses, and after
    /// the estate loads, prints the same Agents section as status, doctor,
    /// reconcile, and `estate convey sync` (`describe_agents_section`), then
    /// the same hop coverage cites (`hop_coverage_cites`: `FAIL` on mismatch,
    /// `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`: `would-allow`,
    /// `would-deny`, `not-enforced`, and `not-enforced reasons:`), before
    /// `declare_hop_covering` writes the mesh and before the lease JSON.
    /// Those cites do not fail this command. A match stays quiet. A missing
    /// mesh is an empty cite list and stays not-enforced. The shared stack
    /// still reads placement-actual for mesh interpretation and Authority.
    /// A present mesh that does not parse, a bad host_class on that file,
    /// `refuse:agent-unplaced` on the mesh already on disk, or a
    /// placement-actual parse failure, refuses before those sections and
    /// before the write. A placement-actual SKU host_class still continues:
    /// Agents and hop cites print and Authority rows are omitted.
    /// `declare_hop_covering` still refuses that SKU before the lease JSON
    /// (`slim_parse_placement_actual` before `persist_mesh`). That diverges
    /// from `estate audits` and `estate history`, whose readers still print
    /// the body. Intention deny, hop-coverage deny, deny-default, and a
    /// capability mismatch, and `--intention-kind` without `--agent`, refuse
    /// before the stack and write nothing. An estate cloud-agent placement
    /// is that coverage deny. A hop id that is not an estate placement stays
    /// the lease stub. When placement-actual marks that id spawned cloud,
    /// declare refuses `refuse:cloud-spawned` after the stack and writes
    /// nothing. `refuse_hop` and a new hop ahead of the placement row refuse
    /// after the stack and write nothing. The mismatch inside declare is the
    /// same coverage gate and is not reached again. Does not spawn. Does not
    /// claim mediation.
    Hop {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "box")]
        kind: String,
        #[arg(long, default_value = "lane-tool")]
        capability: String,
        #[arg(long, default_value = "any")]
        host_class: String,
        #[arg(long, default_value_t = true)]
        wired: bool,
        #[arg(long)]
        ttl_secs: Option<u64>,
        /// Agent on this hop. Repeat for a population. Empty is not a grant.
        #[arg(long = "agent")]
        agents: Vec<String>,
        /// tool | mcp | mount | model | memory_read | agent. Disambiguates the intention.
        /// `agent` is who-may-call-whom (`agent_call` and `agent-call` are the same kind).
        /// Does not set hop `--kind` (the hop declaration kind).
        #[arg(long)]
        intention_kind: Option<String>,
        /// Estate used for intention and placement-derived hop coverage.
        /// A named agent refuses on intention deny or deny-default
        /// (`refuse:intention`). Allow continues to hop coverage, then
        /// refuses when `--capability` does not match that coverage capability.
        /// The hop write refuses that same mismatch and does not store the lease.
        /// Missing or not a file refuses. Hop deny and deny-default refuse.
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Lease-bound call. Refuses without a granted lease.
    /// After intention, hop-coverage, and agent-unbound refuses, and after
    /// the estate loads, prints the same Agents section as status, doctor,
    /// reconcile, and `estate convey hop` (`describe_agents_section`), then
    /// the same hop coverage cites (`hop_coverage_cites`: `FAIL` on mismatch,
    /// `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`: `would-allow`,
    /// `would-deny`, `not-enforced`, and `not-enforced reasons:`), before
    /// `call_hop` or `call_hop_for_agent` and before the call JSON.
    /// Those cites do not fail this command. A match stays quiet. A missing
    /// mesh is an empty cite list and stays not-enforced. The shared stack
    /// still reads placement-actual for mesh interpretation and Authority.
    /// A present mesh that does not parse, a bad host_class on that file,
    /// `refuse:agent-unplaced` on the mesh already on disk, or a
    /// placement-actual parse failure, refuses before those sections and
    /// before the call. A placement-actual SKU host_class still continues:
    /// Agents and hop cites print and Authority rows are omitted.
    /// `call_hop` and `call_hop_for_agent` still refuse that SKU before an
    /// allow and before any restamp (`load_interpreted_mesh` slim-parses
    /// placement-actual before `restamp_hop_from_decl`). That diverges from
    /// `estate audits` and `estate history`, whose readers still print the
    /// body. Intention deny, hop-coverage deny, deny-default, and a
    /// capability mismatch, a missing estate, and `--kind` without `--agent`,
    /// refuse before the stack. An estate cloud-agent placement is that
    /// coverage deny. A hop id that is not an estate placement stays the
    /// lease stub. A missing lease is `refuse:no-lease` after the stack and
    /// does not invent a mesh. A present hop decl with no lease restamps
    /// once after the stack. A populated lease with no `--agent`, or an
    /// agent the lease does not name, is `refuse:agent-unbound` after the
    /// stack and does not print the call JSON. This command does not add a
    /// second mesh write.
    /// Does not spawn. Does not apply. Does not claim mediation.
    /// `--agent` binds one placed agent. Not an IdP.
    /// After the stack, a placement-actual SKU host_class refuses before a
    /// decision receipt and before an allow. A mesh that does not parse and
    /// `refuse:agent-unplaced` already refused before the stack and write no
    /// receipt. A call that passes those refuses appends one versioned
    /// receipt at `{state-dir}/decisions/receipts.jsonl` before the allow or
    /// restamp JSON. Eligible candidates are opaque model-binding ids. The
    /// selector chooses one id or abstains. The host re-validates (`ok`,
    /// `stale`, `ineligible`, `expired`) and may record a fallback. The
    /// selector does not grant permission. Success prints one
    /// `decision receipt:` cite. A journal write that fails after the hop
    /// has already committed prints `decision receipt: journal write failed
    /// after hop commit` and still prints the allow or restamp JSON. The hop
    /// exit stands. Binding digests cover id, class, driver, and binding
    /// params, so param drift marks a hint stale. Unread `estate decisions
    /// export` does not fail the call. No promote. No auto-apply.
    Call {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "lane-tool")]
        capability: String,
        #[arg(long)]
        agent: Option<String>,
        /// tool | mcp | mount | model | memory_read | agent. Omit to infer from the agent.
        /// `agent` is who-may-call-whom. Aliases: agent_call, agent-call.
        #[arg(long)]
        kind: Option<String>,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
    },
    /// List declared hops.
    /// After the estate loads, prints the same Agents section as status,
    /// doctor, reconcile, and `estate history` (`describe_agents_section`),
    /// then the same hop coverage cites (`hop_coverage_cites`: `FAIL` on
    /// mismatch, `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`: `would-allow`,
    /// `would-deny`, `not-enforced`, and `not-enforced reasons:`), before
    /// the hop decl list. Those cites do not fail this command. A match
    /// stays quiet. A missing mesh is an empty cite list and stays
    /// not-enforced. The shared stack reads placement-actual for mesh
    /// interpretation and Authority. A present mesh that does not parse, a
    /// bad host_class on that file, `refuse:agent-unplaced`, or any other
    /// mesh error including a placement-actual parse failure, refuses before
    /// those sections and before the hop list. A placement-actual SKU
    /// host_class still continues: Agents and hop cites print and Authority
    /// rows are omitted. The hop list reader still refuses that SKU before
    /// the hop JSON (`list_hops` loads the interpreted mesh). That diverges
    /// from `estate audits` and `estate history`, whose readers still print
    /// the body. Does not spawn. Does not write the mesh, the leases, the
    /// estate, or the apply audit. Does not claim mediation.
    List {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print hop leases. Cloud-mesh must stay unspawned.
    /// After the estate loads, prints the same Agents section as status,
    /// doctor, reconcile, and `estate leases` (`describe_agents_section`),
    /// then the same hop coverage cites (`hop_coverage_cites`: `FAIL` on
    /// mismatch, `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`: `would-allow`,
    /// `would-deny`, `not-enforced`, and `not-enforced reasons:`), before
    /// the hop lease list. Those cites do not fail this command. A match
    /// stays quiet. A missing mesh is an empty cite list and stays
    /// not-enforced. A present mesh that does not parse, or a bad host_class
    /// on that file, refuses before those sections and before the hop lease
    /// JSON. A placement-actual SKU host_class still continues: Agents and
    /// hop cites print and Authority rows are omitted. The hop lease reader
    /// still refuses that SKU before the hop lease JSON. A mesh population
    /// ahead of placement-actual is `refuse:agent-unplaced` before those
    /// sections and before the hop lease JSON. A spawned cloud hop still
    /// refuses before the JSON. Does not spawn. Does not write the mesh, the
    /// leases, the estate, or the apply audit. Does not claim mediation.
    Leases {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Derive hops from placement-actual.json (slim parse).
    /// When `--estate` is a file, loads that estate and prints the same
    /// Agents section as status, doctor, reconcile, and `estate convey expire`
    /// (`describe_agents_section`), then the same hop coverage cites
    /// (`hop_coverage_cites`: `FAIL` on mismatch, `note` on deny and
    /// deny-default), then Authority (`describe_authority_section` over
    /// `authority_report`: `would-allow`, `would-deny`, `not-enforced`, and
    /// `not-enforced reasons:`), before the mesh write and before the mesh
    /// JSON. Those cites do not fail this command. A match stays quiet. A
    /// missing mesh is an empty cite list and stays not-enforced. The shared
    /// stack still reads placement-actual for mesh interpretation and
    /// Authority. A present mesh that does not parse, a bad host_class on
    /// that file, `refuse:agent-unplaced`, or any other mesh error including
    /// a placement-actual parse failure, refuses before those sections and
    /// before the mesh write. A placement-actual SKU host_class still
    /// continues: Agents and hop cites print and Authority rows are omitted.
    /// Sync still refuses that SKU before the mesh JSON
    /// (`sync_from_placements_covering` slim-parses placement-actual before
    /// it writes). That diverges from `estate audits` and `estate history`,
    /// whose readers still print the body. A placement hop whose stamped
    /// capability disagrees with placement-derived coverage (`lane-tool` on
    /// box, `mesh-stub` on cloud) is `refuse:hop-coverage` after the stack
    /// and writes nothing. A spawned cloud placement still refuses after the
    /// stack and writes nothing. A missing estate file keeps the lease stub
    /// sync and does not invent this stack. A hop id that is not a placement
    /// stays the lease stub. Does not spawn. Does not claim mediation.
    Sync {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Estate used for the placement-derived capability check.
        /// A missing path keeps the lease stub sync (no new refuse).
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// File check of hop leases against the estate. After the estate loads,
    /// prints the same Agents section as plan, drift, apply, status, and
    /// doctor (`describe_agents_section`), then the same hop coverage cites
    /// (`hop_coverage_cites`: `FAIL` on mismatch, `note` on deny and
    /// deny-default), then Authority (`describe_authority_section` over
    /// `authority_report`: `would-allow`, `would-deny`, `not-enforced`, and
    /// `not-enforced reasons:`). Those cites do not fail this command. A
    /// match stays quiet. A missing mesh is an empty cite list and stays
    /// not-enforced. The shared stack reads placement-actual for mesh
    /// interpretation and Authority. A present mesh that does not parse, a
    /// bad host_class on that file, `refuse:agent-unplaced`, or any other
    /// mesh error including a placement-actual parse failure, refuses before
    /// those sections. A placement-actual SKU host_class still continues:
    /// Agents and hop cites print and Authority rows are omitted, and this
    /// file check succeeds. There is no later reader. That diverges from
    /// `estate convey list`, `estate convey leases`, `estate convey expire`,
    /// `estate convey sync`, `estate convey hop`, and `estate convey call`,
    /// whose readers still refuse that SKU after the stack. Deny and
    /// deny-default on the Agents text are notes and do not fail this
    /// command. A would-deny row does not fail this command.
    /// A missing conveyor-mesh.json stays not-enforced and cites that the
    /// file is absent. A present mesh with no lease for a declared
    /// capability says no hop lease names it. Does not write. Does not
    /// spawn. Does not claim mediation. A missing or unreadable estate
    /// refuses before any section. `conveyor-proxy authority` does not call
    /// this stack: `render_hop_coverage_cites` lives in estate-control. The
    /// proxy still refuses a mesh that does not parse, a bad host_class on
    /// that file, `refuse:agent-unplaced`, and a placement-actual parse
    /// failure before Agents. A placement-actual SKU omits Authority and the
    /// proxy command succeeds, without hop cites.
    Authority {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// List expired hop leases. Call refuses them. `--forget` drops leases; hop decls stay so call can restamp.
    /// After the estate loads, prints the same Agents section as status,
    /// doctor, reconcile, and `estate convey list` (`describe_agents_section`),
    /// then the same hop coverage cites (`hop_coverage_cites`: `FAIL` on
    /// mismatch, `note` on deny and deny-default), then Authority
    /// (`describe_authority_section` over `authority_report`: `would-allow`,
    /// `would-deny`, `not-enforced`, and `not-enforced reasons:`), before
    /// the expired hop lease list and before `--forget` writes. Those cites
    /// do not fail this command. A match stays quiet. A missing mesh is an
    /// empty cite list and stays not-enforced. The shared stack still reads
    /// placement-actual for mesh interpretation and Authority. A present mesh
    /// that does not parse, a bad host_class on that file,
    /// `refuse:agent-unplaced`, or any other mesh error including a
    /// placement-actual parse failure, refuses before those sections, before
    /// the expired list, and before `--forget` writes. A placement-actual SKU
    /// host_class still continues: Agents and hop cites print and Authority
    /// rows are omitted. The expired-lease reader still refuses that SKU
    /// before the expired list (`list_expired_hop_leases` loads the
    /// interpreted mesh) and before `--forget` rewrites the mesh. That
    /// diverges from `estate audits` and `estate history`, whose readers
    /// still print the body. An expired spawned cloud hop still refuses
    /// before the list and before the rewrite. Without `--forget` this
    /// command does not write the mesh, the leases, the estate, or the apply
    /// audit. Does not spawn. Does not claim mediation.
    Expire {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value_t = false)]
        forget: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum PlanAction {
    /// Compare two plans or last-applied vs new. Exit 1 if blast grows.
    Diff {
        /// Previous plan JSON or estate YAML.
        #[arg(long)]
        from: Option<PathBuf>,
        /// New plan JSON or estate YAML.
        #[arg(long)]
        to: Option<PathBuf>,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        /// Allow a wider blast radius (otherwise refuse:wider).
        #[arg(long, default_value_t = false)]
        allow_wider: bool,
    },
    /// Write a single markdown ready to paste into a GitHub PR body.
    ExportPr {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = "plans/reviewed")]
        reviewed_dir: PathBuf,
        #[arg(long, default_value = "plans/PR.md")]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
pub(crate) enum SessionsCommand {
    /// Print the full session journal.
    List {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print the last N journal events.
    Tail {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value_t = 20)]
        n: usize,
    },
}

#[derive(Subcommand)]
pub(crate) enum PacksCommand {
    /// List candidate packs and rewrite INDEX.md.
    List {
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
    },
    /// Explicit apply of a pack artifact. Does not rewrite the estate file.
    Import {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "packs/accepted")]
        accepted_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = "jason")]
        curator: String,
    },
    /// Always fails. Auto-promote is locked off.
    Promote {
        #[arg(long)]
        id: String,
    },
    /// Rewrite packs/INDEX.md.
    Index {
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
    },
    /// Write a proposal pack. Never auto-applies. Jason reviews the diff.
    Propose {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "packs/accepted")]
        accepted_dir: PathBuf,
        #[arg(long, default_value = "packs/proposed")]
        proposed_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// Copy a proposal into enrich_packs edit instructions. Does not rewrite the estate.
    Accept {
        #[arg(long)]
        id: String,
        #[arg(long)]
        curator: String,
        #[arg(long, default_value = "packs/proposed")]
        proposed_dir: PathBuf,
        #[arg(long, default_value = "packs/accepted")]
        accepted_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
}

#[derive(Subcommand)]
pub(crate) enum ClassifyCommand {
    /// Validate one-letter JSONL, split it, and write a one-letter LLaMA-Factory dataset plus held-out JSONL.
    /// Offline. Does not train.
    Prepare {
        /// JSONL of `{state, question, options, answer}` records. Option labels are consecutive letters from A.
        #[arg(long)]
        input: PathBuf,
        /// Directory for `dataset.jsonl`, `dataset_info.json`, `heldout.jsonl`, and `prepare.json`.
        #[arg(long, default_value = ".cell/classify")]
        out: PathBuf,
        /// Seed for the Fisher–Yates split. Same seed and file split the same way.
        #[arg(long, default_value_t = 20_260_920)]
        seed: u64,
        /// Fraction of question groups held out for eval. Greater than 0 and less than 1.
        #[arg(long, default_value_t = 0.2)]
        held_out_ratio: f64,
        /// LLaMA-Factory dataset shape. The assistant target is exactly one letter.
        #[arg(long, value_enum, default_value_t = crate::classify::DatasetFormat::Sharegpt)]
        format: crate::classify::DatasetFormat,
        /// Key written into `dataset_info.json`.
        #[arg(long, default_value = "classify_decisions")]
        dataset_name: String,
        /// Fail on any bad row and write nothing. Omit to skip bad rows and print the count plus the first line numbers.
        #[arg(long, default_value_t = false)]
        strict: bool,
        /// Overwrite files in a non-empty `--out` directory. Without this, a non-empty `--out` is refused.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Download a public Hugging Face classification set and write qwen JSONL.
    /// This slice downloads ag_news, devign (CodeXGLUE defect detection), and rust_idiom (CommitPackFT Rust before/after).
    /// Sampled rows land in `.cell/classify-import/<alias>-<train-size>-s<seed>/`.
    /// ag_news options stay in class-table order (A=World, B=Sports, C=Business, D=Sci/Tech).
    /// devign options stay in class-table order (A=Secure, B=Insecure). Held-out is the official test split.
    /// rust_idiom options stay in class-table order (A=NeedsFix, B=Idiomatic). Held-out is a seeded commit holdout, not an official test split.
    /// Output is for local training only. Do not redistribute.
    Import {
        /// `ag_news`, `fancyzhx/ag_news`, `devign`, `google/code_x_glue_cc_defect_detection`, `rust_idiom`, or `bigcode/commitpackft`. banking77 and multi_nli are catalog rows for a later slice.
        #[arg(long)]
        dataset: String,
        /// Class-balanced train rows for ag_news and devign. rust_idiom samples whole A/B commit pairs; an odd count is refused. `all` keeps the split. No upper cap.
        #[arg(long, default_value = "all")]
        train_size: String,
        /// Held-out rows from the official test split, or whole commits from the rust_idiom holdout. An odd rust_idiom count is refused. `all` is 7600 for ag_news, 2732 for devign, and 936 for rust_idiom.
        #[arg(long, default_value = "all")]
        heldout_size: String,
        /// Sample seed. Default 42.
        #[arg(long, default_value_t = crate::classify_import::DEFAULT_IMPORT_SEED)]
        seed: u64,
        /// Directory for sampled `train.jsonl`, `heldout.jsonl`, and `import.json`.
        /// Default: `.cell/classify-import/<dataset>-<train-size>-s<seed>/`.
        /// The full native download stays in `.cell/classify-import/<dataset>/native/`.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Download again even when `native/` is already present.
        #[arg(long, default_value_t = false)]
        force: bool,
        /// Offline native JSONL (`index`, `text`, `label`). Skips the network. Pair with `--native-test`.
        #[arg(long)]
        native_train: Option<PathBuf>,
        /// Offline native JSONL for the official test split.
        #[arg(long)]
        native_test: Option<PathBuf>,
        /// Local HF dataset snapshot (`**/train-*.parquet` and `**/test-*.parquet`). Not modified.
        #[arg(long)]
        from_local: Option<PathBuf>,
        /// `bulk` (default) runs `hf download --repo-type dataset`, then reads parquet.
        /// `rows-api` pages the datasets-server rows API.
        #[arg(long, value_enum, default_value_t = crate::classify_import::ImportFetch::Bulk)]
        fetch: crate::classify_import::ImportFetch,
        /// Python with pyarrow. Unset uses `ESTATE_PYTHON`, then `python3`.
        #[arg(long)]
        python: Option<String>,
    },
    /// Score held-out JSONL against an OpenAI-compatible chat endpoint (Ollama `/v1` or a hosted endpoint).
    /// Temperature 0, small max_tokens, thinking off where the body supports it. Does not record a live PASS.
    Eval {
        /// Held-out JSONL from `classify prepare` (state, question, options, answer letter).
        #[arg(long)]
        records: PathBuf,
        /// Base URL. `http://127.0.0.1:11434` and `https://api.together.ai` both gain `/v1/chat/completions` when needed.
        #[arg(long)]
        endpoint: Option<String>,
        /// Model id sent in the chat body. An Ollama tag or a hosted model name.
        #[arg(long)]
        model: String,
        /// Environment variable that holds the bearer token. The value is never printed. Omit for a local endpoint with no key.
        #[arg(long)]
        api_key_env: Option<String>,
        /// JSON report path. Default: `classify-report.json` beside the records file.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Validate and write a sample request. Does not call the network.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Score with a built-in letter script. Does not call the network. Not a model score.
        #[arg(long, default_value_t = false)]
        mock: bool,
        /// Per-request timeout in seconds.
        #[arg(long, default_value_t = 30)]
        timeout_secs: u64,
        /// Request shape. `openai` sets enable_thinking false. `ollama` sets reasoning_effort none on `/v1`. `ollama-native` posts `/api/chat` with think false.
        #[arg(long, value_enum, default_value_t = crate::classify::EvalApi::Openai)]
        api: crate::classify::EvalApi,
        /// Prepend this many labeled train exemplars to each held-out prompt. Omit for zero-shot. `0` is refused.
        #[arg(long)]
        few_shot: Option<u32>,
        /// Exemplar JSONL for `--few-shot`. qwen records (`train.jsonl`) or the prepared train file (`dataset.jsonl`, sharegpt or alpaca).
        #[arg(long)]
        exemplars: Option<PathBuf>,
        /// Seed for the exemplar shuffle. Same seed and file pick the same order. Used only with `--few-shot`.
        #[arg(long, default_value_t = 20_260_920)]
        seed: u64,
    },
    /// Letter journey: prepare, LoRA YAML, train, merge, GGUF, Ollama seat, base-vs-specialist eval.
    /// `--preset qwen` (default) is Qwen/Qwen3.5-4B. `--preset deepseek-r1-distill` is DeepSeek-R1-Distill-Qwen-1.5B with template `deepseekr1` and local `llamafactory-cli` train. `--preset glm4-chat` is GLM-4-9B-Chat with template `glm4` and local `llamafactory-cli` train.
    /// `--print` is the default and does not run tools or call the network. `--train-driver local` (default) uses llamafactory-cli. `--train-driver together` uploads the prepared dataset and launches a LoRA job. On the DeepSeek and GLM-4 Chat presets, Together needs `--together-model`. `--run` with together reads `TOGETHER_API_KEY` or `--api-key-env` and never prints the secret.
    /// `--run` downloads a Hub base once into `--base-cache` (default `.cell/classify-base-cache/<safe-id>/`) with `hf`, falling back to `huggingface-cli` only when `hf` is absent. A later `--out` reuses that snapshot. A local `--base` directory is used as-is. `--run` then refuses when llamafactory-cli, llama.cpp convert, ollama, or a GPU is missing.
    /// The comparison file is local output. It does not record a live PASS. `READY_FOR_LIVE_TEST` stays no.
    /// After compare, a successful run prints `estate enrich import-trained` for the specialist GGUF (`trained_shape` gguf, `auto_apply=false`), then Standing next (estate): `apply-proposal`, `plan`, `apply --require-plan`, and `reconcile`. It does not execute them. `--print` prints those lines as planned steps and does not write a proposal. `--import-trained` records the proposal only on `--run` when that GGUF is a regular file and `--estate`, `--prepared`, and `--enrich-tag` are set. A missing specialist GGUF refuses or skips the handoff and does not invent a proposal. The factory does not claim it trained.
    /// The proposal is one local specialty seat (`class` local). `--binding-id` names that seat. The default stays `local_slm`. A new id is added beside existing local seats. An existing local id is replaced in place. `--dataset ag_news` names the function `ag_news`. Frontier bindings stay peers. Other local specialty bindings stay beside it. Equal-class frontier and local stays. `make ag-news-journey` prints `--dataset ag_news` at `--train-size` 3000.
    /// `--dual` plans or runs `--preset qwen` (Qwen/Qwen3.5-4B) and `--preset glm4-chat` (zai-org/glm-4-9b-chat) on one `classify expand` rust_idiom cache (`--dataset rust_idiom --expand-tag`). Both students use that cache's held-out file. Outs are `{out}/qwen` and `{out}/glm4-chat` (the default `--out` gains a `-dual` suffix). The compare file is `dual-compare.json`. `--print` writes both journey plans and a compare stub and does not call the network. `--run` runs each existing journey. DeepSeek is refused. This is not a factory live PASS.
    /// `--modest` is only valid with `--dual`. It replaces the `--train-size all` default with `500` (`parse_split_size` count) and writes LLaMA-Factory `max_steps` 50 on both students when `--max-steps` is omitted. An explicit `--train-size` above 500 is refused. This is not a factory live PASS.
    Journey {
        /// `qwen` keeps Qwen/Qwen3.5-4B and tag `classify-specialist`. `deepseek-r1-distill` uses DeepSeek-R1-Distill-Qwen-1.5B, template `deepseekr1`, and tag `deepseek-r1-distill-specialist` unless `--base` or `--tag` is set to something else. `glm4-chat` uses `zai-org/glm-4-9b-chat`, template `glm4`, and tag `glm4-chat-specialist` unless `--base` or `--tag` is set to something else.
        #[arg(long, value_enum, default_value_t = crate::classify_journey::JourneyPreset::Qwen)]
        preset: crate::classify_journey::JourneyPreset,
        /// one-letter JSONL. Default: `examples/fixtures/classify-decisions.jsonl`.
        #[arg(long)]
        input: Option<PathBuf>,
        /// Journey directory. Prepare output, recipe, adapter, export, GGUF, and reports live here.
        #[arg(long, default_value = ".cell/classify-journey")]
        out: PathBuf,
        /// Hugging Face train base. Default `Qwen/Qwen3.5-4B` (LLaMA-Factory template `qwen3_5`).
        /// A Hub id is downloaded once into `--base-cache` (default `.cell/classify-base-cache/<safe-id>/`) and converted from that directory. Later journeys with other `--out` directories reuse that snapshot. A local weights directory is used as-is and is not copied into the cache.
        #[arg(long, default_value = crate::classify_journey::DEFAULT_BASE)]
        base: String,
        /// Shared directory for Hub base snapshots. Default `.cell/classify-base-cache`. Each Hub id gets one subdirectory. Ignored when `--base` is a local directory.
        #[arg(long, default_value = crate::classify_journey::DEFAULT_BASE_CACHE)]
        base_cache: PathBuf,
        /// Opt-in Ollama library tag for the base eval. Omit to build the base with the same convert, quant, and Modelfile as the specialist. A set tag can differ in precision.
        #[arg(long)]
        base_tag: Option<String>,
        /// Ollama tag created from the specialist GGUF.
        #[arg(long, default_value = crate::classify_journey::DEFAULT_TAG)]
        tag: String,
        /// OpenAI-compatible base URL. Ollama is `http://127.0.0.1:11434`.
        #[arg(long, default_value = "http://127.0.0.1:11434")]
        endpoint: String,
        /// Dataset key. Must match `dataset_info.json` from classify prepare.
        #[arg(long, default_value = crate::classify_journey::DEFAULT_DATASET)]
        dataset_name: String,
        #[arg(long, default_value_t = 20_260_920)]
        seed: u64,
        #[arg(long, default_value_t = 0.2)]
        held_out_ratio: f64,
        /// Optional LLaMA-Factory `max_steps`. Omit for one epoch.
        #[arg(long)]
        max_steps: Option<u32>,
        /// GGUF quant applied to the base and the specialist. `f16` skips llama-quantize.
        #[arg(long, default_value = crate::classify_journey::DEFAULT_QUANT)]
        quant: String,
        /// llama.cpp checkout. `$dir/convert_hf_to_gguf.py` and llama-quantize must exist. Env `LLAMA_CPP_DIR` is the fallback.
        #[arg(long)]
        llama_cpp_dir: Option<PathBuf>,
        /// Print the step plan. This is the default. Does not train.
        #[arg(long, default_value_t = false)]
        print: bool,
        /// Run the steps. Skips a step only when its manifest matches the current inputs.
        /// Eval and Ollama skips also include the specialist tag, endpoint, and base tag. The recipe skip includes `max_steps`, the dataset name, and the template.
        #[arg(long, default_value_t = false)]
        run: bool,
        /// Replace a file `--out` and a non-empty prepare directory.
        #[arg(long, default_value_t = false)]
        force: bool,
        /// Specialist accuracy minus base accuracy must reach this value. Local exit code only.
        #[arg(long)]
        min_delta: Option<f64>,
        /// Specialist accuracy must reach this value. Local exit code only.
        #[arg(long)]
        min_accuracy: Option<f64>,
        /// Fail unless the Newcombe 95% CI for (specialist accuracy − base accuracy) has a lower bound strictly greater than 0.
        /// Combines with `--min-delta` and `--min-accuracy`: every set threshold must hold. Default off. Local exit code only. Does not record a live PASS.
        #[arg(long, default_value_t = false)]
        require_significant_lift: bool,
        /// Per-request HTTP timeout in seconds. Together job polling uses `--together-poll-secs` instead.
        #[arg(long, default_value_t = 120)]
        timeout_secs: u64,
        /// Wall-clock deadline in seconds for Together job polling. Default 10800. Separate from `--timeout-secs`.
        #[arg(long, default_value_t = crate::classify_journey::DEFAULT_TOGETHER_POLL_SECS)]
        together_poll_secs: u64,
        /// Train path. `local` (default) runs `llamafactory-cli train`. `together` uploads the prepared dataset and launches a LoRA job on Together.
        /// `--print` never calls the network. `--run` reads the key from `--api-key-env` (default `TOGETHER_API_KEY`) and never prints the value.
        #[arg(long, value_enum, default_value_t = crate::classify_journey::TrainDriver::Local)]
        train_driver: crate::classify_journey::TrainDriver,
        /// Together base model id. Default `Qwen/Qwen3.5-4B` on `--preset qwen`. The DeepSeek and GLM-4 Chat presets refuse Together unless this is set.
        #[arg(long)]
        together_model: Option<String>,
        /// Together API root. Used only with `--train-driver together` and `--run`.
        #[arg(long, default_value = crate::classify_journey::DEFAULT_TOGETHER_API)]
        together_base_url: String,
        /// Environment variable that holds the Together API key. Default `TOGETHER_API_KEY`. The value is never printed.
        #[arg(long)]
        api_key_env: Option<String>,
        /// Public set to import instead of the built-in fixture. `ag_news` downloads fancyzhx/ag_news. `devign` downloads google/code_x_glue_cc_defect_detection. `rust_idiom` downloads the Rust subset of bigcode/commitpackft.
        /// Import already holds out the official test split for ag_news and devign. rust_idiom holds out a seeded commit split (seed 42), not an official test split. Prepare does not split again.
        /// The default tag and `--out` gain a suffix such as `-agnews-3000` so sizes can coexist.
        #[arg(long)]
        dataset: Option<String>,
        /// Class-balanced train rows for `--dataset` ag_news and devign. rust_idiom samples whole commit pairs; an odd count is refused. `all` keeps the split.
        #[arg(long, default_value = "all")]
        train_size: String,
        /// Held-out rows for `--dataset`, from the official test split, or whole commits from the rust_idiom holdout. An odd rust_idiom count is refused.
        #[arg(long, default_value = "all")]
        heldout_size: String,
        /// Local HF dataset snapshot. Same as `classify import --from-local`. Not modified.
        #[arg(long)]
        from_local: Option<PathBuf>,
        /// `bulk` (default) or `rows-api`. Same as `classify import --fetch`.
        #[arg(long, value_enum, default_value_t = crate::classify_import::ImportFetch::Bulk)]
        fetch: crate::classify_import::ImportFetch,
        /// Python with pyarrow. Unset uses `ESTATE_PYTHON`, then `python3`.
        #[arg(long)]
        python: Option<String>,
        /// Score the base a second time with N labeled exemplars from `dataset.jsonl` in `--out`.
        /// `0` (default) keeps the base eval zero-shot. The shuffle seed is `--seed`.
        /// The report is `base-few-shot-report.json`. It does not record a live PASS.
        #[arg(long, default_value_t = 0)]
        few_shot: u32,
        /// Read a `classify expand` cache (`.cell/classify-import/rust_idiom-<train-size>-s<seed>-<tag>/`) instead of importing again.
        /// The specialist tag and `--out` gain `-<tag>` when they are still the defaults. rust_idiom only. Does not call the teacher.
        #[arg(long)]
        expand_tag: Option<String>,
        /// Train and evaluate qwen (Qwen/Qwen3.5-4B) and glm4-chat (zai-org/glm-4-9b-chat) on the same rust_idiom `--expand-tag` cache.
        /// Requires `--dataset rust_idiom` and `--expand-tag`. Writes `{out}/qwen`, `{out}/glm4-chat`, and `{out}/dual-compare.json`.
        /// `--print` (default) writes both plans and a compare stub and does not use the network. `--run` executes both journeys.
        /// The compare file is local output. It is not a factory live PASS. `READY_FOR_LIVE_TEST` stays no.
        #[arg(long, default_value_t = false)]
        dual: bool,
        /// Short dual gauge for one 5090-class proof. Only valid with `--dual`.
        /// Replaces `--train-size all` (the clap default) with `500`. An explicit `--train-size` at or under 500 is kept. A count above 500 is refused.
        /// Omitting `--max-steps` writes LLaMA-Factory `max_steps` 50 on both qwen and glm4-chat. An explicit `--max-steps` is kept.
        /// `dual-compare.json` records `"modest": true` plus `train_size` and `max_steps`. `--print` does not train. Not a factory live PASS. `READY_FOR_LIVE_TEST` stays no.
        #[arg(long, default_value_t = false)]
        modest: bool,
        /// Estate file named on the import-trained handoff. Omit to print `<estate.yaml>`.
        /// `--import-trained` reads this file and does not rewrite it.
        #[arg(long)]
        estate: Option<PathBuf>,
        /// Enrich prepare directory (`prepare.json`, job train) for the import-trained handoff.
        /// Omit to print `<prepared>`.
        #[arg(long)]
        prepared: Option<PathBuf>,
        /// Tag for import-trained. Must be `cell-enrich-{pack_id}` when `--import-trained` records.
        /// Omit to print `cell-enrich-<pack-id>`.
        #[arg(long)]
        enrich_tag: Option<String>,
        /// After a successful `--run`, record the specialist GGUF with `estate enrich import-trained`.
        /// `trained_shape` is `gguf`. The proposal stays `auto_apply=false`. Does not apply the estate and does not promote.
        /// `--print` prints the same line and Standing next and does not write a proposal.
        /// Needs `--estate`, `--prepared`, and `--enrich-tag` when recording.
        /// A missing specialist GGUF refuses and does not invent a proposal.
        /// `--dual` prints each student's handoff and does not record a proposal.
        #[arg(long, default_value_t = false)]
        import_trained: bool,
        /// Portable local binding id on the import-trained handoff. Omit to keep `local_slm`.
        /// A new id is added as class local beside existing local seats, including `local_slm`.
        /// An existing local id is replaced in place. A hardware SKU is refused.
        /// The proposal stays `auto_apply=false`. This command does not apply the estate.
        #[arg(long)]
        binding_id: Option<String>,
    },
    /// Grow the rust_idiom FixedClasses curriculum with an OpenAI-compatible coding teacher.
    /// `--print` is the default. It writes `expand-plan.json` and does not call the network.
    /// `--run` reads `TEACHER_API_KEY` or `--api-key-env` (for example `OPENAI_API_KEY`) and never prints the value.
    /// Inputs are an existing rust_idiom train JSONL or import cache, and an optional raw Rust snippet JSONL (`--from-local`).
    /// Output is qwen A=NeedsFix, B=Idiomatic JSONL in `.cell/classify-import/rust_idiom-<train-size>-s<seed>-<tag>/`.
    /// Held-out commits stay out of the expanded train. `classify journey --dataset rust_idiom --expand-tag <tag>` trains that cache and does not split again.
    /// Parquet is refused. This command does not train. Does not record a live PASS. `READY_FOR_LIVE_TEST` stays no.
    Expand {
        /// `rust_idiom` or `bigcode/commitpackft`.
        #[arg(long, default_value = "rust_idiom")]
        dataset: String,
        /// qwen train JSONL, or an import cache directory that contains `train.jsonl` and `heldout.jsonl`.
        #[arg(long)]
        train: Option<PathBuf>,
        /// Held-out qwen JSONL. Required when `--train` is a file. A cache directory supplies its own `heldout.jsonl`.
        #[arg(long)]
        heldout: Option<PathBuf>,
        /// Raw Rust snippet JSONL (`snippet`, `text`, `code`, or `old_contents`). The teacher writes before/after pairs. Not modified. Parquet is refused.
        #[arg(long)]
        from_local: Option<PathBuf>,
        /// Cache directory. Default: `.cell/classify-import/rust_idiom-<train-size>-s<seed>-<tag>/`.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Suffix on the cache directory and on the journey tag. Letters, digits, `-`, or `_`.
        #[arg(long)]
        tag: String,
        /// Train-size token recorded on the cache. Same token `classify journey --train-size` uses. Default `all`.
        #[arg(long, default_value = "all")]
        train_size: String,
        /// Names the expand cache directory, same token as the sampled import path. Default 42.
        /// The report `holdout_seed` is read from the source import or native manifest, not from this flag.
        #[arg(long, default_value_t = crate::classify_import::DEFAULT_IMPORT_SEED)]
        seed: u64,
        /// Write `expand-plan.json` only. This is the default. Does not call the teacher.
        #[arg(long, default_value_t = false)]
        print: bool,
        /// Call the teacher and write `train.jsonl`, a byte copy of the held-out file, and `expand-report.json`.
        #[arg(long, default_value_t = false)]
        run: bool,
        /// OpenAI-compatible base URL. `/v1/chat/completions` is added when it is missing. Used only with `--run`.
        #[arg(long)]
        endpoint: Option<String>,
        /// Model id sent in the chat body. Used only with `--run`.
        #[arg(long)]
        model: Option<String>,
        /// Environment variable that holds the teacher API key. Default `TEACHER_API_KEY`. `OPENAI_API_KEY` is the other common name. The value is never printed.
        #[arg(long)]
        api_key_env: Option<String>,
        /// Per-request HTTP timeout in seconds.
        #[arg(long, default_value_t = 60)]
        timeout_secs: u64,
    },
    /// Compile-and-test grader for MultiPL-E Rust and HumanEvalPack Rust.
    /// Generation tasks are not FixedClasses A/B. `--print` is the default and does not compile or download.
    /// `--run` reads `--from-local` or `--tasks` and scores one completion per task with `rustc` or `cargo test`.
    /// `humanevalpack_rust` is `bigcode/humanevalpack` (MIT, from HumanEval MIT). `multiple_rust` is `nuprl/MultiPL-E` humaneval-rs (BSD-3-Clause translations of HumanEval MIT, not MBPP).
    /// Output is for local training proof only. Do not redistribute. Does not record a live PASS. `READY_FOR_LIVE_TEST` stays no.
    Grade {
        /// `humanevalpack_rust`, `bigcode/humanevalpack`, `multiple_rust`, or `nuprl/MultiPL-E`.
        #[arg(long)]
        dataset: Option<String>,
        /// Local JSONL snapshot, or a directory of JSONL files. Not modified. Parquet is refused.
        #[arg(long)]
        from_local: Option<PathBuf>,
        /// Already materialized task JSONL (`id`, `prompt`, `tests`). Not a letter decision file.
        #[arg(long)]
        tasks: Option<PathBuf>,
        /// Completion JSONL (`id`, `completion`). One row per task.
        #[arg(long)]
        completions: Option<PathBuf>,
        /// Directory for `grade-plan.json` or `grade-report.json` and `tasks.jsonl`.
        #[arg(long, default_value = crate::classify_grade::DEFAULT_GRADE_OUT)]
        out: PathBuf,
        /// Write the plan only. This is the default. Does not compile.
        #[arg(long, default_value_t = false)]
        print: bool,
        /// Load tasks and score completions. Does not download.
        #[arg(long, default_value_t = false)]
        run: bool,
        /// Score a stable prefix of task ids after sorting. Not a random sample. Omit for every task.
        #[arg(long)]
        limit: Option<usize>,
        /// Per-task compile and run timeout in seconds.
        #[arg(long, default_value_t = 30)]
        timeout_secs: u64,
        /// Score the canonical completion stored on each task. Still not a live PASS.
        #[arg(long, default_value_t = false)]
        use_canonical: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum EnrichCommand {
    /// Write artifacts for a seated runtime or a portable manifest.
    Prepare {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        /// Pack id, or a path to a pack JSON file.
        #[arg(long)]
        pack: PathBuf,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        /// TrainEnrichDriver id. Omit for the first catalog card. Not with --all-drivers.
        #[arg(long)]
        driver: Option<String>,
        /// Prepare every registered driver into sibling directories.
        #[arg(long, default_value_t = false)]
        all_drivers: bool,
        /// Final directory. Default: `{state_dir}/enrich/{pack_id}/{driver}`.
        /// With --all-drivers, each driver writes to `{out}/{driver}`.
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// `enrich` or `train`. Omit for the driver default (`train` on llamafactory-lora, llamafactory-qlora, axolotl-lora, axolotl-qlora, unsloth-qlora, and mlx-lm-lora, `enrich` otherwise). `--all-drivers` defaults to enrich. mlx-lm-lora is included in `--all-drivers` only when host_class_affinity is apple-silicon.
        #[arg(long)]
        job: Option<String>,
        /// Import gate. Must match locked curator `jason`.
        #[arg(long, default_value = "jason")]
        curator: String,
        /// Short gauge run. Writes `max_steps` into the LLaMA-Factory recipe and the Axolotl yaml. unsloth-qlora and mlx-lm-lora do not write max_steps. Omit for the one-epoch recipe. `--max-steps 0` is refuse:max-steps.
        #[arg(long)]
        max_steps: Option<u32>,
        /// Official SFT scale on LLaMA-Factory cards only (`examples/train_lora/qwen3_lora_sft.yaml`: cutoff_len 2048, num_train_epochs 3.0, gradient_accumulation_steps 8, warmup_ratio 0.1). Axolotl stays on its example files. unsloth-qlora and mlx-lm-lora do not implement it (`refuse:official-scale` when that card is alone). Omit for the short recipe. `--max-steps` still overrides epochs.
        #[arg(long, default_value_t = false)]
        official_scale: bool,
        /// Copy instruct rows from pack source_paths under --state-dir into dataset.jsonl on the train recipe cards. Omit to keep the scaffold. unsloth-qlora or mlx-lm-lora alone is refuse:dataset. Does not download.
        #[arg(long, default_value_t = false)]
        from_feed: bool,
    },
    /// Prepare an accepted pack into `{state_dir}/enrich`. Same refuses as prepare. Does not apply.
    FromPack {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        /// Pack id, or a path to a pack JSON file. An id is read from the drop or `accepted/`.
        #[arg(long)]
        pack: PathBuf,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        /// One TrainEnrichDriver id. Omit to prepare every card. Not with --all-drivers.
        #[arg(long)]
        driver: Option<String>,
        /// Prepare every registered driver. This is the default when --driver is omitted.
        #[arg(long, default_value_t = false)]
        all_drivers: bool,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// `enrich` or `train`. Omit for the driver default (`train` on llamafactory-lora, llamafactory-qlora, axolotl-lora, axolotl-qlora, unsloth-qlora, and mlx-lm-lora, `enrich` otherwise). With no `--driver`, the default job is enrich. mlx-lm-lora is included only when host_class_affinity is apple-silicon.
        #[arg(long)]
        job: Option<String>,
        /// Import gate. Must match locked curator `jason`.
        #[arg(long, default_value = "jason")]
        curator: String,
        /// Short gauge run. Writes `max_steps` into the LLaMA-Factory recipe and the Axolotl yaml. unsloth-qlora and mlx-lm-lora do not write max_steps. Omit for the one-epoch recipe. `--max-steps 0` is refuse:max-steps.
        #[arg(long)]
        max_steps: Option<u32>,
        /// Official SFT scale on LLaMA-Factory cards only (`examples/train_lora/qwen3_lora_sft.yaml`: cutoff_len 2048, num_train_epochs 3.0, gradient_accumulation_steps 8, warmup_ratio 0.1). Axolotl stays on its example files. unsloth-qlora and mlx-lm-lora do not implement it (`refuse:official-scale` when that card is alone). Omit for the short recipe. `--max-steps` still overrides epochs.
        #[arg(long, default_value_t = false)]
        official_scale: bool,
        /// Copy instruct rows from pack source_paths under --state-dir into dataset.jsonl on the train recipe cards. Omit to keep the scaffold. unsloth-qlora or mlx-lm-lora alone is refuse:dataset. Does not download.
        #[arg(long, default_value_t = false)]
        from_feed: bool,
    },
    /// List prepared packs under `{state_dir}/enrich`. Does not create the directory.
    List {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Validate a prepare dir and write a local_slm binding proposal. Does not apply.
    ImportPrepared {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        /// Directory that holds prepare.json.
        #[arg(long)]
        prepared: PathBuf,
        /// Local tag created outside the factory. Must be `cell-enrich-{pack_id}`.
        #[arg(long)]
        tag: String,
        /// File the operator loaded (Modelfile or returned weights). Must exist.
        #[arg(long)]
        path: PathBuf,
        /// Import gate. Must match locked curator `jason`.
        #[arg(long, default_value = "jason")]
        curator: String,
    },
    /// Record a trained adapter dir, merged export dir, or GGUF on a local seat proposal. Does not apply.
    /// `--binding-id` defaults to `local_slm`. A new id is added as class local beside existing local seats.
    /// An existing local id is replaced in place.
    ImportTrained {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        /// Directory that holds a llamafactory-lora, llamafactory-qlora, axolotl-lora, axolotl-qlora, unsloth-qlora, or mlx-lm-lora prepare.json with job train.
        #[arg(long)]
        prepared: PathBuf,
        /// Local tag created outside the factory. Must be `cell-enrich-{pack_id}`.
        #[arg(long)]
        tag: String,
        /// Adapter output_dir (adapter_config.json), merged export_dir (config.json and a non-adapter .safetensors file, optional Modelfile), or one .gguf file (a directory must hold exactly one).
        #[arg(long)]
        adapter: PathBuf,
        /// Portable local binding id. Omit to keep `local_slm`.
        /// A new id is added as class local beside existing local seats, including `local_slm`.
        /// An existing local id is replaced in place. A hardware SKU is refused.
        /// The proposal stays `auto_apply=false`. This command does not apply.
        #[arg(long)]
        binding_id: Option<String>,
        /// Import gate. Must match locked curator `jason`.
        #[arg(long, default_value = "jason")]
        curator: String,
    },
    /// Stage a binding proposal for estate plan and estate apply --require-plan.
    /// Does not apply. Does not rewrite the source estate.
    ApplyProposal {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        /// Directory that holds binding-proposal.json and prepare.json.
        #[arg(long)]
        prepared: PathBuf,
        /// Must match the proposal tag `cell-enrich-{pack_id}`.
        #[arg(long)]
        tag: String,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Printed on the next-step plan/apply lines. The stage is not a plan.
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        /// Import gate. Must match locked curator `jason`.
        #[arg(long, default_value = "jason")]
        curator: String,
        /// Opt-in. Probe the seated runtime for `cell-enrich-{pack}` before staging.
        /// Default off. Refuses when the tag is missing or the runtime is down.
        #[arg(long, default_value_t = false)]
        verify_local_tag: bool,
    },
    /// Print the external adapter merge into a Hugging Face directory.
    /// Axolotl prints `axolotl merge-lora`. A LLaMA-Factory adapter prints
    /// `llamafactory-cli export` for export.yaml.
    /// unsloth-qlora prints `save_pretrained_merged` with `save_method` `merged_16bit`.
    /// Does not merge, does not shell out, and does not promote.
    MergeAdapt {
        /// Directory that holds an axolotl-lora, axolotl-qlora, llamafactory-lora, llamafactory-qlora, unsloth-qlora, or mlx-lm-lora prepare.json.
        #[arg(long)]
        prepared: PathBuf,
        /// Adapter output_dir (`adapter_config.json`). A merged Hugging Face directory or a GGUF is refuse:adapter. unsloth-qlora also needs adapter_model.safetensors or adapter_model.bin.
        #[arg(long)]
        adapter: PathBuf,
    },
    /// Print the llama.cpp convert_hf_to_gguf.py line for a merged export.
    /// Does not convert, does not shell out, and does not promote.
    GgufConvert {
        /// Directory that holds a llamafactory-lora, llamafactory-qlora, axolotl-lora, axolotl-qlora, or unsloth-qlora prepare.json.
        #[arg(long)]
        prepared: PathBuf,
        /// Merged export directory (config.json and a .safetensors file whose name does not start with adapter_model).
        #[arg(long)]
        weights: PathBuf,
    },
    /// Print the seat line for a merged export, a GGUF, or an adapter directory.
    /// Ollama is the default print. A GGUF also prints llama-cli and llama-server.
    /// Does not create, does not shell out, and does not promote.
    LocalSeat {
        /// Directory that holds a llamafactory-lora, llamafactory-qlora, axolotl-lora, axolotl-qlora, unsloth-qlora, or mlx-lm-lora prepare.json.
        #[arg(long)]
        prepared: PathBuf,
        /// Merged export directory (config.json and .safetensors, optional Modelfile) or a .gguf file.
        /// An adapter directory is refuse:seat. Not with --adapter.
        #[arg(long, conflicts_with = "adapter", required_unless_present = "adapter")]
        weights: Option<PathBuf>,
        /// Adapter output_dir (adapter_config.json, the same marker import-trained accepts).
        /// Prints a Modelfile whose FROM is prepare.json seat_tag and whose ADAPTER is this directory.
        /// A merged export or a GGUF is refuse:adapter. unsloth-qlora and mlx-lm-lora are refuse:adapter. Not with --weights.
        #[arg(long, conflicts_with = "weights", required_unless_present = "weights")]
        adapter: Option<PathBuf>,
        /// Printed local-run software. ollama (default) keeps ollama create and, for a GGUF, also prints llama-cli and llama-server. llama.cpp selects those GGUF lines. A merged directory still points at gguf-convert first. --adapter with llama.cpp is refuse:runtime.
        #[arg(long, default_value = "ollama")]
        runtime: String,
    },
    /// List registered TrainEnrichDriver cards. Does not prepare.
    Drivers,
}

#[derive(Subcommand)]
pub(crate) enum AuditCommand {
    /// Write a reviewable folder (and optional tarball) of local audit files.
    /// After the estate loads, and before the copy and tar finish, writes
    /// `honesty.md` next to `MANIFEST.md`. The manifest names that file.
    /// The snapshot is the same Agents section (`describe_agents_section`),
    /// hop coverage cites (`hop_coverage_cites`: `FAIL` on mismatch, `note`
    /// on deny and deny-default), and Authority
    /// (`describe_authority_section` over `authority_report`) that status,
    /// doctor, reconcile, and convey authority print. Those cites do not
    /// fail this command. A match stays quiet. A missing mesh is an empty
    /// cite list and stays not-enforced. `conveyor-mesh.json` is copied
    /// when present. A present mesh that does not parse, or a bad
    /// host_class on that file, refuses before the snapshot and before the
    /// out dir is wiped. A placement-actual SKU host_class still exports;
    /// `honesty.md` keeps Agents and hop cites and omits Authority rows.
    /// A mesh population ahead of placement-actual is `refuse:agent-unplaced`
    /// before the snapshot and before the out dir is written. Does not
    /// spawn. Does not rewrite leases, the estate, the apply audit, or the
    /// live mesh beyond the reconcile files this command already writes.
    Export {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "plans")]
        plans_dir: PathBuf,
        #[arg(long, default_value = "packs")]
        packs_dir: PathBuf,
        #[arg(long, default_value = ".cell/audit-export")]
        out: PathBuf,
        /// Also write `{out}.tar.gz` when `tar` is on PATH.
        #[arg(long, default_value_t = false)]
        tar: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum FeedCommand {
    /// Read scrubbed events and write a candidate pack (not in the estate).
    Pack {
        #[arg(long, default_value = ".cell/feed")]
        feed_dir: PathBuf,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "overnight-traces")]
        id: String,
    },
    /// List candidate packs in the drop zone.
    List {
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
    },
    /// Explicit apply of a pack artifact. Does not rewrite the estate file.
    Import {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "packs")]
        drop_dir: PathBuf,
        #[arg(long, default_value = "packs/accepted")]
        accepted_dir: PathBuf,
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = "jason")]
        curator: String,
    },
    /// Always fails. Auto-promote is locked off.
    Promote {
        #[arg(long)]
        id: String,
    },
    /// Print durable feed-cursor.json watermark.
    Cursor {
        #[arg(long, default_value = ".cell/feed")]
        feed_dir: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    #[test]
    fn call_help_refuses_capability_mismatch_after_allow() {
        let cmd = Cli::command();
        let help = cmd
            .find_subcommand("convey")
            .unwrap()
            .find_subcommand("call")
            .unwrap()
            .clone()
            .render_help()
            .to_string();
        assert!(
            help.contains("Allow continues to hop coverage, then")
                && help.contains("does not match that coverage capability"),
            "{help}"
        );
    }
}

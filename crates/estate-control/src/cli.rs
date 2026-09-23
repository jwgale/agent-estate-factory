use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "estate",
    about = "Cell One estate-control: validate, plan, apply, drift. Specialist complete is a thin HttpLocal delegate, not a gateway.",
    after_help = "Day-90 topics: estate help status | plan | apply | reconcile | feed-loop | backup | frontier | day90-mixed | north-star | charter | enrich\nEntrypoint: make gate-90   Live boxes: docs/DAY90-PLUS.md (parked, not green)",
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
        /// Topic: status, plan, apply, reconcile, feed-loop, backup, frontier, day90-mixed, north-star, charter. Omit to list.
        #[arg(value_name = "TOPIC")]
        topic: Option<String>,
    },
    /// Fail closed if the estate file is invalid.
    Validate {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
    },
    /// Human-readable blast-radius plan; append-only write to plans/.
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
    Drift {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = ".")]
        roots_base: PathBuf,
    },
    /// List equal-class bindings. Does not invoke them.
    Models {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
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
    Leases {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print apply-audit.jsonl (gated apply history).
    Audits {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Desired vs actual placement reconcile. Sacred-id deny stays.
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
    History {
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
    /// List expired placement leases. Apply/resume refuse them.
    Expire {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Drop expired rows so a later apply can record fresh leases. Does not spawn.
        #[arg(long, default_value_t = false)]
        forget: bool,
    },
    /// One-page health: .cell layout, schema files, compile-only CI present.
    Doctor {
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        /// Pre-merge operator checks. Local only. No live Mac / GPU.
        #[arg(long, default_value_t = false)]
        strict: bool,
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
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Lease-bound call. Refuses without a granted lease.
    Call {
        #[arg(long)]
        id: String,
        #[arg(long, default_value = "lane-tool")]
        capability: String,
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
        #[arg(long, default_value = "policy/cell-one.policy.v0.yaml")]
        policy: PathBuf,
    },
    /// List declared hops.
    List {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Print hop leases. Cloud-mesh must stay unspawned.
    Leases {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// Derive hops from placement-actual.json (slim parse).
    Sync {
        #[arg(long, default_value = ".cell")]
        state_dir: PathBuf,
    },
    /// List expired hop leases. Call refuses them. `--forget` drops leases; hop decls stay so call can restamp.
    Expire {
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
        /// `enrich` or `train`. Omit for the driver default (`train` on llamafactory-lora, llamafactory-qlora, axolotl-lora, and axolotl-qlora, `enrich` otherwise). `--all-drivers` defaults to enrich.
        #[arg(long)]
        job: Option<String>,
        /// Import gate. Must match locked curator `jason`.
        #[arg(long, default_value = "jason")]
        curator: String,
        /// Short gauge run. Writes `max_steps` into the LLaMA-Factory recipe and the Axolotl yaml. Omit for the one-epoch recipe.
        #[arg(long)]
        max_steps: Option<u32>,
        /// Official SFT scale on LLaMA-Factory cards only (`examples/train_lora/qwen3_lora_sft.yaml`: cutoff_len 2048, num_train_epochs 3.0, gradient_accumulation_steps 8, warmup_ratio 0.1). Axolotl stays on its example files. Omit for the short recipe. `--max-steps` still overrides epochs.
        #[arg(long, default_value_t = false)]
        official_scale: bool,
        /// Copy instruct rows from pack source_paths under --state-dir into dataset.jsonl. Omit to keep the scaffold. Does not download.
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
        /// `enrich` or `train`. Omit for the driver default (`train` on llamafactory-lora, llamafactory-qlora, axolotl-lora, and axolotl-qlora, `enrich` otherwise). With no `--driver`, the default job is enrich.
        #[arg(long)]
        job: Option<String>,
        /// Import gate. Must match locked curator `jason`.
        #[arg(long, default_value = "jason")]
        curator: String,
        /// Short gauge run. Writes `max_steps` into the LLaMA-Factory recipe and the Axolotl yaml. Omit for the one-epoch recipe.
        #[arg(long)]
        max_steps: Option<u32>,
        /// Official SFT scale on LLaMA-Factory cards only (`examples/train_lora/qwen3_lora_sft.yaml`: cutoff_len 2048, num_train_epochs 3.0, gradient_accumulation_steps 8, warmup_ratio 0.1). Axolotl stays on its example files. Omit for the short recipe. `--max-steps` still overrides epochs.
        #[arg(long, default_value_t = false)]
        official_scale: bool,
        /// Copy instruct rows from pack source_paths under --state-dir into dataset.jsonl. Omit to keep the scaffold. Does not download.
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
    /// Record a trained adapter dir, merged export dir, or GGUF on the local_slm proposal. Does not apply.
    ImportTrained {
        #[arg(long, default_value = "examples/estate.yaml")]
        estate: PathBuf,
        /// Directory that holds a llamafactory-lora, llamafactory-qlora, axolotl-lora, or axolotl-qlora prepare.json with job train.
        #[arg(long)]
        prepared: PathBuf,
        /// Local tag created outside the factory. Must be `cell-enrich-{pack_id}`.
        #[arg(long)]
        tag: String,
        /// Adapter output_dir (adapter_config.json), merged export_dir (config.json and a non-adapter .safetensors file, optional Modelfile), or one .gguf file (a directory must hold exactly one).
        #[arg(long)]
        adapter: PathBuf,
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
    /// Validate a merged export directory or a GGUF and print the ollama create line.
    /// Does not create, does not shell out, and does not promote.
    LocalSeat {
        /// Directory that holds a llamafactory-lora or llamafactory-qlora prepare.json.
        #[arg(long)]
        prepared: PathBuf,
        /// Merged export directory (config.json and .safetensors, optional Modelfile) or a .gguf file.
        #[arg(long)]
        weights: PathBuf,
    },
    /// List registered TrainEnrichDriver cards. Does not prepare.
    Drivers,
}

#[derive(Subcommand)]
pub(crate) enum AuditCommand {
    /// Write a reviewable folder (and optional tarball) of local audit files.
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

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
    /// tev1-style one-letter classify loop. Prepare does not train. Eval does not record a live PASS.
    Classify {
        #[command(subcommand)]
        command: ClassifyCommand,
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
pub(crate) enum ClassifyCommand {
    /// Validate tev1-style JSONL, split it, and write a one-letter LLaMA-Factory dataset plus held-out JSONL.
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
        #[arg(long, default_value = "tev1_decisions")]
        dataset_name: String,
        /// Fail on any bad row and write nothing. Omit to skip bad rows and print the count plus the first line numbers.
        #[arg(long, default_value_t = false)]
        strict: bool,
        /// Overwrite files in a non-empty `--out` directory. Without this, a non-empty `--out` is refused.
        #[arg(long, default_value_t = false)]
        force: bool,
    },
    /// Download a public Hugging Face classification set and write tev1 JSONL.
    /// Sampled rows land in `.cell/classify-import/<alias>-<train-size>-s<seed>/`.
    /// ag_news options stay in class-table order (A=World, B=Sports, C=Business, D=Sci/Tech).
    /// The license is unspecified on the Hub card. Output is for local training only. Do not redistribute.
    Import {
        /// `ag_news` or `fancyzhx/ag_news`. banking77 and multi_nli are catalog rows for a later slice.
        #[arg(long)]
        dataset: String,
        /// Class-balanced train rows. `all` keeps the official train split. No upper cap.
        #[arg(long, default_value = "all")]
        train_size: String,
        /// Held-out rows drawn only from the official test split. `all` is 7600 for ag_news.
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
    },
    /// Letter journey: prepare, LoRA YAML, train, merge, GGUF, Ollama seat, base-vs-specialist eval.
    /// `--preset tev1` (default) is Qwen/Qwen3.5-4B. `--preset deepseek-r1-distill` is DeepSeek-R1-Distill-Qwen-1.5B with template `deepseekr1` and local `llamafactory-cli` train.
    /// `--print` is the default and does not run tools or call the network. `--train-driver local` (default) uses llamafactory-cli. `--train-driver together` uploads the prepared dataset and launches a LoRA job. On the DeepSeek preset, Together needs `--together-model`. `--run` with together reads `TOGETHER_API_KEY` or `--api-key-env` and never prints the secret.
    /// `--run` downloads the base with `hf`, falling back to `huggingface-cli` only when `hf` is absent, unless `--base` is a local directory. It then refuses when llamafactory-cli, llama.cpp convert, ollama, or a GPU is missing.
    /// The comparison file is local output. It does not record a live PASS. `READY_FOR_LIVE_TEST` stays no.
    Journey {
        /// `tev1` keeps Qwen/Qwen3.5-4B and tag `tev1-specialist`. `deepseek-r1-distill` uses DeepSeek-R1-Distill-Qwen-1.5B, template `deepseekr1`, and tag `deepseek-r1-distill-specialist` unless `--base` or `--tag` is set to something else.
        #[arg(long, value_enum, default_value_t = crate::classify_journey::JourneyPreset::Tev1)]
        preset: crate::classify_journey::JourneyPreset,
        /// tev1-style JSONL. Default: `examples/fixtures/tev1-decisions.jsonl`.
        #[arg(long)]
        input: Option<PathBuf>,
        /// Journey directory. Prepare output, recipe, adapter, export, GGUF, and reports live here.
        #[arg(long, default_value = ".cell/classify-journey")]
        out: PathBuf,
        /// Hugging Face train base. Default `Qwen/Qwen3.5-4B` (LLaMA-Factory template `qwen3_5`).
        /// A Hub id is downloaded into `base-hf` and converted from that directory. A local weights directory is used as-is.
        #[arg(long, default_value = crate::classify_journey::DEFAULT_BASE)]
        base: String,
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
        /// Together base model id. Default `Qwen/Qwen3.5-4B` on `--preset tev1`. The DeepSeek preset refuses Together unless this is set.
        #[arg(long)]
        together_model: Option<String>,
        /// Together API root. Used only with `--train-driver together` and `--run`.
        #[arg(long, default_value = crate::classify_journey::DEFAULT_TOGETHER_API)]
        together_base_url: String,
        /// Environment variable that holds the Together API key. Default `TOGETHER_API_KEY`. The value is never printed.
        #[arg(long)]
        api_key_env: Option<String>,
        /// Public set to import instead of the built-in fixture. `ag_news` downloads fancyzhx/ag_news.
        /// Import already holds out the official test split. Prepare does not split again.
        /// The default tag and `--out` gain a suffix such as `-agnews-3000` so sizes can coexist.
        #[arg(long)]
        dataset: Option<String>,
        /// Class-balanced train rows for `--dataset`. `all` keeps the official train split.
        #[arg(long, default_value = "all")]
        train_size: String,
        /// Held-out rows for `--dataset`, drawn only from the official test split.
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
    /// Record a trained adapter dir, merged export dir, or GGUF on the local_slm proposal. Does not apply.
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

//! Day-90 operator topic pages. `estate help [topic]`.
//! Does not apply, spawn, or promote. Local only.

use anyhow::{bail, Result};

const TOPICS: &[&str] = &[
    "status",
    "plan",
    "apply",
    "reconcile",
    "feed-loop",
    "backup",
    "frontier",
    "day90-mixed",
    "north-star",
    "charter",
    "enrich",
];

pub(crate) fn cmd_help(topic: Option<&str>) -> Result<()> {
    match topic.map(str::trim).filter(|t| !t.is_empty()) {
        None => {
            print!("{INDEX}");
            Ok(())
        }
        Some("status") => {
            print!("{STATUS}");
            Ok(())
        }
        Some("plan") => {
            print!("{PLAN}");
            Ok(())
        }
        Some("apply") => {
            print!("{APPLY}");
            Ok(())
        }
        Some("reconcile") => {
            print!("{RECONCILE}");
            Ok(())
        }
        Some("feed-loop") | Some("feed") => {
            print!("{FEED_LOOP}");
            Ok(())
        }
        Some("backup") => {
            print!("{BACKUP}");
            Ok(())
        }
        Some("frontier") => {
            print!("{FRONTIER}");
            Ok(())
        }
        Some("day90-mixed") | Some("mixed") => {
            print!("{DAY90_MIXED}");
            Ok(())
        }
        Some("north-star") | Some("northstar") | Some("charter") => {
            print!("{NORTH_STAR}");
            Ok(())
        }
        Some("enrich") | Some("train") => {
            let (head, tail) = ENRICH.split_once(ENRICH_MATRIX_ANCHOR).expect(
                "enrich help keeps the prepare-does-not-train anchor for the beachhead matrix",
            );
            print!("{head}{ENRICH_MATRIX_ANCHOR}\n{LF_BEACHHEAD_MATRIX}\n{tail}");
            Ok(())
        }
        Some(other) => {
            eprintln!("unknown help topic: {other}");
            eprintln!("topics: {}", TOPICS.join(", "));
            bail!("refuse:help-topic: unknown '{other}'");
        }
    }
}

const INDEX: &str = "\
Cell One — Day-90 operator help
===============================
Local only. Hosted CI is compile-only. Cloud-agent is declared, not spawned.
Live Mac / GPU wait in docs/DAY90-PLUS.md. Do not fake them.

  estate help status
  estate help plan
  estate help apply
  estate help reconcile
  estate help feed-loop
  estate help backup
  estate help frontier
  estate help day90-mixed
  estate help north-star
  estate help charter
  estate help enrich

Entrypoint: make gate-90
Loop:       make day90
Mixed:      make day90-mixed
Feed walk:  make feed-loop
Frontier:   estate specialist --driver frontier
Enrich:     estate enrich prepare
From pack:  estate enrich from-pack
Walk:       make enrich-prepare
Train:      make train-prepare
QLoRA walk: make qlora-journey
LoRA walk:  make lora-journey
Seat walk:  make seat-journey
Axolotl:    make axolotl-qlora-journey
Ax chain:   make uniqueness-axolotl
Unsloth:    make unsloth-qlora-journey
Us chain:   make uniqueness-unsloth
Unsloth LoRA: make unsloth-lora-journey
Us LoRA chain: make uniqueness-unsloth-lora
mlx-lm:     make mlx-lm-lora-journey
            print-only Apple Silicon mlx-lm LoRA journey (operator section 16)
mlx chain:  make uniqueness-mlx
            print-only chain of that Apple Silicon journey (operator section 16)
Ax LoRA:    make axolotl-lora-journey
Ax LoRA chain: make uniqueness-axolotl-lora
Train next: make train-next
Full print: make uniqueness-full
LoRA train: make train-next-lora
LoRA seat:  make seat-journey-lora
LoRA full:  make uniqueness-full-lora
Prove list: make uniqueness-prove-checklist
Purpose:    make purpose-build-checklist
            print-only operator path for purpose-building an SLM on demand (operator section 15)
Pick:       make purpose-build-pick
            print-only host and stack picker for purpose-build journeys (operator section 17)
Journey:    make purpose-build-journey
            when an SLM fits mid-software-build, or on demand, the same print-only purpose-build on-demand entry (operator section 18)
DeepSeek:   make deepseek-r1-distill-journey
            print-only DeepSeek-R1-Distill chat QLoRA journey (operator section 19)
Ds chain:   make uniqueness-deepseek
            print-only chain of that journey (operator section 19)
DeepSeek LoRA: make deepseek-r1-distill-lora-journey
Ds LoRA chain: make uniqueness-deepseek-lora
GLM-4:      make glm4-chat-journey
            print-only GLM-4 Chat QLoRA journey (operator section 20)
GLM chain:  make uniqueness-glm
            print-only chain of that journey (operator section 20)
GLM-4 LoRA: make glm4-chat-lora-journey
GLM LoRA chain: make uniqueness-glm-lora
Matrix:     make lf-beachhead-prepare
Live prove: make enrich-live-prove
";

const STATUS: &str = "\
estate status — one-pager
=========================
paused?, lease counts, expired, last plan, last apply, open proposals,
policy present?, doctor line. The frontier line is the binding
`params.model`. The schema card stays `grok-4.7` and is not the binding.
A cell catalog that disagrees is `refuse:frontier-model`. A cell catalog
that does not parse is the same refuse, before the cell success line.
A missing catalog is not a disagreement. An unreadable proposal file
is `refuse:proposal-unreadable` before the status page. A missing
proposals directory is not a proposal. Doctor treats a missing conveyor
mesh as no expired hop leases. A mesh that does not parse, or a hop
host class that is not a class, is FAIL before factory ready. A missing
lifecycle.json is not a failure, and doctor does not invent suspended.
A lifecycle file that does not parse is FAIL before factory ready. A
parsed file prints its state. Status does not invent suspended for a
missing lifecycle.json. It prints paused: - and lifecycle: -. A
lifecycle file that does not parse is a refuse before the status page.
A parsed file prints paused and its state. A missing apply-audit.jsonl is not a
failure, and doctor does not invent a line count. An audit file that
does not parse is FAIL before factory ready. A parsed file prints its
line count. A missing lifecycle.jsonl is not a failure, and doctor
does not invent a line count. A history file that does not parse is
FAIL before factory ready. A parsed file prints its line count.
A missing sessions.jsonl is not a failure, and doctor does not invent
a line count. A journal file that does not parse is FAIL before factory
ready. A parsed file prints its line count. A missing
actual-state.json is not a failure, and doctor does not invent a
session count. A state file that does not parse is FAIL before factory
ready. A parsed file prints its session count. A missing
desired-snapshot.yaml is not a failure, and doctor does not invent a
frontier model from it. A snapshot that does not parse is FAIL before
factory ready when no cell catalog is present. A parsed snapshot prints
its name. A missing
model-actual.json is not a failure, and doctor does not invent a
binding count. A model file that does not parse is FAIL before factory
ready. A parsed file prints its binding count. Status refuses that
same file (`refuse:model-actual`) before the status page. A missing
file is not a failure there either, and status does not invent a
binding count. A missing `{state}/enrich` directory is not a failure,
and status does not invent a prepare count. Doctor is silent on that
missing directory too. An empty enrich directory notes that no
prepare.json is present and does not invent zero packs. A present
prepare.json prints pack, driver, job, seat_tag and train_base when
those fields are present, trained_shape when import-trained recorded
it, and the out path. That line is the prepare record. It does not
mean the factory trained, merged, converted, or seated a model.
A prepare.json that does not parse, or that fails its schema, is
`refuse:prepare-unreadable` or `refuse:prepare` before the status page
and FAIL before factory ready. A symlink in the enrich tree is
`refuse:enrich-index` the same way. Train catalog lines print the
in-tree card status (integration, optional, or portable) with
live=false. A prepare probe is not live. Cloud-agent
stays \"declared, not spawned\".

  estate status --estate examples/estate.yaml --state-dir .cell
  make day90

Does not apply. Does not spawn. Does not require a Mac or a GPU.
";

const PLAN: &str = "\
estate plan — human control surface
===================================
Writes reviewable markdown under plans/. Does not apply.

  estate plan --estate examples/estate.yaml --plans-dir plans
  estate plan --reviewed
  estate plan diff --estate examples/estate.yaml
  estate plan export-pr --out plans/PR.md

apply --require-plan / --require-fresh-plan read these files.

The plan prints an Agents section: id, lane, desktop, placement
(box, cloud-agent declared-not-spawned, or none), declared tool / mcp /
mount / model counts, and that agent's allow / deny / deny-default
coverage for model class, tool / mcp / mount, intention (memory and
compiled intentions), and hop (placement-derived). It does not spawn
agents. Control does not complete.

The plan line prints the bound frontier model, or model=- when the binding
sets none. It does not copy the schema card. An estate with no frontier
binding refuses (refuse:frontier-invent) instead of inventing a frontier
source_driver or catalog model. No live key.

A granted box hop lease, or a box hop declaration with no lease, that
disagrees with placement hop coverage is `refuse:hop-coverage` (mismatch)
and fails plan. Deny and deny-default are notes and do not fail plan by
themselves. A match stays quiet. Cloud hops stay out. A missing mesh is
not a failure. A present mesh that does not parse fails plan and is not
rewritten. The check does not write the mesh, the leases, or the estate.
";

const APPLY: &str = "\
estate apply — converge desired state
=====================================
Binds box sessions. Records placement leases. Does not spawn cursor-cloud.

  estate apply --dry-run --estate examples/estate.yaml --state-dir .cell
  estate apply --estate examples/estate.yaml --state-dir .cell
  estate apply --require-plan --require-fresh-plan
  estate apply --force          # only when drift refuses

Second identical apply is a no-op (unchanged). Expired leases refuse.
After `estate expire --forget`, apply restamps leases. That is not `--force`.
A cell catalog that disagrees with the binding is `refuse:frontier-model`
before any apply write. The schema card is not the binding. A missing
catalog is not a disagreement, and `--force` does not overwrite one.
";

const RECONCILE: &str = "\
estate reconcile — desired vs actual
====================================
Report only. --suggest writes a patch file. Never auto-applies.

  estate reconcile --estate examples/estate.yaml --state-dir .cell
  estate reconcile --suggest --estate examples/estate.yaml --state-dir .cell

Refuse codes: missing-lease, extra-lease, kind-mismatch,
host-class-mismatch, cloud-spawned, sacred-id, expired.
An estate with no frontier binding refuses (refuse:frontier-invent)
before reconcile.json, a suggest patch, or a resume catalog write.
It does not invent a frontier source_driver or the schema card.
A cell catalog that disagrees with the binding is refuse:frontier-model
before resume or pause-proof writes. The schema card is not the binding.
Jason still edits leases / the estate by hand.
";

const FEED_LOOP: &str = "\
feed-loop — scrubbed trace → pack → propose → accept
====================================================
Fixtures only. Isolated target/feed-loop-cell. Not part of make smoke.

  make feed-loop
  estate feed pack --feed-dir .cell/feed --drop-dir packs
  estate feed cursor --feed-dir .cell/feed
  estate packs propose --id overnight-traces
  estate packs accept --id overnight-traces --curator jason

Cursor is a watermark. Rematerialize does not auto-promote.
Packs tag source_drivers frontier and/or local. Promote stays locked off.
Propose and accept refuse a frontier source_driver when the estate has no
frontier binding (refuse:frontier-invent). They do not invent one.
Import refuses the same way, before an accepted pack or a redaction report.
The redaction report is kind counts. It is not a secret and not the schema card.
Accept writes curator edit instructions. estate.yaml is never rewritten.
See docs/FEED-LOOP.md.
";

const BACKUP: &str = "\
estate backup — local cell archive
==================================
Timestamped backups/cell-backup-*. Restore refuses sacred mismatch.
A cell catalog that disagrees with the binding refuses (refuse:frontier-model)
before the archive or the restore write. A frontier source_driver with no
frontier binding is refuse:frontier-invent. A desired snapshot whose sacred
set disagrees is refuse:sacred-mismatch. The schema card is not the binding.

  estate backup --state-dir .cell --out backups
  estate backup --prune 5 --state-dir .cell --out backups
  estate restore --from backups/cell-backup-... --dry-run

--prune N keeps the newest N archives and deletes the rest.
N=0 refuses. Does not upload. Does not spawn.
";

const FRONTIER: &str = "\
frontier — grok-4.7 specialist
==============================
Equal-class frontier card. Not a fallback when local is down.

  estate specialist --driver frontier --prompt \"Reply with the single word pong.\"

Requires XAI_API_KEY. Model is grok-4.7 (CELL_FRONTIER_MODEL or XAI_MODEL).
estate catalog dumps that schema card and labels it as not a binding.
It will not overwrite a catalog whose frontier model is not the card.
An unset binding stays empty. The schema card is not copied into it.
Optional CELL_FRONTIER_ENDPOINT (default https://api.x.ai/v1).
Unset key refuses. Sacred text and hardware SKUs refuse before any POST.
--driver http-remote stays the local card on CELL_LOCAL_ENDPOINT.
Requested local drivers do not POST frontier when local is down.
reasoning_effort xhigh is docs-only. The factory POST does not send it.
No key in CI. This page is not a live-box proof.
";

const DAY90_MIXED: &str = "\
day90-mixed — mixed fixture plan → apply
=========================================
Opt-in. Not part of make smoke, make gate-90, or Actions.

  make day90-mixed

Walks examples/fixtures/mixed-frontier-local.yaml on an isolated cell:
status → plan → apply --require-plan → status → doctor.
Then validates examples/hosts/frontier-http.yaml and prints status.
That host binding names model grok-4.7 on http-remote. No apply. No live key.
The script unsets XAI_API_KEY and CELL_*_ENDPOINT.
It sets CELL_FRONTIER_MODEL and still prints grok-4.7 only from the binding.
A local-only estate in that walk is refuse:frontier-invent.
examples/estate.yaml stays hash-locked. Not part of fixtures-check.
";

const NORTH_STAR: &str = "\
north-star — Agent Estate Factory
=================================
charter prints this same page. charter.md is the source of truth.
Words: docs/UBIQUITOUS_LANGUAGE.md.

Next-generation harness and custom AI creator suite: first-class agents
under a security-first control suite, a Grok Bot–like harness, and
on-spot specialty SLMs, on one box with sacred isolation (Cyera CI +
Rust classroom out; Sanctum is not Cyera) and equal-class frontier+local.

Pillars, in order:
- Agents. Spin up and run agents as first-class under security-as-IaC. Sacred isolation, plan/apply, fail closed. Control does not complete.
- Harness. Grok Bot–like look and feel. The UI may still be deferred.
- Specialty SLMs. Create, train, enrich, and seat purpose-built small models when the work needs them. One facet.
- Later, optional. A central model brain that learns from the user. Parked. Not a commitment.

Posture:
- Security. Akin to a giant service mesh for agents. IaC controls on spin-up, hops, tools, and model seats are a core surface. The control catalog is still being designed. This page does not list it.
- Sanctum credentials are an open design call. Not yes. Not no. Not a credential vault. Sanctum stays a lane. Cyera CI stays out. Sanctum is not Cyera.
- Placement. Fully local, cloud, or a mix by budget and need. Boxes, rented GPU, frontier, mixed. Equal-class frontier and local, and host class, carry that. One estate stays the control plane.
- Horizon. Plan 12–18 months ahead of the market so the architecture is ready when agent, harness, and security patterns shift. Not a thin clone of today's tools.

Beachhead (overnight packing may stay SLM-heavy; this page does not rebalance it):
- A local runtime is an ecosystem seat. Ollama is today's entrant. Catalog / route / bind take the next process.
- Integrate the driver. A from-scratch local server waits until the entrant does not already do the job.
- Facilitate train/enrich of purpose-built small-parameter models. Open-source SLMs get more common.
- Beachhead: curator packs, the specialist path, and TrainEnrichDriver.
- estate enrich prepare writes artifacts. llamafactory-lora writes a LLaMA-Factory LoRA recipe (no quantization). llamafactory-qlora writes the QLoRA recipe. axolotl-lora writes the bf16 Axolotl YAML. axolotl-qlora writes the 4-bit Axolotl YAML. unsloth-qlora is an optional Nvidia-only NEXT handoff and does not write a script. After the operator-owned train, merge-adapt prints Unsloth's documented merged_16bit save. mlx-lm-lora is an optional Apple Silicon NEXT handoff (MLX.md) and does not write a script. This page does not run a trainer.

Anti-shrink:
- Not a gateway. Not an MCP catalog.
- Not an Ollama wrapper-as-product. Not LM Studio-alone.
- Not a thin Grok Bot clone without the estate. The look and feel stays.
- Not a UI-only shell. Not undirected agent sprawl. Not an undirected agent farm. Agents stay first-class.
- The mesh-like security posture, the harness, and the creator suite stay the product.
- Not frontier-proxy-only. Not a local-studio-only shrink.
- Not a weight browser. Enrich packs stay manual. The curator accepts by hand.

Pointers:
  charter.md
  docs/UBIQUITOUS_LANGUAGE.md
  make gate-90
  make day90
  docs/LIVE-PROBES.md

This page does not plan, apply, or probe. Off make smoke,
make gate-90, and Actions.
";

/// Operator table in `docs/lf-beachhead-matrix.md`. `estate help enrich` prints these bytes.
const LF_BEACHHEAD_MATRIX: &str = include_str!("../../../docs/lf-beachhead-matrix.md");

const ENRICH_MATRIX_ANCHOR: &str = "Prepare writes artifacts. It does not train.\n";

const ENRICH: &str = "\
enrich — train/enrich prepare
=============================
estate help train prints this page. Job field is train or enrich.
Default job is enrich. llamafactory-lora, llamafactory-qlora, axolotl-lora, axolotl-qlora, unsloth-qlora, and mlx-lm-lora default to train.
--all-drivers defaults to enrich and includes a card only when that job
is allowed. Prepare writes artifacts. It does not train.

The table above is docs/lf-beachhead-matrix.md. estate help enrich
and estate help train print that file. A bare Ollama seat tag on
those train bases is refuse:train-base.
Opt-in prepare walk: make lf-beachhead-prepare. It reads that table
and prepares every smoke fixture on a throwaway copy of
examples/estate.yaml. It checks prepare.json, recipe.yaml, the
template, and the row knobs, then prints SKIP live train. Phi-3-small
stays QLoRA-only and is not a row. It does not train, merge, convert,
seat, or promote. Not in make smoke, make gate-90, or Actions.
READY_FOR_LIVE_TEST stays no.
make purpose-build-checklist is the print-only operator path for purpose-building an SLM on demand (operator section 15). It points at
the print-only cards already on tip, including the Apple Silicon print pointer make mlx-lm-lora-journey, the DeepSeek-R1-Distill print pointer make deepseek-r1-distill-journey and make uniqueness-deepseek plus the LoRA twins, and the GLM-4 Chat print pointer make glm4-chat-journey and make uniqueness-glm plus the LoRA twins, and does not run them. It does not
train, convert, shell out to ollama, promote, or apply the estate. It
does not invent a live PASS. The recorded PASS stays the only live
uniqueness prove. The re-prove card stays make uniqueness-prove-checklist.
Walk: docs/operator-enrich-journeys.md (section 15). Not in make smoke,
make gate-90, or Actions.
make purpose-build-pick is the print-only host and stack picker for purpose-build journeys (operator section 17).
It names Nvidia / CUDA (lf-beachhead-prepare, qlora-journey, uniqueness-full primary; Unsloth optional; Axolotl integration),
Apple Silicon (mlx-lm-lora-journey and uniqueness-mlx, optional; refuse:host on stock any-affinity packs),
and the Target A LoRA twins that already exist. It does not run those targets.
It does not invent a live PASS. Not native MLX. Not in make smoke,
make gate-90, or Actions. READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 17).
make deepseek-r1-distill-journey is the print-only DeepSeek-R1-Distill chat QLoRA journey (operator section 19).
make uniqueness-deepseek is the print-only chain of that journey. The LoRA twin is
make deepseek-r1-distill-lora-journey and make uniqueness-deepseek-lora.
Seat tag llama3. Train base deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B. Template deepseekr1.
They do not train, merge, convert, shell out to ollama, or promote.
They do not invent a live PASS. Not in make smoke, make gate-90, or Actions.
READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 19).
make glm4-chat-journey is the print-only GLM-4 Chat QLoRA journey (operator section 20).
make uniqueness-glm is the print-only chain of that journey. The LoRA twin is
make glm4-chat-lora-journey and make uniqueness-glm-lora.
Seat tag llama3. Train base zai-org/glm-4-9b-chat. Template glm4.
They do not train, merge, convert, shell out to ollama, or promote.
They do not invent a live PASS. Not in make smoke, make gate-90, or Actions.
READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 20).
When an SLM fits mid-software-build, or on demand, make purpose-build-journey is the print-only purpose-build on-demand entry (operator section 18).
It runs make purpose-build-pick, then make purpose-build-checklist. It does not inline those bodies.
It does not train, fuse, convert, shell out to ollama, promote, or apply the estate.
It does not invent a live PASS. The recorded PASS stays the only live uniqueness prove.
Not native MLX. Not in make smoke, make gate-90, or Actions. READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 18).
make mlx-lm-lora-journey is the print-only Apple Silicon mlx-lm LoRA journey (operator section 16).
make uniqueness-mlx is the print-only chain of that journey. Both stay
print-only. They do not train, fuse, convert, shell out to ollama, or
promote. They do not invent a live PASS. Not native MLX. Not in make
smoke, make gate-90, or Actions. READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 16).

  estate enrich drivers
  estate enrich from-pack --estate <your-estate.yaml> \\
    --pack <accepted-pack-id> --state-dir .cell
  estate enrich prepare --estate <your-estate.yaml> \\
    --pack examples/fixtures/specialist-overnight.pack.json \\
    --driver llamafactory-lora --job train --state-dir .cell
  estate enrich prepare --estate <your-estate.yaml> \\
    --pack examples/fixtures/specialist-overnight.pack.json \\
    --driver llamafactory-qlora --job train --state-dir .cell
  estate enrich prepare --estate <your-estate.yaml> \\
    --pack examples/fixtures/specialist-overnight.pack.json \\
    --all-drivers --state-dir .cell
  estate enrich list --state-dir .cell
  estate enrich import-prepared --estate <your-estate.yaml> \\
    --prepared .cell/enrich/overnight-traces/ollama-modelfile \\
    --tag cell-enrich-overnight-traces \\
    --path .cell/enrich/overnight-traces/ollama-modelfile/Modelfile
  estate enrich import-trained --estate <your-estate.yaml> \\
    --prepared .cell/enrich/overnight-traces/llamafactory-lora \\
    --tag cell-enrich-overnight-traces \\
    --adapter .cell/enrich/overnight-traces/llamafactory-lora/outputs
  estate enrich import-trained --estate <your-estate.yaml> \\
    --prepared .cell/enrich/overnight-traces/llamafactory-qlora \\
    --tag cell-enrich-overnight-traces \\
    --adapter .cell/enrich/overnight-traces/llamafactory-qlora/outputs
  estate enrich apply-proposal --estate <your-estate.yaml> \\
    --prepared .cell/enrich/overnight-traces/ollama-modelfile \\
    --tag cell-enrich-overnight-traces --state-dir .cell
  make enrich-prepare
  make train-prepare
  make qlora-journey
  make train-next
  make lora-journey
  make seat-journey
  make axolotl-qlora-journey
  make uniqueness-axolotl
  make unsloth-qlora-journey
  make uniqueness-unsloth
  make unsloth-lora-journey
  make uniqueness-unsloth-lora
  make mlx-lm-lora-journey
  make uniqueness-mlx
  make axolotl-lora-journey
  make uniqueness-axolotl-lora
  make uniqueness-ladder
  make uniqueness-full
  make train-next-lora
  make seat-journey-lora
  make uniqueness-full-lora
  make uniqueness-prove-checklist
  make purpose-build-checklist
  make purpose-build-pick
  make purpose-build-journey
  make deepseek-r1-distill-journey
  make uniqueness-deepseek
  make deepseek-r1-distill-lora-journey
  make uniqueness-deepseek-lora
  make glm4-chat-journey
  make uniqueness-glm
  make glm4-chat-lora-journey
  make uniqueness-glm-lora
  make lf-beachhead-prepare
  make enrich-live-prove

TrainEnrichDriver lives in the data plane (model-estate).
Cards today: ollama-modelfile (Ollama create / Modelfile FROM+SYSTEM),
external-manifest (portable JSON/YAML, no vendor lock),
llamafactory-lora (LLaMA-Factory LoRA recipe.yaml, no quantization, rank 8;
default job train; does not require bitsandbytes),
llamafactory-qlora (LLaMA-Factory QLoRA recipe.yaml + instruct chat dataset.jsonl;
default job train; requires bitsandbytes), axolotl-lora (bf16 Axolotl axolotl.yml,
sequence_len 2048, micro_batch_size 2, gradient_accumulation_steps 2, lora_r 16;
default job train), axolotl-qlora (4-bit Axolotl axolotl.yml, load_in_4bit true,
sequence_len 4096, micro_batch_size 2, gradient_accumulation_steps 4, lora_r 32;
default job train), unsloth-qlora (optional NEXT card, status optional,
Nvidia-only QLoRA handoff; writes UNSLOTH.md; does not write a script,
a recipe, or dataset.jsonl; does not call Unsloth; after that train,
merge-adapt prints save_pretrained_merged with save_method merged_16bit;
default job train),
and mlx-lm-lora (optional NEXT card, status optional, Apple Silicon
LoRA handoff; writes MLX.md only when host_class_affinity is
apple-silicon; another affinity is refuse:host and writes nothing;
does not write a script, a recipe, or dataset.jsonl; does not call
mlx-lm; default job train).
A later entrant adds one catalog card. Floor and control dispatch
do not match driver ids. --all-drivers prepares every card the job
allows, into sibling directories. mlx-lm-lora is omitted unless
host_class_affinity is apple-silicon, so a train set on any other
affinity still prepares. Omit --driver on prepare for the
first card. from-pack omits --driver to prepare every allowed card
into .cell/enrich.

Modelfile FROM is the seated model. That is params.model on the
local binding, or a pack model_hint that is already a model tag
(for example llama3). The binding id local_slm is not a model tag.
A missing seated name is refuse:base-model and writes nothing.
examples/estate.yaml leaves params.model unset. A lab copy sets it.

llamafactory-lora and llamafactory-qlora write model_name_or_path from the
train base. Set pack field train_base_model, or params.train_base_model
on the local binding, to a Hugging Face repo id (namespace/name) or a
local directory of HF weights. A relative directory is written as an
absolute path. A directory name that is an Ollama seat tag
(./llama3) is refuse:train-base and writes nothing. template is
inferred by scanning path segments of that train base, starting at
the last segment. A leaf such as weights or an HF snapshot hash uses
the nearest ancestor that names a family. A Qwen3 name containing
instruct and not thinking, or containing nothink, uses qwen3_nothink.
Other Qwen3 names use qwen3.
Phi-3 mini, Phi-3 medium, and Phi-3.5 Instruct use template phi.
Phi-3-small uses phi_small. A nested local path segment matches the same way.
examples/fixtures/phi3-instruct.pack.json is the Phi-3 Instruct QLoRA smoke pack.
--driver llamafactory-qlora on that pack is a reproduce target beside
Qwen LoRA/QLoRA and still writes quantization_method bnb and quantization_bit 4.
That reproduce line is QLoRA-only. Phi-3-small uses phi_small and still gets
that QLoRA line.
examples/fixtures/phi3-instruct-lora.pack.json is the Phi-3 Instruct LoRA
smoke pack. --driver llamafactory-lora on that pack is the non-quant twin
of that QLoRA prepare. It writes template phi, lora_rank 8, packing false,
and no quantization_bit or quantization_method. That reproduce line is
LoRA-only and only for Phi-3 mini, Phi-3 medium, and Phi-3.5.
Phi-3-small, Phi-4, and Phi-4-mini do not get that line.
Llama-3.2-1B-Instruct and Llama-3.2-3B-Instruct use template llama3.
Llama-3.2 vision (11B and 90B) uses mllama. A short llama-3 stem does not
label those names llama3. llama-30b stays default.
examples/fixtures/llama32-instruct.pack.json is the Llama-3.2 Instruct QLoRA smoke pack.
--driver llamafactory-qlora on that pack is a reproduce target beside
Phi-3 and Qwen LoRA/QLoRA and still writes quantization_method bnb and quantization_bit 4.
That reproduce line is QLoRA-only and only for Llama-3.2-1B-Instruct and
Llama-3.2-3B-Instruct.
examples/fixtures/llama32-instruct-lora.pack.json is the Llama-3.2 Instruct LoRA
smoke pack. --driver llamafactory-lora on that pack is the non-quant twin
of that QLoRA prepare. It uses template llama3, lora_rank 8, packing false,
and no quantization_bit or quantization_method. That reproduce line is
LoRA-only and only for Llama-3.2-1B-Instruct and Llama-3.2-3B-Instruct.
Llama-3.2 vision, a Llama-3.2 base, and Llama-3.1 Instruct do not get that line.
Gemma-2-2b-it, Gemma-2-9b-it, and Gemma-2-27b-it use template gemma2.
A short gemma stem does not label those names gemma. gemma-2b and gemma-7b
stay gemma. Gemma-3 stays off gemma2.
examples/fixtures/gemma2-instruct.pack.json is the Gemma-2 Instruct QLoRA smoke pack.
--driver llamafactory-qlora on that pack is a reproduce target beside
Phi-3, Llama-3.2, and Qwen LoRA/QLoRA and still writes quantization_method bnb
and quantization_bit 4. That reproduce line is QLoRA-only and only for a
Gemma-2 Instruct id.
examples/fixtures/gemma2-instruct-lora.pack.json is the Gemma-2 Instruct LoRA
smoke pack. --driver llamafactory-lora on that pack is the non-quant twin
of that QLoRA prepare. It writes template gemma2, lora_rank 8, and no
quantization_bit or quantization_method. That reproduce line is LoRA-only
and only for a Gemma-2 Instruct id. A Gemma-2 base, original Gemma, and
Gemma-3 do not get that line.
Mistral-7B-Instruct-v0.1, v0.2, and v0.3 use template mistral.
A Mistral-7B base uses that same template and is not the reproduce target.
Mistral-Small uses mistral_small. Mistral-Nemo uses ministral.
Mixtral uses mistral and is not this reproduce target.
LLaVA-NeXT-Mistral uses llava_next_mistral.
Ministral, Ministral-3, Codestral, Devstral, and Pixtral stay off mistral.
examples/fixtures/mistral-instruct.pack.json is the Mistral Instruct QLoRA smoke pack.
--driver llamafactory-qlora on that pack is a reproduce target beside
Phi-3, Llama-3.2, Gemma-2, and Qwen LoRA/QLoRA and still writes quantization_method bnb
and quantization_bit 4. That reproduce line is QLoRA-only and only for a
Mistral-7B Instruct id.
examples/fixtures/mistral-instruct-lora.pack.json is the Mistral Instruct LoRA
smoke pack. --driver llamafactory-lora on that pack is the non-quant twin
of that QLoRA prepare. It writes template mistral, lora_rank 8, packing false,
and no quantization_bit or quantization_method. That reproduce line is LoRA-only
and only for a Mistral-7B Instruct id. A Mistral-7B base, Mistral-Small,
Mistral-Nemo, Mixtral, and LLaVA-NeXT-Mistral do not get that line.
Qwen/Qwen3-4B-Instruct-2507, Qwen/Qwen3-30B-A3B-Instruct-2507,
Qwen/Qwen3-235B-A22B-Instruct-2507, and Qwen/Qwen3-Next-80B-A3B-Instruct
use template qwen3_nothink. examples/train_lora/qwen3_lora_sft.yaml names
Qwen/Qwen3-4B-Instruct-2507 with that template. A Qwen3 thinking or base id,
including Qwen/Qwen3-4B and Qwen/Qwen3-4B-Thinking-2507, uses qwen3 and is
not this reproduce target. Qwen2 stays template qwen and is not
this reproduce target. Qwen2.5 Instruct, including
Qwen/Qwen2.5-0.5B-Instruct, stays template qwen. That is a different
reproduce target.
examples/fixtures/qwen3-instruct.pack.json is the Qwen3 Instruct QLoRA smoke pack.
--driver llamafactory-qlora on that pack is a reproduce target beside
Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA and still writes
quantization_method bnb and quantization_bit 4. That reproduce line is
QLoRA-only and only for that Qwen3 Instruct shape.
examples/fixtures/qwen3-instruct-lora.pack.json is the Qwen3 Instruct LoRA
smoke pack. --driver llamafactory-lora on that pack is the non-quant twin
of that QLoRA prepare. It matches examples/train_lora/qwen3_lora_sft.yaml:
template qwen3_nothink, lora_rank 8, and no quantization_bit or
quantization_method. That reproduce line is LoRA-only and only for that
Qwen3 Instruct shape. A Qwen3 thinking id does not get that line. A
Qwen2.5 Instruct id does not get that Qwen3 line.
Qwen2.5 Instruct text ids use template qwen. That group in constants.py
is Qwen2.5-0.5B-Instruct, Qwen2.5-1.5B-Instruct, Qwen2.5-3B-Instruct,
Qwen2.5-7B-Instruct, Qwen2.5-14B-Instruct, Qwen2.5-32B-Instruct,
Qwen2.5-72B-Instruct, and the Instruct-1M ids. template.py registers
qwen. There is no qwen2_5 template. A Qwen2.5 base, a name containing
thinking, Qwen2.5-Coder, Qwen2.5-Math, and Qwen2.5-VL do not get this
line. Qwen2 Instruct does not get this line. Qwen3 Instruct stays
qwen3_nothink.
examples/fixtures/qwen25-instruct.pack.json is the Qwen2.5 Instruct QLoRA
smoke pack. --driver llamafactory-qlora on that pack is a reproduce
target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct
and still writes quantization_method bnb and quantization_bit 4.
That reproduce line is on llamafactory-qlora and only for that
Qwen2.5 Instruct shape.
examples/fixtures/qwen25-instruct-lora.pack.json is the Qwen2.5 Instruct
LoRA smoke pack. --driver llamafactory-lora on that pack is the non-quant
twin of that QLoRA prepare. It writes template qwen, lora_rank 8, packing
false, and no quantization_bit or quantization_method. That reproduce line
is LoRA-only and only for that Qwen2.5 Instruct shape. A Qwen2.5 base, a
name containing thinking, Qwen2, Qwen2.5-Coder, Qwen2.5-Math, and Qwen2.5-VL
do not get that line. A Qwen3 Instruct id does not get that line.
DeepSeek-R1-Distill chat ids use template deepseekr1. That group in
constants.py is DeepSeek-R1-Distill-Qwen-1.5B, DeepSeek-R1-Distill-Qwen-7B,
DeepSeek-R1-Distill-Llama-8B, DeepSeek-R1-Distill-Qwen-14B,
DeepSeek-R1-Distill-Qwen-32B, and DeepSeek-R1-Distill-Llama-70B.
template.py registers deepseekr1 as a ReasoningTemplate. There is no
deepseek_r1 template. A Qwen or Llama substring in those ids stays
deepseekr1. The Qwen and Llama student checkpoints stay qwen or llama3.
DeepSeek-R1, DeepSeek-R1-Zero, and DeepSeek-R1-0528 stay deepseekr1 and
do not get this line. Qwen2.5 Instruct stays qwen. Qwen3 Instruct stays
qwen3_nothink.
examples/fixtures/deepseek-r1-distill.pack.json is the DeepSeek-R1-Distill
chat QLoRA smoke pack. Its train base is
deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B. --driver llamafactory-qlora on that pack is a
reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5
Instruct, and Qwen3 Instruct and still writes quantization_method bnb
and quantization_bit 4. That reproduce line is on llamafactory-qlora
and only for those six distill chat ids.
examples/fixtures/deepseek-r1-distill-lora.pack.json is the DeepSeek-R1-Distill
chat LoRA smoke pack. --driver llamafactory-lora on that pack is the non-quant
twin of that QLoRA prepare. It writes template deepseekr1, lora_rank 8,
packing false, and no quantization_bit or quantization_method.
examples/train_lora does not ship a DeepSeek yaml. That reproduce line is
LoRA-only and only for those six distill chat ids. The QLoRA note stays on
llamafactory-qlora. DeepSeek-R1, DeepSeek-R1-Zero, and DeepSeek-R1-0528 do
not get that LoRA line. The smoke seat tag is llama3.
An Ollama tag such as deepseek-r1 or deepseek-r1:1.5b is a seat tag for
this checkpoint. It is not the Hugging Face train base.
GLM-4 Chat ids use template glm4. That group in constants.py is
zai-org/glm-4-9b-chat, zai-org/glm-4-9b-chat-1m, zai-org/GLM-4-9B-0414,
and zai-org/GLM-4-32B-0414. The DEFAULT DownloadSource for GLM-4-9B-Chat
is zai-org/glm-4-9b-chat. template.py registers glm4. There is no glm_4
template. ChatGLM3 stays chatglm3. A GLM-4 base stays glm4 and does not
get this line. GLM-Z1, GLM-4.1V, and GLM-4.5 do not get this line.
Qwen2.5 Instruct stays qwen. Qwen3 Instruct stays qwen3_nothink.
DeepSeek-R1-Distill chat stays deepseekr1.
examples/fixtures/glm4-chat.pack.json is the GLM-4 Chat QLoRA smoke pack.
Its train base is zai-org/glm-4-9b-chat. --driver llamafactory-qlora on
that pack is a reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral,
Qwen2.5 Instruct, Qwen3 Instruct, and DeepSeek-R1-Distill chat and still
writes quantization_method bnb and quantization_bit 4. That reproduce
line is on llamafactory-qlora and only for those GLM-4 Chat ids.
examples/fixtures/glm4-chat-lora.pack.json is the GLM-4 Chat LoRA smoke
pack. --driver llamafactory-lora on that pack is the non-quant twin of
that QLoRA prepare. It writes template glm4, lora_rank 8, packing false,
and no quantization_bit or quantization_method. examples/train_lora does
not ship a GLM-4 yaml. That reproduce line is LoRA-only and only for
those GLM-4 Chat ids. The QLoRA note stays on llamafactory-qlora. A
GLM-4 base, GLM-Z1, GLM-4.1V, and GLM-4.5 do not get that LoRA line.
DeepSeek-R1-Distill chat does not get that LoRA line. The smoke seat
tag is llama3. An Ollama tag such as glm4, glm4:9b, or glm-4:9b is a
seat tag for this checkpoint. It is not the Hugging Face train base.
Select LoRA with --driver llamafactory-lora (16-bit base, no
quantization_bit, lora_rank 8, packing false). Select QLoRA with
--driver llamafactory-qlora (quantization_bit 4, quantization_method bnb,
lora_rank 16).
A bare Ollama seat tag is refuse:train-base and writes nothing.
This factory does not map the seat tag onto a Hub repo.
axolotl-lora and axolotl-qlora write that same train base to base_model in axolotl.yml.
axolotl-lora is bf16 LoRA (adapter lora, load_in_4bit false), matching
examples/llama-3/lora-1b.yml. axolotl-qlora is 4-bit QLoRA (adapter qlora,
load_in_4bit true), matching examples/llama-3/qlora.yml. sequence_len,
micro_batch_size, gradient_accumulation_steps, and lora_r are the values
in those files. prepare.json base_model and seat_tag stay the Ollama id
for Modelfile FROM and for the adapter join. NEXT.md names axolotl train.
After axolotl train, estate enrich merge-adapt prints Axolotl's
axolotl merge-lora line (docs.axolotl.ai getting-started section 4.4
and the CLI page). --lora-model-dir is the adapter directory
(adapter_config.json). Axolotl writes the merged Hugging Face
directory to output_dir/merged. This prepare sets output_dir to
outputs, so that directory is outputs/merged. axolotl merge-lora
does not take --out. axolotl-qlora also prints the CLI --dequant
line. That flag writes a bf16 checkpoint for a quantized base. Both
lines write output_dir/merged. This factory does not run the merge.
Axolotl does not write GGUF. Then gguf-convert prints
python3 convert_hf_to_gguf.py with --outtype auto, local-seat prints
ollama create, and import-trained records the adapter directory, that
merged directory, or a .gguf file. unsloth-qlora prints its own
save ladder after the operator-owned train. merge-adapt prints
model.save_pretrained_merged with save_method merged_16bit when the
adapter directory holds adapter_config.json and adapter_model.safetensors
(adapter_model.bin when safe_serialization is False). The merged
directory is merged beside the prepare. This factory does not create
that directory and does not print PeftModel.merge_and_unload for this
card. gguf-convert and local-seat then print the convert and the seat.
local-seat --adapter on this prepare is refuse:adapter. mlx-lm-lora prints its own fuse line.
merge-adapt on that prepare prints mlx_lm.fuse. The adapter directory
holds adapter_config.json and adapters.safetensors. --save-path is
fused_model beside the prepare. --export-gguf writes
ggml-model-f16.gguf inside that directory. local-seat prints the
Ollama line for that GGUF file. The fused directory is MLX weights.
This factory does not print a Hugging Face convert line for it and
does not run fuse.

  estate enrich merge-adapt \\
    --prepared .cell/enrich/<pack-id>/axolotl-qlora \\
    --adapter .cell/enrich/<pack-id>/axolotl-qlora/outputs
  estate enrich gguf-convert \\
    --prepared .cell/enrich/<pack-id>/axolotl-qlora \\
    --weights .cell/enrich/<pack-id>/axolotl-qlora/outputs/merged
  estate enrich local-seat \\
    --prepared .cell/enrich/<pack-id>/axolotl-qlora \\
    --weights .cell/enrich/<pack-id>/axolotl-qlora/outputs/merged

The LoRA card does not require bitsandbytes. QLoRA still needs
bitsandbytes: pip install 'bitsandbytes>=0.49'.
A short gauge run passes --max-steps 10. Axolotl writes max_steps and
omits saves_per_epoch on that gauge. The default recipe leaves
max_steps unset. Pass --official-scale to write the LLaMA-Factory
examples/train_lora/qwen3_lora_sft.yaml scale on llamafactory-lora and
llamafactory-qlora only: cutoff_len 2048, num_train_epochs 3.0,
gradient_accumulation_steps 8, warmup_ratio 0.1.
Rank, packing, and quantization stay on the selected LLaMA-Factory card.
--max-steps still overrides epochs. axolotl-lora and axolotl-qlora
stay on their example files when that flag is set.
Omit the flag for the short LLaMA-Factory recipe. A prepare with
--official-scale and no train recipe card is refuse:official-scale.
unsloth-qlora is not a recipe card. --official-scale on that card
alone is refuse:official-scale. --from-feed on that card alone is
refuse:dataset. The card does not write max_steps. --max-steps 0
is still refuse:max-steps. mlx-lm-lora is the same kind of handoff
for apple-silicon. --official-scale and --from-feed on that card
alone are the same refuses. Another host class is refuse:host.

export.yaml is the merge card. Prepare does not merge. NEXT.md says
the merge has not happened, points at llamafactory-cli export, and
says to leave that file unquantized. import-trained refuses
export.yaml when it sets quantization_bit or quantization_method
(refuse:export). A comment line does not trip that refuse.

dataset.jsonl defaults to a scaffold (or a three-row stub when the
pack source_paths list is empty). prepare.json records dataset_mode,
dataset_rows, dataset_from_feed, dataset_skipped, and
dataset_read_paths. PREPARE.md and NEXT.md on llamafactory-lora,
llamafactory-qlora, axolotl-lora, and axolotl-qlora say those rows are not training
data. Pass --from-feed to copy instruct
rows that are already under --state-dir (for example
.cell/feed/events.jsonl). A missing file is refuse:dataset and writes
nothing. This factory does not download pack sources. ShareGPT
messages, Alpaca instruction and output, and a scrubbed feed event
with a note are the rows it copies. An event with no note is skipped.
kind and object_class are checked before that copy, so a frontier
event wrapped as messages is still refuse:frontier-invent when the
estate has no frontier binding. Each source is at most 8 MiB, all
sources together are at most 8 MiB, and prepare reads the opened
file rather than a path it reopens.

Each prepare writes prepare.json, PREPARE.md, and NEXT.md.
NEXT.md has the handoff command, artifact paths, and fail-closed
reminders. The factory does not shell out to ollama create.

estate enrich list reads {state}/enrich and refuses when that
directory is missing. It does not create the directory.

estate enrich import-prepared checks prepare.json plus the tag and
path you created outside the factory. It writes binding-proposal.json
and binding-proposal.md for the existing local_slm seat.
import-prepared does not apply.

estate enrich import-trained is that same proposal for a llamafactory-lora,
llamafactory-qlora, axolotl-lora, axolotl-qlora, unsloth-qlora, or mlx-lm-lora prepare whose job is train. --adapter
is one of three shapes. An adapter output_dir contains adapter_config.json
(the recipe output_dir, outputs/). A merged export_dir contains config.json
and at least one .safetensors file whose name does not start with
adapter_model. adapter_model.safetensors is an adapter weight, not a merged
export. A Modelfile in that directory is
recorded when present (the export.yaml export_dir, export/). A GGUF path
is one .gguf file, or a directory with exactly one top-level .gguf file.
A directory with more than one is refuse:adapter. Marker files must be regular files in that directory.
A symlinked marker, or a symlinked --adapter path, is refuse:adapter.
NEXT.md on llamafactory-lora and llamafactory-qlora prints
the exact import-trained command for each shape, with the prepared
directory filled in. A path that matches none of those shapes, or more
than one, is refuse:adapter and writes no proposal. On mlx-lm-lora a
directory with config.json and a .safetensors file whose name does not
start with adapter_model is a fused MLX directory. import-trained
refuses it (refuse:adapter) and does not record trained_shape merged.
That card records the adapter directory or a GGUF file. prepare.json and
binding-proposal.json record trained_shape and trained_paths on one
success path. A failed write removes a partial proposal. It does
not apply, does not promote, and does not rewrite the estate.
apply-proposal, plan, and apply --require-plan stay the join. Ollama
stays the local-run seat. Curator stays jason. Sacred, SKU, and a
frontier source with no frontier binding still refuse.

estate enrich apply-proposal reads that proposal, checks it against
prepare.json, and writes {state}/enrich-stage/staged-estate.yaml.
That file is the estate plan input. It does not apply and it does not
rewrite the source estate. Then:

  estate plan --estate {state}/enrich-stage/staged-estate.yaml
  estate apply --estate {state}/enrich-stage/staged-estate.yaml --require-plan

The source estate is written only when that apply succeeds.
Same tag and binding again is a no-op. A mismatched prepare, a missing
proposal, or the wrong tag refuses before the stage exists.
--verify-local-tag is off unless you set it. When set, the seated
runtime must list the tag (OpenAI /v1/models or Ollama /api/tags).
A down runtime or a missing tag refuses before the stage exists.

Default out is .cell/enrich/{pack}/{driver}. See docs/cell-layout.md.
With --all-drivers and --out, each driver writes to {out}/{driver}.
Sacred text, a hardware SKU, a missing pack, the wrong curator, and a
frontier source_driver with no frontier binding refuse before any write.
Point --estate at a lab copy. examples/estate.yaml on main stays hash-locked.
apply-proposal and import-prepared leave the source estate unchanged.
Promote stays off. No train POST.

Opt-in walk: make enrich-prepare. make train-prepare writes
LLaMA-Factory LoRA and QLoRA recipes, Axolotl LoRA and QLoRA recipes,
and the optional Unsloth and mlx-lm handoffs
under /tmp and does not run a trainer. Neither is part of
make smoke, make gate-90, or Actions. make enrich-live-prove runs ollama create on a throwaway
cell when the seat is up, then removes the tag. It is an opt-in seated
handoff. It is not a factory-wide live test. READY_FOR_LIVE_TEST stays no.
estate enrich merge-adapt prints the external adapter merge for an
axolotl-lora, axolotl-qlora, llamafactory-lora, or llamafactory-qlora
train prepare, the mlx_lm.fuse line for an mlx-lm-lora train
prepare on apple-silicon, and Unsloth's save_pretrained_merged
merged_16bit line for an unsloth-qlora train prepare. --adapter is an adapter directory
(adapter_config.json). On mlx-lm-lora that directory also holds
adapters.safetensors. On unsloth-qlora it also holds
adapter_model.safetensors (or adapter_model.bin). A PEFT adapter_model file on mlx-lm-lora, a missing weight
file, another host, a symlink, and a wrong job still refuse.
A merged Hugging Face directory or a GGUF is refuse:adapter. For
Axolotl the printed line is axolotl merge-lora with --lora-model-dir.
axolotl-qlora also prints --dequant. Axolotl writes output_dir/merged.
For a LLaMA-Factory adapter the printed line is llamafactory-cli export
on the prepare's export.yaml. The report lists the keys from
examples/merge_lora/qwen3_lora_sft.yaml: model_name_or_path,
adapter_name_or_path, template, trust_remote_code, export_dir,
export_size, export_device, and export_legacy_format. LLaMA-Factory
writes that export_dir. A missing export.yaml, a quantized export key,
and a train base that does not match model_name_or_path refuse.
The command does not merge, does not shell out, and does not promote.
It then prints gguf-convert and local-seat for that export directory.

  estate enrich merge-adapt \\
    --prepared .cell/enrich/overnight-traces/axolotl-qlora \\
    --adapter .cell/enrich/overnight-traces/axolotl-qlora/outputs

estate enrich gguf-convert prints the llama.cpp convert line for a
merged Hugging Face directory (config.json and at least one
.safetensors file whose name does not start with adapter_model) from
a llamafactory-lora, llamafactory-qlora, axolotl-lora,
axolotl-qlora, or unsloth-qlora train prepare. For unsloth-qlora the
report also prints the three manual lines on Unsloth's saving-to-gguf
page (--outtype f16, bf16, and q8_0, each with --split-max-size 50G).
Unsloth's page does not publish --outtype auto. The factory card line
stays the llama.cpp default. mlx-lm-lora does not use this script.
gguf-convert on that prepare is refuse:seat. The documented GGUF path
is mlx_lm.fuse --export-gguf, which writes ggml-model-f16.gguf. This
factory does not invent a convert script. An adapter directory, a
directory that only holds export.yaml, a symlinked weights path, a
symlinked marker, and a path that is already a GGUF are refuse:seat.
The printed line is:

  python3 convert_hf_to_gguf.py <merged-dir> --outfile <sibling>.gguf --outtype auto

--outtype auto is that script's default (highest-fidelity 16-bit float).
The outfile is a sibling of the merged directory. The command then
points at local-seat for that file. It does not run llama.cpp, does
not write a GGUF, and does not choose a quantization type.
When tokenizer_config.json in that directory has extra_special_tokens
as a JSON list or JSON null, gguf-convert is refuse:tokenizer.
transformers raises AttributeError ('list' object has no attribute
'keys') during convert_hf_to_gguf.py when the value is a list. JSON
null is the same refuse: transformers calls .keys() on that non-object
value. A Qwen-family export missing vocab.json or merges.txt is the
same refuse. Qwen-family is config.json model_type or architectures,
or tokenizer_class, naming Qwen. Copy those tokenizer
files from the HF cache snapshot for the train base already on disk,
or the equivalent base checkout, into the export directory.
HF hub snapshots are often symlinks into the HF cache.
Copy with dereference (cp -aL or cp --dereference, or the equivalent)
so the files in the export directory are real files, not symlinks.
A plain cp -a leaves tokenizer_config.json as a symlink.
enrich does not follow a symlinked tokenizer_config.json.
Keep the export tokenizer_config.json as
tokenizer_config.json.bak. Then re-run estate enrich gguf-convert on
that export directory. This factory does not download weights and does
not copy the files.

  estate enrich gguf-convert \\
    --prepared .cell/enrich/overnight-traces/llamafactory-qlora \\
    --weights .cell/enrich/overnight-traces/llamafactory-qlora/export

estate enrich local-seat validates that same merged directory
(optional Modelfile) or a .gguf file for a llamafactory-lora,
llamafactory-qlora, axolotl-lora, axolotl-qlora, or unsloth-qlora train prepare.
adapter_model*.safetensors is not merged evidence. A symlinked weights
path or a symlinked marker is refuse:seat. It prints the ollama create
line. For a GGUF whose Modelfile is not already on disk, it also prints
the Modelfile whose FROM is that file, plus llama-cli -m and
llama-server -m --port 8080 for that file. On that GGUF print-only path,
local-seat is print-only. It prints the Modelfile and does not write
$PREPARED/Modelfile. Write that file from the printed contents before
ollama create.
When modelfile_on_disk=true, the report uses the on-disk Modelfile when FROM already names the artifact. Do not write $PREPARED/Modelfile again.
Ollama stays the default print. --runtime llama.cpp selects those
llama.cpp lines and still prints the Ollama line. A merged directory
is not a llama.cpp seat: the report points at gguf-convert first and
does not print llama-cli for the directory. Axolotl does not write that GGUF.
mlx-lm-lora on apple-silicon seats one .gguf file, the file
mlx_lm.fuse --export-gguf writes (default name ggml-model-f16.gguf).
A fused MLX directory is refuse:seat. import-trained on that card
records the adapter directory or the GGUF file. The fused directory
is refuse:adapter there too. --adapter on that prepare is
refuse:adapter. Another host is refuse:host.
unsloth-qlora seats the merged 16-bit directory or a GGUF file.
--adapter on that prepare is refuse:adapter. Unsloth documents Ollama
through a GGUF, not an Ollama adapter line for the PEFT directory.
A missing UNSLOTH.md or train base is refuse:train-base.
The create name is cell-enrich-{pack}. The seat tag is prepare.json
seat_tag. The command does not run ollama or llama.cpp. GGUF conversion stays
llama.cpp convert_hf_to_gguf.py, outside this factory. On llamafactory-lora,
llamafactory-qlora, axolotl-lora, axolotl-qlora, and unsloth-qlora,
import-trained records that same merged directory or GGUF on the
local_slm proposal. local-seat does not promote.

  estate enrich local-seat \\
    --prepared .cell/enrich/overnight-traces/llamafactory-qlora \\
    --weights .cell/enrich/overnight-traces/llamafactory-qlora/export

That --weights path is the merged export directory. When its Modelfile
is already on disk, the report uses the on-disk Modelfile when FROM already names the artifact.
Do not write $PREPARED/Modelfile again.

Pass --adapter instead of --weights to print the no-merge seat.
--adapter is an adapter output_dir (adapter_config.json, the same
marker import-trained accepts, plus the adapter weights when the train
wrote them). The printed Modelfile uses FROM the prepare.json seat_tag
and ADAPTER that directory. --weights still refuses an adapter
directory (refuse:seat). A merged export or a GGUF passed to --adapter
is refuse:adapter. unsloth-qlora and mlx-lm-lora --adapter are
refuse:adapter even when the directory is a real adapter. A symlinked adapter path or a symlinked marker is
refused the same way. The command prints the ollama create line and
does not run it. When the adapter Modelfile is not already on disk,
local-seat is print-only. It prints the Modelfile
and does not write that file. Write that file from the printed
contents before ollama create. When FROM is the seat tag and ADAPTER
already names that directory, the report uses that on-disk Modelfile.
llama.cpp does not load that adapter directory in one
line. --runtime llama.cpp with --adapter is refuse:runtime. Another
runtime name is refuse:runtime after the shape checks.

  estate enrich local-seat \\
    --prepared .cell/enrich/overnight-traces/llamafactory-qlora \\
    --adapter .cell/enrich/overnight-traces/llamafactory-qlora/outputs

Page: docs/local-seat.md. READY_FOR_LIVE_TEST stays no.

Qwen QLoRA journey (Target C)
-----------------------------
Popular path. Prepare llamafactory-qlora, run the NEXT.md train and
export lines outside this factory, print the GGUF convert, print the
Ollama create, then record the artifact shape. The seat tag and the
train base stay separate. A 5090 smoke seated llama3 and trained
Qwen/Qwen2.5-0.5B-Instruct. template is qwen. model_name_or_path is
that train base. quantization_bit is 4 and quantization_method is bnb.
A missing train base or a bare seat tag is refuse:train-base and
writes nothing. This factory does not map the seat tag onto a Hub
repo and does not download weights.

  estate enrich prepare --estate <your-estate.yaml> \\
    --pack <pack-id> --driver llamafactory-qlora --job train \\
    --state-dir .cell

Run llamafactory-cli train and llamafactory-cli export from NEXT.md
on a CUDA host. This factory does not run them.

  estate enrich gguf-convert \\
    --prepared .cell/enrich/<pack-id>/llamafactory-qlora \\
    --weights .cell/enrich/<pack-id>/llamafactory-qlora/export

That prints python3 convert_hf_to_gguf.py with --outtype auto and an
outfile beside the export directory. It does not convert. A missing
export directory is refuse:seat. A JSON list or JSON null
extra_special_tokens, or a Qwen-family export missing vocab.json or
merges.txt, is refuse:tokenizer. Copy the tokenizer files from the
HF cache snapshot for the train base, or the equivalent base checkout,
into the export directory.
HF hub snapshots are often symlinks into the HF cache.
Copy with dereference (cp -aL or cp --dereference, or the equivalent)
so the files in the export directory are real files, not symlinks.
A plain cp -a leaves tokenizer_config.json as a symlink.
enrich does not follow a symlinked tokenizer_config.json. Keep the
export tokenizer_config.json as tokenizer_config.json.bak. Then re-run
estate enrich gguf-convert.

  estate enrich local-seat \\
    --prepared .cell/enrich/<pack-id>/llamafactory-qlora \\
    --weights .cell/enrich/<pack-id>/llamafactory-qlora/export.gguf

That prints ollama create for cell-enrich-<pack-id>. On that GGUF
path, local-seat is print-only. It prints the Modelfile and does not write
$PREPARED/Modelfile. Write that file from the printed contents before
ollama create. When --weights
is the GGUF it also prints llama-cli -m and llama-server -m for that
file. --runtime llama.cpp selects those lines. It does not create the
model and does not run llama.cpp. After you write that file, run the
printed ollama create line yourself. The same step stands when you already
ran ollama create outside this factory. This factory did not run
ollama create. The standing next step records that GGUF.

  estate enrich import-trained --estate <your-estate.yaml> \\
    --prepared .cell/enrich/<pack-id>/llamafactory-qlora \\
    --tag cell-enrich-<pack-id> \\
    --adapter .cell/enrich/<pack-id>/llamafactory-qlora/export.gguf

import-trained records trained_shape and trained_paths. The GGUF
shape is gguf. The proposal stays auto_apply=false. outputs/ records
adapter. export/ records merged. import-trained does not apply the
estate and does not promote. Opt-in ladder check:
make qlora-journey. It prints this ladder, checks the prepare
artifacts, and prints SKIP live train. It does not run a trainer
and does not convert. Not in make smoke, make gate-90, or Actions.
The train step is make train-next. It prepares the same card and
prints the NEXT.md llamafactory-cli train line. It prints
SKIP live train. CELL_TRAIN_LIVE=1 stays print-only. It does not
run a trainer. make uniqueness-ladder does not run it.
make uniqueness-full runs make qlora-journey, then make train-next,
then make seat-journey. It does not train. make uniqueness-ladder
stays qlora-journey then seat-journey.
make uniqueness-prove-checklist prints the recorded Target C operator
steps from docs/LIVE-PROBES.md. It does not train, convert, shell out
to ollama, or promote. It does not invent a live PASS.
After step 8 (import-trained, trained_shape gguf, auto_apply=false)
it prints Standing next (estate). The proposal stays auto_apply=false.
The factory does not apply the estate without an explicit operator
--require-plan path. No promote and no auto-promote. The coda names
plan, apply --require-plan, and reconcile and does not execute them.
The re-prove card is make uniqueness-prove-checklist.
make purpose-build-checklist is the print-only operator path for purpose-building an SLM on demand (operator section 15). It points
at the print-only cards already on tip, including make mlx-lm-lora-journey, make deepseek-r1-distill-journey, make uniqueness-deepseek, make glm4-chat-journey, and make uniqueness-glm, plus the LoRA twins, and does not run them. After
import-trained (trained_shape gguf, auto_apply=false) it prints
Standing next (estate). The coda names apply-proposal, plan,
apply --require-plan, and reconcile and does not execute them.
It does not invent a live PASS. The recorded PASS stays the only
live uniqueness prove. Walk: docs/operator-enrich-journeys.md (section 15).
make purpose-build-pick is the print-only host and stack picker for purpose-build journeys (operator section 17).
It names the same cards by host and does not run them. Nvidia / CUDA primary is
lf-beachhead-prepare, qlora-journey, and uniqueness-full. Unsloth stays optional.
Axolotl stays integration. Apple Silicon is mlx-lm-lora-journey and uniqueness-mlx.
A stock any-affinity pack is refuse:host for mlx-lm-lora. Target A LoRA twins
are the LLaMA-Factory, Unsloth, and Axolotl pairs that already exist.
Walk: docs/operator-enrich-journeys.md (section 17).
make deepseek-r1-distill-journey is the print-only DeepSeek-R1-Distill chat QLoRA journey (operator section 19).
make uniqueness-deepseek is the print-only chain of that journey.
make deepseek-r1-distill-lora-journey and make uniqueness-deepseek-lora are the LoRA twin.
Seat tag llama3. Train base deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B. Template deepseekr1.
They do not train, merge, convert, shell out to ollama, or promote.
They do not invent a live PASS. Not in make smoke, make gate-90, or Actions.
READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 19).
make glm4-chat-journey is the print-only GLM-4 Chat QLoRA journey (operator section 20).
make uniqueness-glm is the print-only chain of that journey. The LoRA twin is
make glm4-chat-lora-journey and make uniqueness-glm-lora.
Seat tag llama3. Train base zai-org/glm-4-9b-chat. Template glm4.
They do not train, merge, convert, shell out to ollama, or promote.
They do not invent a live PASS. Not in make smoke, make gate-90, or Actions.
READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 20).
When an SLM fits mid-software-build, or on demand, make purpose-build-journey is the print-only purpose-build on-demand entry (operator section 18).
It runs make purpose-build-pick, then make purpose-build-checklist. It does not inline those bodies.
It does not invent a live PASS. The recorded PASS stays the only live uniqueness prove.
Walk: docs/operator-enrich-journeys.md (section 18).
CELL_TRAIN_LIVE=1 stays print-only. CELL_SEAT_LIVE=1 stays print-only.
Not native MLX. Not in make smoke, make gate-90, or Actions.
READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 8, Target C).

Qwen LoRA journey (Target A)
----------------------------
Unquantized path. Prepare llamafactory-lora, run the NEXT.md train and
export lines outside this factory, print the merge, print the GGUF
convert, print the Ollama create, then record the artifact shape. The
seat tag and the train base stay separate. A 5090 smoke seated llama3
and trained Qwen/Qwen2.5-0.5B-Instruct on this card, then exported and
seated that gauge from the files prepare wrote. template is qwen.
model_name_or_path is that train base. finetuning_type is lora.
lora_rank is 8. packing is false. quantization_bit and
quantization_method stay off the recipe. This path does not require
bitsandbytes. A missing train base or a bare seat tag is
refuse:train-base and writes nothing. This factory does not map the
seat tag onto a Hub repo and does not download weights.

  estate enrich prepare --estate <your-estate.yaml> \\
    --pack <pack-id> --driver llamafactory-lora --job train \\
    --state-dir .cell

Run llamafactory-cli train and llamafactory-cli export from NEXT.md
on a CUDA host. This factory does not run them.

  estate enrich merge-adapt \\
    --prepared .cell/enrich/<pack-id>/llamafactory-lora \\
    --adapter .cell/enrich/<pack-id>/llamafactory-lora/outputs

That prints llamafactory-cli export for export.yaml and the keys from
examples/merge_lora/qwen3_lora_sft.yaml. It does not merge.

  estate enrich gguf-convert \\
    --prepared .cell/enrich/<pack-id>/llamafactory-lora \\
    --weights .cell/enrich/<pack-id>/llamafactory-lora/export

That prints python3 convert_hf_to_gguf.py with --outtype auto and an
outfile beside the export directory. It does not convert. A missing
export directory is refuse:seat. A JSON list or JSON null
extra_special_tokens, or a Qwen-family export missing vocab.json or
merges.txt, is refuse:tokenizer. Copy the tokenizer files from the
HF cache snapshot for the train base, or the equivalent base checkout,
into the export directory.
HF hub snapshots are often symlinks into the HF cache.
Copy with dereference (cp -aL or cp --dereference, or the equivalent)
so the files in the export directory are real files, not symlinks.
A plain cp -a leaves tokenizer_config.json as a symlink.
enrich does not follow a symlinked tokenizer_config.json. Keep the
export tokenizer_config.json as tokenizer_config.json.bak. Then re-run
estate enrich gguf-convert.

  estate enrich local-seat \\
    --prepared .cell/enrich/<pack-id>/llamafactory-lora \\
    --weights .cell/enrich/<pack-id>/llamafactory-lora/export.gguf

That prints ollama create for cell-enrich-<pack-id>. On that GGUF
path, local-seat is print-only. It prints the Modelfile and does not write
$PREPARED/Modelfile. Write that file from the printed contents before
ollama create. When --weights
is the GGUF it also prints llama-cli -m and llama-server -m for that
file. --runtime llama.cpp selects those lines. It does not create the
model and does not run llama.cpp. After you write that file, run the
printed ollama create line yourself. The same step stands when you already
ran ollama create outside this factory. This factory did not run
ollama create. The standing next step records that GGUF.

  estate enrich import-trained --estate <your-estate.yaml> \\
    --prepared .cell/enrich/<pack-id>/llamafactory-lora \\
    --tag cell-enrich-<pack-id> \\
    --adapter .cell/enrich/<pack-id>/llamafactory-lora/export.gguf

import-trained records trained_shape and trained_paths. The GGUF
shape is gguf. The proposal stays auto_apply=false. outputs/ records
adapter. export/ records merged. import-trained does not apply the
estate and does not promote. Opt-in ladder check:
make lora-journey. It prints this ladder, checks the prepare
artifacts, and prints SKIP live train. It does not run a trainer,
does not merge, and does not convert. Not in make smoke, make gate-90,
or Actions. READY_FOR_LIVE_TEST stays no.
The train step is make train-next-lora. It prepares the same
llamafactory-lora card and prints the NEXT.md train and export lines.
It does not print a bitsandbytes install. It prints SKIP live train.
CELL_TRAIN_LIVE=1 stays print-only. It does not run a trainer.
make seat-journey-lora is that seat print on the LoRA card. It keeps
refuse:adapter, refuse:tokenizer, and refuse:seat, then prints merge,
convert, seat, and import against the same fixture stubs.
make uniqueness-full-lora runs make lora-journey, then make
train-next-lora, then make seat-journey-lora. It does not train.
make uniqueness-full stays the QLoRA chain. Not in make smoke,
make gate-90, or Actions.
Walk: docs/operator-enrich-journeys.md (section 9, Target A).

Target C seat ladder
--------------------
Opt-in print path for the same Qwen QLoRA card once a merged export
and a GGUF exist. make qlora-journey leaves those commands at
refuse:seat. make seat-journey prepares llamafactory-qlora (seat
llama3, train base Qwen/Qwen2.5-0.5B-Instruct) on a throwaway estate
copy, writes fixture stubs, and prints the lines. The adapter stub
is outputs/adapter_config.json. The merged stub is export/config.json
plus export/model.safetensors. The GGUF stub starts with GGUF magic.
An empty file is refuse:seat. Before the good merged stub, the script
writes a 5090-shaped export. config.json sets model_type qwen2 and
architectures Qwen2ForCausalLM. tokenizer_config.json sets
extra_special_tokens to a JSON list. vocab.json and merges.txt are
missing. gguf-convert on that directory is refuse:tokenizer and does
not print the convert line. The refuse names restoring tokenizer files
from the HF cache snapshot for the train base, or the equivalent base
checkout, into the export directory.
HF hub snapshots are often symlinks into the HF cache.
Copy with dereference (cp -aL or cp --dereference, or the equivalent)
so the files in the export directory are real files, not symlinks.
A plain cp -a leaves tokenizer_config.json as a symlink.
enrich does not follow a symlinked tokenizer_config.json.
Then re-run estate enrich gguf-convert. The
script then replaces that fixture with config.json {} and
model.safetensors. It does not copy tokenizer files and does not
write tokenizer_config.json.bak.

  estate enrich merge-adapt \\
    --prepared <prepared> --adapter <prepared>/outputs

That prints llamafactory-cli export. It does not merge.

  estate enrich gguf-convert \\
    --prepared <prepared> --weights <prepared>/export

That prints python3 convert_hf_to_gguf.py with --outtype auto.
It does not convert.

  estate enrich local-seat \\
    --prepared <prepared> --weights <prepared>/export.gguf

That prints ollama create for cell-enrich-<pack-id>, plus
llama-cli -m and llama-server -m. On that GGUF path, local-seat is
print-only. It prints the Modelfile and does not write
$PREPARED/Modelfile. Write that file from the printed contents before
ollama create. It does not create the
model. After you write that file, run the printed ollama create line
yourself. The same step stands when you already ran ollama create outside
this factory. This factory did not run ollama create. The standing
next step records that GGUF.

  estate enrich import-trained --estate <your-estate.yaml> \\
    --prepared <prepared> --tag cell-enrich-<pack-id> \\
    --adapter <prepared>/export.gguf

import-trained records trained_shape gguf. The proposal stays
auto_apply=false. import-trained does not apply the estate.
The script prints SKIP live train, SKIP live convert, and
SKIP live seat. CELL_SEAT_LIVE=1 does not run those programs.
Not in make smoke, make gate-90, or Actions.
READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 10, Target C seat ladder).

Axolotl QLoRA journey
---------------------
Print-only popular-config parity for axolotl-qlora.
make axolotl-qlora-journey prepares that card on a throwaway copy of
examples/estate.yaml. Seat tag llama3. Train base Qwen/Qwen2.5-0.5B-Instruct.
axolotl.yml matches examples/llama-3/qlora.yml: adapter qlora, load_in_4bit
true, sequence_len 4096, lora_r 32. base_model is that train base.
A seat tag with no train base is refuse:train-base. A missing adapter is
refuse:adapter. A missing merged directory or GGUF is refuse:seat.
Before the good merged stub, a Qwen-shaped outputs/merged is refuse:tokenizer.
The good stub is config.json {} plus model.safetensors (not adapter_model).
The GGUF stub starts with GGUF magic. The script prints axolotl merge-lora,
including --dequant, then gguf-convert, local-seat, and import-trained.
It prints SKIP live train, SKIP live convert, and SKIP live seat.
It does not run axolotl, does not convert, does not run ollama, and does
not promote. make uniqueness-axolotl runs the prepare-assert phase, then
the seat-print phase. Both stay print-only. Not in make smoke, make gate-90,
or Actions. READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 11).

Unsloth QLoRA journey
---------------------
Print-only optional NEXT path for unsloth-qlora. Status stays optional.
make unsloth-qlora-journey prepares that card on a throwaway copy of
examples/estate.yaml. Seat tag llama3. Train base Qwen/Qwen2.5-0.5B-Instruct.
The card writes UNSLOTH.md, PREPARE.md, NEXT.md, and prepare.json.
It does not write a script, a recipe, or dataset.jsonl.
A seat tag with no train base is refuse:train-base.
A missing adapter, or an adapter directory without adapter_model.safetensors,
is refuse:adapter. A merged directory or a GGUF passed as --adapter is
refuse:adapter. local-seat --adapter on this card is refuse:adapter.
A missing merged directory or GGUF is refuse:seat.
Before the good merged stub, a Qwen-shaped merged directory beside the
prepare is refuse:tokenizer. The adapter stub is adapter_config.json plus
adapter_model.safetensors. The good merged stub is config.json {} plus
model.safetensors. The GGUF stub starts with GGUF magic.
The script prints save_pretrained_merged with save_method merged_16bit,
then gguf-convert (including the three manual outtypes), local-seat, and
import-trained. import-trained records trained_shape gguf. The proposal
stays auto_apply=false. It prints SKIP live train, SKIP live convert, and
SKIP live seat. It does not call Unsloth, does not convert, does not run
ollama, and does not promote. make uniqueness-unsloth runs the
prepare-assert phase, then the seat-print phase. Both stay print-only.
Not in make smoke, make gate-90, or Actions. READY_FOR_LIVE_TEST stays no.
Walk: docs/operator-enrich-journeys.md (section 12).

Axolotl LoRA journey
--------------------
Print-only popular-config parity for axolotl-lora.
make axolotl-lora-journey prepares that card on a throwaway copy of
examples/estate.yaml. Seat tag llama3. Train base Qwen/Qwen2.5-0.5B-Instruct.
axolotl.yml matches examples/llama-3/lora-1b.yml: adapter lora,
load_in_8bit false, load_in_4bit false, sequence_len 2048, lora_r 16.
base_model is that train base.
A seat tag with no train base is refuse:train-base. A missing adapter is
refuse:adapter. A missing merged directory or GGUF is refuse:seat.
Before the good merged stub, a Qwen-shaped outputs/merged is refuse:tokenizer.
The good stub is config.json {} plus model.safetensors (not adapter_model).
The GGUF stub starts with GGUF magic. The script prints axolotl merge-lora
without --dequant, then gguf-convert, local-seat, and import-trained.
import-trained records trained_shape gguf. The proposal stays auto_apply=false.
It prints SKIP live train, SKIP live convert, and SKIP live seat.
It does not run axolotl, does not convert, does not run ollama, and does
not promote. make uniqueness-axolotl-lora runs the prepare-assert phase, then
the seat-print phase. It does not run make uniqueness-axolotl.
Both stay print-only. Not in make smoke, make gate-90,
or Actions. READY_FOR_LIVE_TEST stays no. This is not a live PASS.
Walk: docs/operator-enrich-journeys.md (section 13).

Unsloth LoRA journey
--------------------
Print-only optional NEXT path for unsloth-lora. Status stays optional.
It is the non-quant twin of unsloth-qlora. It does not write load_in_4bit.
make unsloth-lora-journey prepares that card on a throwaway copy of
examples/estate.yaml. Seat tag llama3. Train base Qwen/Qwen2.5-0.5B-Instruct.
The card writes UNSLOTH.md, PREPARE.md, NEXT.md, and prepare.json.
It does not write a script, a recipe, or dataset.jsonl.
A seat tag with no train base is refuse:train-base.
--official-scale on this card alone is refuse:official-scale.
--from-feed on this card alone is refuse:dataset.
A missing adapter, or an adapter directory without adapter_model.safetensors,
is refuse:adapter. A merged directory or a GGUF passed as --adapter is
refuse:adapter. local-seat --adapter on this card is refuse:adapter.
A missing merged directory or GGUF is refuse:seat.
Before the good merged stub, a Qwen-shaped merged directory beside the
prepare is refuse:tokenizer. The script prints save_pretrained_merged
with save_method merged_16bit, and the documented LoRA save
(save_method lora), then gguf-convert, local-seat, and import-trained.
import-trained records trained_shape gguf. The proposal stays auto_apply=false.
It prints SKIP live train, SKIP live convert, and SKIP live seat.
It does not call Unsloth, does not convert, does not run ollama, and does
not promote. make uniqueness-unsloth-lora runs the prepare-assert phase,
then the seat-print phase. It does not run make unsloth-qlora-journey or
make uniqueness-unsloth. Both stay print-only. Not in make smoke,
make gate-90, or Actions. READY_FOR_LIVE_TEST stays no. This is not a live PASS.
Walk: docs/operator-enrich-journeys.md (section 14).

mlx-lm LoRA journey
-------------------
make mlx-lm-lora-journey is the print-only Apple Silicon mlx-lm LoRA journey (operator section 16).
make uniqueness-mlx is the print-only chain of that journey.
Print-only optional NEXT path for mlx-lm-lora. Status stays optional.
Apple Silicon affinity only. Another host is refuse:host and writes nothing.
make mlx-lm-lora-journey prepares that card on a throwaway copy of
examples/estate.yaml. Seat tag llama3. Train base Qwen/Qwen2.5-0.5B-Instruct.
The throwaway pack sets host_class_affinity to apple-silicon. The stock
overnight pack stays any and is refuse:host.
The card writes MLX.md, PREPARE.md, NEXT.md, and prepare.json.
It does not write a script, a recipe, or dataset.jsonl.
A seat tag with no train base is refuse:train-base.
--official-scale on this card alone is refuse:official-scale.
--from-feed on this card alone is refuse:dataset.
A missing adapter, a checkpoint without adapters.safetensors, or
adapter_model.safetensors is refuse:adapter. A fused MLX directory or a
GGUF passed as --adapter is refuse:adapter. local-seat --adapter on this
card is refuse:adapter. A missing GGUF is refuse:seat. gguf-convert stays
refuse:seat. The adapter stub is adapter_config.json plus adapters.safetensors.
The GGUF stub is fused_model/ggml-model-f16.gguf and starts with GGUF magic.
The script prints mlx_lm.fuse with --adapter-path, --save-path, and
--export-gguf, then local-seat and import-trained. import-trained records
trained_shape gguf. A fused MLX directory does not record trained_shape merged.
The proposal stays auto_apply=false. It prints SKIP live train, SKIP live
convert, and SKIP live seat. It does not call mlx-lm, does not fuse, does
not convert, does not run ollama, and does not promote.
make uniqueness-mlx runs the prepare-assert phase, then the seat-print phase.
It does not run make unsloth-qlora-journey or make uniqueness-unsloth.
Both stay print-only. Not in make smoke, make gate-90, or Actions.
READY_FOR_LIVE_TEST stays no. This is not a live PASS. Not native MLX.
Walk: docs/operator-enrich-journeys.md (section 16).

Docs: docs/TRAIN-ENRICH.md and docs/LIVE-PROBES.md.
Words: docs/UBIQUITOUS_LANGUAGE.md.
Journeys: docs/operator-enrich-journeys.md.
";

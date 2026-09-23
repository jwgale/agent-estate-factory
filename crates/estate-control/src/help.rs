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
            print!("{ENRICH}");
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
binding count. Cloud-agent
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

The plan line prints the bound frontier model, or model=- when the binding
sets none. It does not copy the schema card. An estate with no frontier
binding refuses (refuse:frontier-invent) instead of inventing a frontier
source_driver or catalog model. No live key.
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

One-box Agent Estate Factory — plan/apply IaC, sacred isolation
(Cyera CI + Rust classroom out; Sanctum is not Cyera), equal-class
frontier+local, manual enrich packs.

Suite (first-class):
- A local runtime is an ecosystem seat. Ollama is today's entrant. Catalog / route / bind take the next process.
- Integrate the driver. A from-scratch local server waits until the entrant does not already do the job.
- Facilitate train/enrich of purpose-built small-parameter models. Open-source SLMs get more common.
- Beachhead: curator packs, the specialist path, and TrainEnrichDriver.
- estate enrich prepare writes artifacts. llamafactory-lora writes a LLaMA-Factory LoRA recipe (no quantization). llamafactory-qlora writes the QLoRA recipe. axolotl-lora writes the bf16 Axolotl YAML. axolotl-qlora writes the 4-bit Axolotl YAML. unsloth-qlora is an optional Nvidia-only NEXT handoff and does not write a script. This page does not run a trainer.

Anti-shrink:
- Not a gateway. Not an MCP catalog.
- Not an Ollama wrapper-as-product. Not LM Studio-alone.
- Not a Grok Bot clone. Not an agent farm.
- Not frontier-proxy-only. Not a local-studio-only shrink.
- Enrich packs stay manual. The curator accepts by hand.

Pointers:
  charter.md
  docs/UBIQUITOUS_LANGUAGE.md
  make gate-90
  make day90
  docs/LIVE-PROBES.md

This page does not plan, apply, or probe. Off make smoke,
make gate-90, and Actions.
";

const ENRICH: &str = "\
enrich — train/enrich prepare
=============================
estate help train prints this page. Job field is train or enrich.
Default job is enrich. llamafactory-lora, llamafactory-qlora, axolotl-lora, axolotl-qlora, and unsloth-qlora default to train.
--all-drivers defaults to enrich and includes a card only when that job
is allowed. Prepare writes artifacts. It does not train.

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
default job train), and unsloth-qlora (optional NEXT card, status optional,
Nvidia-only QLoRA handoff; writes UNSLOTH.md; does not write a script,
a recipe, or dataset.jsonl; does not call Unsloth; default job train).
A later entrant adds one catalog card. Floor and control dispatch
do not match driver ids. --all-drivers prepares every card the job
allows, into sibling directories. Omit --driver on prepare for the
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
examples/fixtures/phi3-instruct.pack.json is the Phi-3 smoke pack.
--driver llamafactory-qlora on that pack is a reproduce target beside
Qwen LoRA/QLoRA and still writes quantization_method bnb and quantization_bit 4.
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
is still refuse:max-steps.

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
llamafactory-qlora, axolotl-lora, axolotl-qlora, or unsloth-qlora prepare whose job is train. --adapter
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
than one, is refuse:adapter and writes no proposal. prepare.json and
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
LLaMA-Factory LoRA and QLoRA recipes and Axolotl LoRA and QLoRA recipes
under /tmp and does not run either trainer. Neither is part of
make smoke, make gate-90, or Actions. make enrich-live-prove runs ollama create on a throwaway
cell when the seat is up, then removes the tag. It is an opt-in seated
handoff. It is not a factory-wide live test. READY_FOR_LIVE_TEST stays no.
estate enrich gguf-convert prints the llama.cpp convert line for a
merged LLaMA-Factory export directory (config.json and at least one
.safetensors file whose name does not start with adapter_model).
An adapter directory, a symlinked weights path, a symlinked marker,
and a path that is already a GGUF are refuse:seat. The printed line is:

  python3 convert_hf_to_gguf.py <merged-dir> --outfile <sibling>.gguf --outtype auto

--outtype auto is that script's default (highest-fidelity 16-bit float).
The outfile is a sibling of the merged directory. The command then
points at local-seat for that file. It does not run llama.cpp, does
not write a GGUF, and does not choose a quantization type.

  estate enrich gguf-convert \\
    --prepared .cell/enrich/overnight-traces/llamafactory-qlora \\
    --weights .cell/enrich/overnight-traces/llamafactory-qlora/export

estate enrich local-seat validates a merged LLaMA-Factory export
directory (config.json and at least one .safetensors file whose name
does not start with adapter_model, optional Modelfile) or a .gguf file.
adapter_model*.safetensors is not merged evidence. A symlinked weights
path or a symlinked marker is refuse:seat. It prints the ollama create
line. For a GGUF it also prints the Modelfile whose FROM is that file.
The create name is cell-enrich-{pack}. The seat tag is prepare.json
seat_tag. The command does not run ollama or llama.cpp. GGUF conversion stays
llama.cpp convert_hf_to_gguf.py, outside this factory. import-trained
records that same merged directory or GGUF on the local_slm proposal.
local-seat does not promote.

  estate enrich local-seat \\
    --prepared .cell/enrich/overnight-traces/llamafactory-qlora \\
    --weights .cell/enrich/overnight-traces/llamafactory-qlora/export

Page: docs/local-seat.md. READY_FOR_LIVE_TEST stays no.
Docs: docs/TRAIN-ENRICH.md and docs/LIVE-PROBES.md.
Words: docs/UBIQUITOUS_LANGUAGE.md.
Journeys: docs/operator-enrich-journeys.md.
";

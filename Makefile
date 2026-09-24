.PHONY: validate plan apply apply-gated apply-dry-run drift models catalog catalog-dump supervisor proxy-check gate gate-60 gate-90 day90 day90-mixed pause-stop pause-start pause-status test check task-mock suspend resume status plans feed-pack feed-import feed-list leases audits history probes feed-cursor floor-suspend floor-resume floor-history operator-day convey packs-list packs-index reconcile packs-propose audit-export expire doctor doctor-strict fixtures-check sessions plan-diff smoke backup restore pause-proof policy-check feed-loop live-specialist real-world enrich-prepare enrich-live-prove train-prepare qlora-journey lora-journey seat-journey lf-beachhead-prepare uniqueness-ladder uniqueness-full uniqueness-full-lora uniqueness-prove-checklist train-next train-next-lora seat-journey-lora axolotl-qlora-journey uniqueness-axolotl unsloth-qlora-journey uniqueness-unsloth axolotl-lora-journey uniqueness-axolotl-lora unsloth-lora-journey uniqueness-unsloth-lora mlx-lm-lora-journey uniqueness-mlx purpose-build-checklist purpose-build-pick purpose-build-journey deepseek-r1-distill-journey uniqueness-deepseek deepseek-r1-distill-lora-journey uniqueness-deepseek-lora glm4-chat-journey uniqueness-glm glm4-chat-lora-journey uniqueness-glm-lora classify-prepare classify-eval tev1-journey deepseek-classify-journey

ESTATE ?= examples/estate.yaml
STATE ?= .cell

validate:
	cargo run -q -p estate-control -- validate --estate $(ESTATE)

plan:
	cargo run -q -p estate-control -- plan --estate $(ESTATE) --plans-dir plans

apply:
	cargo run -q -p estate-control -- apply --estate $(ESTATE) --state-dir $(STATE) --roots-base .

apply-gated:
	cargo run -q -p estate-control -- apply --estate $(ESTATE) --state-dir $(STATE) --roots-base . --require-plan

apply-dry-run:
	cargo run -q -p estate-control -- apply --dry-run --estate $(ESTATE) --state-dir $(STATE) --roots-base .

drift:
	cargo run -q -p estate-control -- drift --estate $(ESTATE) --state-dir $(STATE)

models:
	cargo run -q -p estate-control -- models --estate $(ESTATE)

catalog:
	cargo run -q -p model-estate -- catalog

catalog-dump:
	cargo run -q -p estate-control -- catalog --out $(STATE)/catalog.json

supervisor:
	cargo run -q -p floor-supervisor -- apply --estate $(ESTATE) --state-dir $(STATE) --roots-base .

proxy-check:
	cargo run -q -p conveyor-proxy -- check --estate $(ESTATE) --agent horizon --kind memory_read --object lane:research; true

pause-stop:
	./scripts/pause-kit.sh stop

pause-start:
	./scripts/pause-kit.sh start

pause-status:
	./scripts/pause-kit.sh status

gate:
	./scripts/day30-gate.sh

gate-60:
	./scripts/day60-gate.sh

# Thin alias: smoke (includes day90) + doctor --strict + GATE-90 checklist.
# Local only. Do not add to GitHub Actions.
gate-90:
	./scripts/day90-gate.sh

day90:
	./scripts/day90.sh

# Mixed frontier+local fixture: plan → apply --require-plan.
# Isolated cell. Fixtures only. Do not add to smoke or GitHub Actions.
day90-mixed:
	bash scripts/day90-mixed.sh

suspend:
	cargo run -q -p estate-control -- suspend --state-dir $(STATE)

resume:
	cargo run -q -p estate-control -- resume --estate $(ESTATE) --state-dir $(STATE)

status:
	cargo run -q -p estate-control -- status --estate $(ESTATE) --state-dir $(STATE)

plans:
	cargo run -q -p estate-control -- plans --plans-dir plans

feed-pack:
	cargo run -q -p estate-control -- feed pack --feed-dir $(STATE)/feed --drop-dir packs

feed-list:
	cargo run -q -p estate-control -- feed list --drop-dir packs

feed-import:
	cargo run -q -p estate-control -- feed import --id overnight-traces --drop-dir packs --accepted-dir packs/accepted --estate $(ESTATE)

leases:
	cargo run -q -p estate-control -- leases --state-dir $(STATE)

audits:
	cargo run -q -p estate-control -- audits --state-dir $(STATE)

floor-suspend:
	cargo run -q -p floor-supervisor -- suspend --state-dir $(STATE)

floor-resume:
	cargo run -q -p floor-supervisor -- resume --estate $(ESTATE) --state-dir $(STATE) --roots-base .

history:
	cargo run -q -p estate-control -- history --state-dir $(STATE)

probes:
	cargo run -q -p estate-control -- probes

feed-cursor:
	cargo run -q -p estate-control -- feed cursor --feed-dir $(STATE)/feed

floor-history:
	cargo run -q -p floor-supervisor -- history --state-dir $(STATE)

operator-day:
	./scripts/operator-day.sh

convey:
	cargo run -q -p estate-control -- convey sync --state-dir $(STATE)
	cargo run -q -p estate-control -- convey list --state-dir $(STATE)

packs-list:
	cargo run -q -p estate-control -- packs list --drop-dir packs

packs-index:
	cargo run -q -p estate-control -- packs index --drop-dir packs

reconcile:
	cargo run -q -p estate-control -- reconcile --estate $(ESTATE) --state-dir $(STATE)

packs-propose:
	cargo run -q -p estate-control -- packs propose --id overnight-traces --drop-dir packs --accepted-dir packs/accepted --proposed-dir packs/proposed --estate $(ESTATE)

audit-export:
	cargo run -q -p estate-control -- audit export --estate $(ESTATE) --state-dir $(STATE) --plans-dir plans --packs-dir packs --out $(STATE)/audit-export --tar

expire:
	cargo run -q -p estate-control -- expire --state-dir $(STATE)

doctor:
	cargo run -q -p estate-control -- doctor --root . --state-dir $(STATE)

doctor-strict:
	cargo run -q -p estate-control -- doctor --strict --root . --state-dir $(STATE)

# Fixtures only: scrubbed trace → pack → propose → accept.
# Local only. Do not add to smoke or GitHub Actions.
feed-loop:
	./scripts/feed-loop.sh

fixtures-check:
	./scripts/fixtures-check.sh

sessions:
	cargo run -q -p estate-control -- sessions list --state-dir $(STATE)

plan-diff:
	cargo run -q -p estate-control -- plan diff --estate $(ESTATE) --state-dir $(STATE) --plans-dir plans

task-mock:
	cargo run -q -p model-estate -- task --estate $(ESTATE) --agent horizon --act model --object xai_grok --mock

smoke:
	./scripts/smoke.sh

# Opt-in live specialist. Requires CELL_LOCAL_ENDPOINT.
# Local only. Do not add to smoke or GitHub Actions.
live-specialist:
	./scripts/live-specialist.sh

# Opt-in real-world kit: check + vanilla doctor; live SKIP without CELL_LOCAL_ENDPOINT.
# Local only. Do not add to smoke or GitHub Actions.
real-world:
	bash scripts/real-world.sh

# Opt-in enrich prepare through apply-proposal, plan, and require-plan apply.
# No live train. Local only. Do not add to smoke, gate-90, or GitHub Actions.
enrich-prepare:
	bash scripts/enrich-prepare.sh

# Opt-in seated-runtime enrich handoff. Runs ollama create when the seat is up.
# Not a factory-wide live test. Local only. Do not add to smoke, gate-90, or GitHub Actions.
enrich-live-prove:
	bash scripts/enrich-live-prove.sh

# Opt-in LLaMA-Factory recipe and Axolotl recipe. Does not run either trainer.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
train-prepare:
	bash scripts/train-prepare.sh

# Opt-in Target C ladder: Qwen / LLaMA-Factory QLoRA prepare artifacts.
# Prints the ladder. Does not train, convert, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
qlora-journey:
	bash scripts/qlora-journey.sh

# Opt-in Target A ladder: Qwen / LLaMA-Factory LoRA prepare artifacts.
# Prints the ladder. Does not train, merge, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
lora-journey:
	bash scripts/lora-journey.sh

# Opt-in Target C seat ladder: refuse:tokenizer, then fixture stubs for merge, convert, seat, and import.
# Prints the lines. Does not train, merge, convert, seat, or promote.
# Pins SEAT_CARD to llamafactory-qlora so an ambient card cannot retarget it.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
seat-journey:
	SEAT_CARD=llamafactory-qlora bash scripts/seat-journey.sh

# Opt-in Target A seat ladder: the same script as seat-journey on llamafactory-lora.
# Same fixture stubs. Same refuse:adapter, refuse:tokenizer, and refuse:seat.
# Does not train, merge, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
seat-journey-lora:
	SEAT_CARD=llamafactory-lora bash scripts/seat-journey.sh

# Opt-in print-only prepare of every LLaMA-Factory beachhead matrix row.
# Reads docs/lf-beachhead-matrix.md. Does not train, merge, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
lf-beachhead-prepare:
	bash scripts/lf-beachhead-prepare.sh

# Opt-in print-only Target C uniqueness chain: qlora-journey then seat-journey.
# Does not run train-next. Does not train, merge, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-ladder:
	bash scripts/uniqueness-ladder.sh

# Opt-in print-only Target C full uniqueness print chain:
# qlora-journey, then train-next, then seat-journey.
# Does not train, merge, convert, seat, or promote.
# Does not change uniqueness-ladder (that chain does not run train-next).
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-full:
	bash scripts/uniqueness-full.sh

# Opt-in print-only Target A full uniqueness print chain:
# lora-journey, then train-next-lora, then seat-journey-lora.
# Does not train, merge, convert, seat, or promote.
# Does not change uniqueness-full or uniqueness-ladder.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-full-lora:
	bash scripts/uniqueness-full-lora.sh

# Opt-in print-only operator checklist for the recorded Target C live uniqueness ladder.
# Prints ordered steps from docs/LIVE-PROBES.md. After step 8 (import-trained) it prints
# Standing next (estate): auto_apply=false, no promote, and the existing plan / apply /
# reconcile entrypoints. Print only. Does not execute them.
# Does not train, convert, shell out to ollama, or promote.
# Does not invent a live PASS. CELL_TRAIN_LIVE=1 and CELL_SEAT_LIVE=1 stay print-only.
# Not native MLX.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-prove-checklist:
	bash scripts/uniqueness-prove-checklist.sh

# Opt-in print-only Target C train step: after llamafactory-qlora prepare,
# print the NEXT.md train recipe. Does not train, merge, convert, seat, or promote.
# Pins TRAIN_CARD to llamafactory-qlora so an ambient card cannot retarget it.
# CELL_TRAIN_LIVE=1 stays print-only.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
train-next:
	TRAIN_CARD=llamafactory-qlora bash scripts/train-next.sh

# Opt-in print-only Target A train step: after llamafactory-lora prepare,
# print the NEXT.md train and export lines. No bitsandbytes install line.
# Does not train, merge, convert, seat, or promote.
# CELL_TRAIN_LIVE=1 stays print-only.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
train-next-lora:
	TRAIN_CARD=llamafactory-lora bash scripts/train-next.sh

# Opt-in print-only Axolotl QLoRA journey. Checks axolotl.yml against
# examples/llama-3/qlora.yml, then prints merge, convert, seat, and import
# against fixture stubs. Does not run axolotl, convert, ollama, or promote.
# AXOLOTL_QLORA_PHASE=prepare stops after the card asserts and the missing-path
# refuses. AXOLOTL_QLORA_PHASE=seat prints the fixture ladder.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
axolotl-qlora-journey:
	bash scripts/axolotl-qlora-journey.sh

# Opt-in print-only chain: prepare-assert, then seat-print, of axolotl-qlora-journey.
# Does not run qlora-journey, seat-journey, or train-next.
# Does not train, merge, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-axolotl:
	bash scripts/uniqueness-axolotl.sh

# Opt-in print-only Unsloth QLoRA journey. Prepares the optional NEXT card
# unsloth-qlora (UNSLOTH.md handoff), then prints merge, convert, seat, and
# import against fixture stubs. Does not call Unsloth, convert, ollama, or promote.
# UNSLOTH_QLORA_PHASE=prepare stops after the handoff asserts and the missing-path
# and wrong-shape refuses. UNSLOTH_QLORA_PHASE=seat prints the fixture ladder.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
unsloth-qlora-journey:
	bash scripts/unsloth-qlora-journey.sh

# Opt-in print-only chain: prepare-assert, then seat-print, of unsloth-qlora-journey.
# Does not run qlora-journey, seat-journey, train-next, or axolotl-qlora-journey.
# Does not train, merge, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-unsloth:
	bash scripts/uniqueness-unsloth.sh

# Opt-in print-only Axolotl LoRA journey. Checks axolotl.yml against
# examples/llama-3/lora-1b.yml, then prints merge, convert, seat, and import
# against fixture stubs. merge-adapt prints axolotl merge-lora without --dequant.
# Does not run axolotl, convert, ollama, or promote.
# AXOLOTL_LORA_PHASE=prepare stops after the card asserts and the missing-path
# refuses. AXOLOTL_LORA_PHASE=seat prints the fixture ladder.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
axolotl-lora-journey:
	bash scripts/axolotl-lora-journey.sh

# Opt-in print-only chain: prepare-assert, then seat-print, of axolotl-lora-journey.
# Does not run axolotl-qlora-journey, uniqueness-axolotl, qlora-journey, seat-journey, or train-next.
# Does not train, merge, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-axolotl-lora:
	bash scripts/uniqueness-axolotl-lora.sh

# Opt-in print-only Unsloth LoRA journey. Prepares the optional NEXT card
# unsloth-lora (UNSLOTH.md handoff, non-quant twin of unsloth-qlora), then
# prints merge, convert, seat, and import against fixture stubs.
# Does not call Unsloth, convert, ollama, or promote.
# UNSLOTH_LORA_PHASE=prepare stops after the handoff asserts and the refuses.
# UNSLOTH_LORA_PHASE=seat prints the fixture ladder.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
unsloth-lora-journey:
	bash scripts/unsloth-lora-journey.sh

# Opt-in print-only chain: prepare-assert, then seat-print, of unsloth-lora-journey.
# Does not run unsloth-qlora-journey, uniqueness-unsloth, or the Axolotl/LF journeys.
# Does not train, merge, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-unsloth-lora:
	bash scripts/uniqueness-unsloth-lora.sh

# Opt-in print-only Apple Silicon mlx-lm LoRA journey (operator section 16).
# Prepares the optional NEXT card mlx-lm-lora (MLX.md handoff), then prints
# fuse, seat, and import against fixture stubs. gguf-convert stays refuse:seat.
# Does not call mlx-lm, fuse, convert, ollama, or promote.
# MLX_LM_LORA_PHASE=prepare stops after the handoff asserts and the refuses.
# MLX_LM_LORA_PHASE=seat prints the fixture ladder.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
mlx-lm-lora-journey:
	bash scripts/mlx-lm-lora-journey.sh

# Opt-in print-only chain of that Apple Silicon mlx-lm LoRA journey (operator section 16).
# prepare-assert, then seat-print, of mlx-lm-lora-journey.
# Does not run the Unsloth, Axolotl, or LLaMA-Factory journeys.
# Does not train, fuse, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-mlx:
	bash scripts/uniqueness-mlx.sh

# Opt-in print-only operator path for purpose-building an SLM on demand (operator section 15).
# Points at existing print journeys (beachhead prepare, qlora, lora, Axolotl, Unsloth, mlx-lm-lora,
# DeepSeek-R1-Distill, GLM-4 Chat),
# the train-next SKIP, merge-adapt, gguf-convert, local-seat, and import-trained.
# After import-trained it prints Standing next (estate): auto_apply=false, no promote,
# and the existing apply-proposal / plan / apply --require-plan / reconcile entrypoints.
# Print only. Does not execute them.
# Does not train, convert, shell out to ollama, promote, or apply the estate.
# Does not invent a live PASS. The recorded Target C PASS stays the only live uniqueness prove.
# CELL_TRAIN_LIVE=1 and CELL_SEAT_LIVE=1 stay print-only.
# Not native MLX.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
purpose-build-checklist:
	bash scripts/purpose-build-checklist.sh

# Opt-in print-only host and stack picker for purpose-build journeys (operator section 17).
# Prints Nvidia / CUDA (LF beachhead, qlora-journey, uniqueness-full primary;
# Unsloth optional; Axolotl integration), Apple Silicon (mlx-lm-lora-journey,
# uniqueness-mlx, optional; refuse:host on stock any-affinity packs), and the
# Target A LoRA twins that already exist. Points at those make targets.
# Does not execute them. Does not train, fuse, convert, shell out to ollama, or promote.
# Does not invent a live PASS. CELL_TRAIN_LIVE=1 and CELL_SEAT_LIVE=1 stay print-only.
# Not native MLX.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
purpose-build-pick:
	bash scripts/purpose-build-pick.sh

# When an SLM fits mid-software-build, or on demand, the same print-only entry.
# Opt-in print-only purpose-build on-demand entry (operator section 18).
# Chains make purpose-build-pick, then make purpose-build-checklist. Those two targets are the parts.
# Calls those targets through make. Does not inline their bodies.
# Does not train, fuse, convert, shell out to ollama, promote, or apply the estate.
# Does not invent a live PASS. The recorded Target C PASS stays the only live uniqueness prove.
# CELL_TRAIN_LIVE=1 and CELL_SEAT_LIVE=1 stay print-only.
# Not native MLX.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
purpose-build-journey:
	bash scripts/purpose-build-journey.sh

# Opt-in print-only DeepSeek-R1-Distill chat QLoRA journey (operator section 19).
# Uses llamafactory-qlora and examples/fixtures/deepseek-r1-distill.pack.json.
# Seat tag llama3. Train base deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B.
# Template deepseekr1. Does not train, merge, convert, seat, or promote.
# DEEPSEEK_R1_DISTILL_PHASE=prepare stops after the card asserts.
# DEEPSEEK_R1_DISTILL_PHASE=seat prints the fixture ladder.
# CELL_TRAIN_LIVE=1 and CELL_SEAT_LIVE=1 stay print-only.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
deepseek-r1-distill-journey:
	DEEPSEEK_CARD=llamafactory-qlora bash scripts/deepseek-r1-distill-journey.sh

# Opt-in print-only chain of that DeepSeek-R1-Distill chat QLoRA journey (operator section 19).
# prepare-assert, then seat-print. Does not run the LoRA twin or the Qwen uniqueness chain.
# Does not train, merge, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-deepseek:
	bash scripts/uniqueness-deepseek.sh

# Opt-in print-only DeepSeek-R1-Distill chat LoRA journey (operator section 19).
# Non-quant twin of the QLoRA card. Uses llamafactory-lora and
# examples/fixtures/deepseek-r1-distill-lora.pack.json. Template deepseekr1.
# Does not run make deepseek-r1-distill-journey.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
deepseek-r1-distill-lora-journey:
	DEEPSEEK_CARD=llamafactory-lora bash scripts/deepseek-r1-distill-journey.sh

# Opt-in print-only chain of that DeepSeek-R1-Distill chat LoRA journey (operator section 19).
# prepare-assert, then seat-print. Does not run make deepseek-r1-distill-journey
# or make uniqueness-deepseek.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-deepseek-lora:
	bash scripts/uniqueness-deepseek-lora.sh

# Opt-in print-only GLM-4 Chat QLoRA journey (operator section 20).
# Uses llamafactory-qlora and examples/fixtures/glm4-chat.pack.json.
# Seat tag llama3. Train base zai-org/glm-4-9b-chat.
# Template glm4. Does not train, merge, convert, seat, or promote.
# GLM4_CHAT_PHASE=prepare stops after the card asserts.
# GLM4_CHAT_PHASE=seat prints the fixture ladder.
# CELL_TRAIN_LIVE=1 and CELL_SEAT_LIVE=1 stay print-only.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
glm4-chat-journey:
	GLM_CARD=llamafactory-qlora bash scripts/glm4-chat-journey.sh

# Opt-in print-only chain of that GLM-4 Chat QLoRA journey (operator section 20).
# prepare-assert, then seat-print. Does not run the LoRA twin or the Qwen uniqueness chain.
# Does not train, merge, convert, seat, or promote.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-glm:
	bash scripts/uniqueness-glm.sh

# Opt-in print-only GLM-4 Chat LoRA journey (operator section 20).
# Non-quant twin of the QLoRA card. Uses llamafactory-lora and
# examples/fixtures/glm4-chat-lora.pack.json. Template glm4.
# Does not run make glm4-chat-journey.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
glm4-chat-lora-journey:
	GLM_CARD=llamafactory-lora bash scripts/glm4-chat-journey.sh

# Opt-in print-only chain of that GLM-4 Chat LoRA journey (operator section 20).
# prepare-assert, then seat-print. Does not run make glm4-chat-journey
# or make uniqueness-glm.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
uniqueness-glm-lora:
	bash scripts/uniqueness-glm-lora.sh

# Opt-in tev1-style classify prepare. Offline JSONL to a one-letter LLaMA-Factory set.
# Does not train. Local only. Do not add to smoke, gate-90, or GitHub Actions.
classify-prepare:
	bash scripts/classify-prepare.sh

# Opt-in tev1-style classify eval. Mock unless CLASSIFY_ENDPOINT is set.
# Does not record a live PASS. Local only. Do not add to smoke, gate-90, or GitHub Actions.
classify-eval:
	bash scripts/classify-eval.sh

# Opt-in tev1 reproduce journey. Default is --print and local llamafactory-cli train.
# TEV1_RUN=1 executes train, merge, GGUF, Ollama, and eval on this host.
# TRAIN_DRIVER=together selects the Together LoRA driver. It is still opt-in.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
# Does not record a live PASS.
tev1-journey:
	bash scripts/tev1-journey.sh

# Opt-in DeepSeek-R1-Distill classify journey. Default is --print and local llamafactory-cli train.
# DEEPSEEK_CLASSIFY_RUN=1 executes train, merge, GGUF, Ollama, and eval on this host.
# Together stays on the tev1 path unless TOGETHER_MODEL is set with TRAIN_DRIVER=together.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
# Does not record a live PASS. READY_FOR_LIVE_TEST stays no.
deepseek-classify-journey:
	bash scripts/deepseek-classify-journey.sh

backup:
	cargo run -q -p estate-control -- backup --estate $(ESTATE) --state-dir $(STATE) --plans-dir plans --out backups

restore:
	cargo run -q -p estate-control -- restore --from $(FROM) --estate $(ESTATE) --state-dir $(STATE) --plans-dir plans --dry-run

pause-proof:
	cargo run -q -p estate-control -- pause-proof --estate $(ESTATE) --state-dir $(STATE) --roots-base .

policy-check:
	cargo run -q -p estate-control -- policy check --policy policy/cell-one.policy.v0.yaml --action apply

test:
	cargo test --workspace

check:
	cargo check --workspace --locked

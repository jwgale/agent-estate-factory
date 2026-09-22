.PHONY: validate plan apply apply-gated apply-dry-run drift models catalog catalog-dump supervisor proxy-check gate gate-60 gate-90 day90 day90-mixed pause-stop pause-start pause-status test check task-mock suspend resume status plans feed-pack feed-import feed-list leases audits history probes feed-cursor floor-suspend floor-resume floor-history operator-day convey packs-list packs-index reconcile packs-propose audit-export expire doctor doctor-strict fixtures-check sessions plan-diff smoke backup restore pause-proof policy-check feed-loop live-specialist real-world enrich-prepare enrich-live-prove train-prepare

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

# Opt-in Unsloth script and Axolotl recipe. Does not run either trainer.
# Local only. Do not add to smoke, gate-90, or GitHub Actions.
train-prepare:
	bash scripts/train-prepare.sh

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

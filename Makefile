.PHONY: validate plan apply apply-gated drift models catalog supervisor proxy-check gate gate-60 gate-90 pause-stop pause-start pause-status test check task-mock suspend resume status plans feed-pack feed-import feed-list

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

drift:
	cargo run -q -p estate-control -- drift --estate $(ESTATE) --state-dir $(STATE)

models:
	cargo run -q -p estate-control -- models --estate $(ESTATE)

catalog:
	cargo run -q -p model-estate -- catalog

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

gate-90:
	./scripts/day90-gate.sh

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

task-mock:
	cargo run -q -p model-estate -- task --estate $(ESTATE) --agent horizon --act model --object xai_grok --mock

test:
	cargo test --workspace

check:
	cargo check --workspace --locked

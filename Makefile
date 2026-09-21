.PHONY: validate plan apply drift models supervisor proxy-check gate pause-stop pause-start pause-status test

ESTATE ?= examples/estate.yaml
STATE ?= .cell

validate:
	cargo run -q -p estate-control -- validate --estate $(ESTATE)

plan:
	cargo run -q -p estate-control -- plan --estate $(ESTATE) --plans-dir plans

apply:
	cargo run -q -p estate-control -- apply --estate $(ESTATE) --state-dir $(STATE) --roots-base .

drift:
	cargo run -q -p estate-control -- drift --estate $(ESTATE) --state-dir $(STATE)

models:
	cargo run -q -p estate-control -- models --estate $(ESTATE)

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

test:
	cargo test --workspace

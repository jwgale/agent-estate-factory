# Valid Security-as-IaC fixtures

- [`../estate.yaml`](../estate.yaml) is the Cell One desired-state (host_class `any`).
- [`../hosts/`](../hosts/) is the portable host matrix (aliases normalize to locked names).
- [`covering-plan.json`](covering-plan.json) is a reviewable greenfield plan shape (`cell-one.plan.v0`).

Invalid counterparts live under [`../invalid/`](../invalid/): `host-class-bad.yaml`, `stale-plan.json`.

# Pause kit

Cell One is pause-safe when desired-state lives on disk and runtime is disposable.

## Survives stop

| Artifact | Path |
| --- | --- |
| Charter | `charter.md` |
| Estate file | `examples/estate.yaml` |
| Schema | `schema/estate.v0.schema.json` |
| Lane roots | `lanes/{horizon,research,sanctum}/` |
| Plans | `plans/` (append-only) |
| Gate reports | `gate-reports/` |
| Operator lifecycle | `.cell/lifecycle.json` (durable, not estate SoT) |

## Disposable

| Artifact | Path |
| --- | --- |
| PIDs / heartbeats | `.cell/runtime/` |
| Warm desktops / session dirs | `.cell/sessions/` |
| Regenerable actual-state | `.cell/actual-state.json` |

`actual-state.json` is *not* SoT. After a pause you re-apply from the estate file. Lane files you wrote stay put.

## Commands

```bash
make pause-stop    # estate suspend: drop sessions/PIDs; keep lifecycle.json
make pause-start   # estate resume from the estate file
make status
```

Stop does not delete `lanes/` or `lifecycle.json`. Start does not call frontier or local models. Cloud-agent placements are not spawned.

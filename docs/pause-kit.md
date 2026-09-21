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

## Disposable

| Artifact | Path |
| --- | --- |
| PIDs / heartbeats | `.cell/runtime/` |
| Warm desktops / session dirs | `.cell/sessions/` |
| Regenerable actual-state | `.cell/actual-state.json` |

`actual-state.json` is *not* SoT. After a pause you re-apply from the estate file. Lane files you wrote stay put.

## Commands

```bash
make pause-stop    # drop runtime + session dirs; leave estate/lanes/plans
make pause-start   # validate + apply from files
./scripts/pause-kit.sh status
```

Stop does not delete `lanes/`. Start does not call frontier or local models.

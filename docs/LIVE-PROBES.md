# Live probes (hand-off page)

Real-world testing is a recorded live proof on Jason's boxes plus a green
local gate (`make gate-90`). This page is the paste target. An AI gateway
stays out of altitude. Words: [`UBIQUITOUS_LANGUAGE.md`](UBIQUITOUS_LANGUAGE.md).
Product page: [`NORTH-STAR.md`](NORTH-STAR.md).

Copy-paste for Jason at a Mac or a Linux/5090-class box. CI never sets
these variables. Without an endpoint, `estate probes --live` prints SKIP
and exits 0. `make real-world` does the same for its live steps and does
not invent PASS. That command is not in smoke, `make gate-90`, or Actions.

Parked status: [`DAY90-PLUS.md`](DAY90-PLUS.md). Dry shapes (no network):
`schema/live-probe-shapes.v0.json`.

Do not put `5090`, `4090`, or `m3-max` in estate binding ids or probe ids.
A 5090 is one `consumer-nvidia` or `rented-nvidia` host.

`estate specialist` is a thin data-plane delegate (`HttpLocal`). It is
not a gateway. Default job is `complete`: Jason gets real model text
back. Sacred tokens refuse before any HTTP POST. SKU endpoint / model
ids / prompt / completion refuse. Missing endpoint refuses. Native MLX
/ vLLM / TRT stay stub or experimental.

## Recorded live proof (Jason boxes)

These ran on Jason's boxes. They are **not** native MLX. They are **not**
required for `make smoke` / hosted CI. Do not put `5090` in a binding id.

| Box | Command | Result |
| --- | --- | --- |
| Mac (Apple Silicon) | `estate probes --live` against Ollama-on-Mac | **PASS.** `live ok` on the Ollama HTTP adapter. Native `mlx` `specialist()` stays Stub. |
| Mac (Apple Silicon, MacBook Air) | `estate specialist --driver ollama --prompt "Reply with the single word pong."` | **PASS.** `"completion": "Pong"`, `"reason": "compat completion"`. Tip `2ab78a4`. |
| Linux 5090-class (`consumer-nvidia` / `rented-nvidia`) | `estate probes --live` | **PASS.** `live ok (openai /v1/models)`. That GET is not a chat proof. |
| Linux 5090-class | `estate specialist --driver ollama --prompt "Reply with the single word pong."` | **PASS.** `"completion": "Pong"`. Empty OpenAI `message.content` fell through to `/api/chat`. |
| Env-gated key (no box name) | `estate specialist --driver frontier` model `grok-4.7` | **PASS.** `"completion": "pong"`, `"reason": "frontier completion"`. Key never printed. Ran with env-gated `XAI_API_KEY`. |

Not recorded: native MLX. Frontier `grok-4.7` specialist and the Mac
specialist complete are recorded above. The Mac `Pong` is that Mac run.
It is not copied from the 5090 row. Target C live uniqueness is a
separate recorded PASS in the next section. It is not a row in the
table above.

`READY_FOR_LIVE_TEST` for the rows above: **no**. Those runs are recorded.

## Target C live uniqueness (5090-class)

Recorded on 2026-09-23 on a Linux 5090-class host (`consumer-nvidia` /
`rented-nvidia`). Do not put `5090` in a binding id. This PASS is the
outside-factory ladder. It is not `estate probes --live`. It is not the
Mac `Pong` row. It is not native MLX.

**PASS.** Prepare → train/export outside the factory → tokenizer restore
with dereference → `gguf-convert` → `local-seat` print → write the
Modelfile from the printed contents → `ollama create` outside the factory
→ `import-trained` → cleanup.

The workdir was the throwaway `/tmp/cell-one-target-c-live-20260923`.
Prepare used that workdir. It did not use `examples/estate.yaml` as the
prepare target. `examples/estate.yaml` stayed unchanged. Its cksum stayed
`43770130 3391`.

1. Prepare wrote the LLaMA-Factory QLoRA card (`llamafactory-qlora`). Seat tag `llama3`. Train base `Qwen/Qwen2.5-0.5B-Instruct`. The short train on that card was `max_steps` 10.
2. LLaMA-Factory train and export ran outside the factory and exited 0. The merged Hugging Face export is under that workdir's `llamafactory-qlora` tree.
3. `estate enrich gguf-convert` returned `refuse:tokenizer` first (`extra_special_tokens` was a list; the Qwen export was missing `vocab.json` and `merges.txt`). The operator restored tokenizer files from the HF cache snapshot for that train base into the export directory, with dereference (`cp -aL` or `cp --dereference`). A plain `cp -a` left symlinks. Enrich does not follow a symlinked `tokenizer_config.json`. The convert then wrote a BF16 GGUF (~949M). That convert ran outside the factory.
4. `estate enrich local-seat` printed the Modelfile and did not write `$PREPARED/Modelfile`. The operator wrote that file from the printed contents.
5. The operator ran `ollama create` outside the factory. The prove tag was `cell-target-c-qlora-prove`.
6. `estate enrich import-trained` recorded `trained_shape` `gguf` with `auto_apply=false`.
7. The operator cleaned up with `ollama rm` on that tag.

The factory did not train, convert, shell out to ollama, or promote.
`READY_FOR_LIVE_TEST`: **no**. This prove is not in `make smoke`,
`make gate-90`, or GitHub Actions.

After create, the seated model answered a short specialist/pong-style check.
That check is not the recorded `estate probes --live` row and not
the Mac `Pong` row. This page does not record a completion JSON blob for
it.

The tokenizer restore, the unwritten Modelfile, and the GGUF write line
are the later tip locks in PR #149, PR #150, and PR #151. Tip framing
stays through PR #151. This row does not move that SHA.

Walk: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md). Seat: [`local-seat.md`](local-seat.md).

## Mac specialist result (recorded)

Ran on Jason's MacBook Air against tip `2ab78a4`. Same command as the
5090 row. Native MLX stays stub.

| Box | Command | Result |
| --- | --- | --- |
| Mac (Apple Silicon) | `estate specialist --driver ollama --prompt "Reply with the single word pong."` | **PASS.** `"completion": "Pong"`. |

```json
{
  "allow": true,
  "redacted_text": "Reply with the single word pong.",
  "reason": "compat completion",
  "job": "complete",
  "completion": "Pong"
}
```

`READY_FOR_LIVE_TEST`: **no**. That Mac command is recorded. The 5090
block below is the same verb; that `Pong` was already recorded.

## Mac specialist complete (recorded)

Recorded on the MacBook Air against tip `2ab78a4`. Ollama-on-Mac
`probes --live` already PASSed. Native MLX stays stub. `mlx`, `vllm`,
and `trt` stay `not live-ok`. This command does not ping those cards.

`READY_FOR_LIVE_TEST`: **no**.

### MacBook Air

Already recorded: `"allow": true`, `"job": "complete"`, `"reason": "compat completion"`, `"completion": "Pong"`.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
export CELL_LOCAL_MODEL=llama3
estate specialist --driver ollama --prompt "Reply with the single word pong."
```

From a checkout when `estate` is not on `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
export CELL_LOCAL_MODEL=llama3
cargo run -q -p estate-control -- specialist --driver ollama \
  --prompt "Reply with the single word pong."
```

Expect exit 0, `"allow": true`, `"job": "complete"`, and a non-empty
`"completion"` (often `pong` / `Pong`). Empty `completion` is refuse.
If the model is missing: `ollama pull llama3`.

### Linux 5090-class (box is open)

Already recorded: `"completion": "Pong"`. Same env. Do not put `5090`
in a binding id. Host class is `consumer-nvidia` or `rented-nvidia`.
`READY_FOR_LIVE_TEST` for this recorded row: **no**. Run it to confirm
the open box still answers.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
export CELL_LOCAL_MODEL=llama3
estate specialist --driver ollama --prompt "Reply with the single word pong."
```

From a checkout when `estate` is not on `PATH`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
export CELL_LOCAL_MODEL=llama3
cargo run -q -p estate-control -- specialist --driver ollama \
  --prompt "Reply with the single word pong."
```

Opt-in helper (requires `CELL_LOCAL_ENDPOINT`; refuse if unset; not in smoke):

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
export CELL_LOCAL_MODEL=llama3
make live-specialist
```

## Opt-in enrich handoff (seated runtime)

`make enrich-live-prove` is a separate opt-in. It is not `estate probes --live` and it is not `estate specialist`. It prepares a throwaway cell, runs `ollama create cell-enrich-<pack> -f Modelfile` when Ollama is up, runs `estate enrich import-prepared`, checks the tag with `ollama show`, and deletes that tag. `examples/estate.yaml` stays untouched.

`FROM` in that Modelfile is the seated model (`CELL_LOCAL_MODEL`, or `llama3` when `ollama list` has it, otherwise the first listed tag with `:latest` stripped). It is never the binding id `local_slm`. The example estate leaves `params.model` unset, so the script sets that field on a copy under `/tmp/cell-one-enrich-live-prove`. The output path must not contain a hardware SKU.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CELL_LOCAL_MODEL=llama3
make enrich-live-prove
```

Seat down, or `ollama` missing: the script prints `SKIP` and exits 0. That line is not a pass. A hardware SKU in the model name refuses.

`READY_FOR_LIVE_TEST` stays no. This command is an opt-in seated-runtime enrich handoff only. It is not a factory-wide live test. It is not in `make smoke`, `make gate-90`, or GitHub Actions. Command page: [`TRAIN-ENRICH.md`](TRAIN-ENRICH.md).

## What a live probe actually pings

Not the Ollama chat UI. Not native MLX. Not `POST /v0/specialist`.

```
GET {endpoint}/v1/models          # OpenAI-compatible (Ollama, llama.cpp, vLLM)
GET {endpoint}/api/tags           # Ollama native, if /v1/models is missing
```

Timeout is 800ms. A 2xx JSON **models list** is `live_probed=true` /
`live ok (openai /v1/models)` or `live ok (ollama /api/tags)`. An empty
list is up (server running, no weights pulled). Garbage, empty body, or
connect fail is `live_probed=false` / `down: ...`. Unset endpoint is
SKIP. The command still exits 0 in all three cases. It does not invent
success.

Native `mlx` `specialist()` stays **Stub**. Mac live proof is
**Ollama-on-Mac** (same `ollama` card). mlx still reads
`CELL_MLX_ENDPOINT`, then `CELL_LOCAL_ENDPOINT`, but that HTTP ping is
not a live proof. `mlx`, `vllm`, and `trt` stay `live_probed=false` and
print `not live-ok`. They do not print `live ok`.

## Env

| Variable | Mac | Linux / 5090-class |
| --- | --- | --- |
| `CELL_LIVE_PROBE` | `1` / `true` / `yes` if you omit `--live` | same |
| `CELL_LOCAL_ENDPOINT` | Ollama-on-Mac Supported path (`http://127.0.0.1:11434`) | Ollama / llama.cpp / http-remote |
| `CELL_RENTED_ENDPOINT` | unused | alias for a rented (or any) box. Not a SKU. |
| `CELL_MLX_ENDPOINT` | OpenAI-compatible MLX *server* only; native MLX stays stub | unused unless you set it |
| `CELL_VLLM_ENDPOINT` | experimental; unset = SKIP | experimental `/v1/models` |
| `CELL_TRT_ENDPOINT` | experimental; unset = SKIP | experimental; unset = SKIP |
| `CELL_LOCAL_MODEL` | optional model id for `specialist()`; SKU ids refuse. Unset = first `/api/tags` id, then `/v1/models`. | same |
| `CELL_FRONTIER_ENDPOINT` | unused by probes and Ollama specialist | optional base for `--driver frontier`. Default `https://api.x.ai/v1`. |

`XAI_API_KEY` is required for `--driver frontier` (model `grok-4.7`). Probes do not use it. Local specialist does not use it.

## Sample output (exact lines)

Driver column is 12 chars. Status column is 12 chars. Notes wrap on the
next indented line.

### SKIP (no endpoint env; this cloud box / CI)

```
live probe (SKIP without endpoints; not used in CI)
  ollama       status=supported    bindable=true live_probed=false host_class=any
    Ollama-first. Catalog-level probe; not a live ping. SKIP (no endpoint env; CI never requires a live box)
  llama.cpp    status=swap-proof   bindable=true live_probed=false host_class=any
    llama.cpp swap-proof sibling. Same specialist protocol. SKIP (no endpoint env; CI never requires a live box)
  mlx          status=stub         bindable=false live_probed=false host_class=apple-silicon
    Native MLX specialist() is stub. Live Mac proof is Ollama-on-Mac (or OpenAI-compatible) via the HTTP adapter. SKIP (no endpoint env; CI never requires a live box)
  vllm         status=experimental bindable=false live_probed=false host_class=any
    Experimental until Jason verifies. Fail closed; no frontier fallback. SKIP (no endpoint env; CI never requires a live box)
  trt          status=experimental bindable=false live_probed=false host_class=any
    Experimental until Jason verifies. Fail closed; no frontier fallback. SKIP (no endpoint env; CI never requires a live box)
  http-remote  status=supported    bindable=true live_probed=false host_class=any
    CELL_LOCAL_ENDPOINT remote pattern. Catalog-level probe. SKIP (no endpoint env; CI never requires a live box)
```

Exit 0. No `5090` / `4090` / `m3-max`.

### live ok (OpenAI-compatible `/v1/models`, including empty list)

Ollama, llama.cpp, and http-remote share `CELL_LOCAL_ENDPOINT` and may
print `live ok`. mlx falls back to that env and still prints
`not live-ok`. vLLM / TRT do the same when their own env is set. Unset
stays SKIP.

```
live probe (SKIP without endpoints; not used in CI)
  ollama       status=supported    bindable=true live_probed=true host_class=any
    Ollama-first. Catalog-level probe; not a live ping. live ok (openai /v1/models)
  llama.cpp    status=swap-proof   bindable=true live_probed=true host_class=any
    llama.cpp swap-proof sibling. Same specialist protocol. live ok (openai /v1/models)
  mlx          status=stub         bindable=false live_probed=false host_class=apple-silicon
    Native MLX specialist() is stub. Live Mac proof is Ollama-on-Mac (or OpenAI-compatible) via the HTTP adapter. not live-ok (stub or experimental card; HTTP ping is not a live proof)
  vllm         status=experimental bindable=false live_probed=false host_class=any
    Experimental until Jason verifies. Fail closed; no frontier fallback. SKIP (no endpoint env; CI never requires a live box)
  trt          status=experimental bindable=false live_probed=false host_class=any
    Experimental until Jason verifies. Fail closed; no frontier fallback. SKIP (no endpoint env; CI never requires a live box)
  http-remote  status=supported    bindable=true live_probed=true host_class=any
    CELL_LOCAL_ENDPOINT remote pattern. Catalog-level probe. live ok (openai /v1/models)
```

If the server has no `/v1/models` and answers Ollama `GET /api/tags`,
the note is `live ok (ollama /api/tags)` instead.

### down (endpoint set, nothing listening)

`down:` is followed by the ureq connect error (wording varies by OS).

```
live probe (SKIP without endpoints; not used in CI)
  ollama       status=supported    bindable=true live_probed=false host_class=any
    Ollama-first. Catalog-level probe; not a live ping. down: http://127.0.0.1:1/v1/models: Connection Failed: Connect error: Connection refused (os error 111)
```

llama.cpp and http-remote look the same. mlx, vLLM, and TRT stay
`not live-ok` when an endpoint is set. They do not open that ping.
Unset stays SKIP. Exit 0.

## Dry on this box (no GPU, no Jason)

```bash
cargo run -q -p estate-control -- probes --live
# expect: the SKIP sample above, exit 0

cargo test -q -p model-estate --lib
cargo test -q -p model-estate --test specialist_cli
cargo test -q -p estate-control --test day90_heal probes_live_skip
```

## Specialist complete (new live surface)

Default job is `complete`. Jason gets a non-empty `completion` field
from the model. Control does not become a gateway; this is the same
`HttpLocal` adapter as `model-estate specialist --job complete`.

### Mock HTTP (this box / CI)

Factory protocol (`mock-local` / `/v0/specialist`):

```bash
# Terminal 1
cargo run -q -p model-estate -- mock-local --bind 127.0.0.1:47831
```

```bash
# Terminal 2
cargo run -q -p estate-control -- specialist --endpoint http://127.0.0.1:47831 \
  --driver ollama --prompt "hello from the factory"
```

Expect exit 0:

```
{
  "allow": true,
  "redacted_text": "hello from the factory",
  "reason": "complete allow",
  "job": "complete",
  "completion": "mock:hello from the factory"
}
```

OpenAI-compatible mock (`CompatServer`) returns `"completion": "ok"`.
Same adapter, `--driver llama.cpp`, posts `/v1/chat/completions`.

Policy-precheck (no model text) still works:

```bash
cargo run -q -p model-estate -- specialist --endpoint http://127.0.0.1:47831 \
  --job policy-precheck --text "hello from the factory"
```

```
{
  "allow": true,
  "redacted_text": "hello from the factory",
  "reason": "policy-precheck allow",
  "job": "policy-precheck"
}
```

Sacred refuse (exit 1, no HTTP POST):

```bash
cargo run -q -p estate-control -- specialist --endpoint http://127.0.0.1:47831 \
  --prompt "please mention cyera"
```

```
{
  "allow": false,
  "redacted_text": "",
  "reason": "policy-precheck denied sacred token 'cyera'",
  "job": "complete"
}
specialist denied
```

Missing `--endpoint` and unset `CELL_LOCAL_ENDPOINT` refuses (exit 1).
SKU endpoint / `CELL_LOCAL_MODEL` / listed model id / prompt /
completion refuses. No silent allow.

### Jason Mac (Ollama already PASSed probes --live)

This is the one new command. Same `CELL_LOCAL_ENDPOINT` as the #23 probe.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
export CELL_LOCAL_MODEL=llama3
cargo run -q -p estate-control -- specialist --driver ollama \
  --prompt "Reply with the single word pong."
```

Equivalent:

```bash
cargo run -q -p model-estate -- specialist --job complete \
  --prompt "Reply with the single word pong."
```

Expect exit 0, `"allow": true`, `"job": "complete"`, and a **non-empty
`completion`**. Wording varies by model (often `pong` / `Pong.`). It
must not be the prompt echoed as policy `redacted_text` only. Empty
`completion` is refuse.

llama.cpp server (same OpenAI adapter):

```bash
cargo run -q -p estate-control -- specialist --driver llama.cpp \
  --endpoint http://127.0.0.1:8080 --prompt "Reply with the single word pong."
```

Mac `estate specialist` chat is the command in "Mac specialist complete (recorded)".
That MacBook Air run is **PASS** (`"completion": "Pong"`, reason `compat completion`).
`READY_FOR_LIVE_TEST`: **no** for that Mac command. The 5090 `Pong` row stays recorded.

## Frontier (grok-4.7)

Separate from Ollama / `CELL_LOCAL_ENDPOINT`. `--driver frontier`
(aliases `frontier-http`, `ai-gateway`, `openai-compat`) posts
`/v1/chat/completions` to xAI. Model id is **`grok-4.7`** unless
`CELL_FRONTIER_MODEL` or `XAI_MODEL` is set. SKU model ids and SKU
endpoints refuse before POST. Sacred text refuses before POST.

`--driver http-remote` stays the **local** remote card
(`CELL_LOCAL_ENDPOINT`). It is not this path. Local down still does not
fall through to frontier.

Cloud agents use `reasoning_effort` **xhigh** as Jason's standing
default. The factory specialist sends a normal chat completion for
`grok-4.7` and does not require that field.

Unset `XAI_API_KEY` refuses (no invented completion). CI never sets the
key. A fake key plus `--endpoint` against in-process mock returns
`"completion": "ok"` and the POST body contains `"model":"grok-4.7"`.

```bash
export XAI_API_KEY=...
# optional: export CELL_FRONTIER_ENDPOINT=https://api.x.ai/v1
# optional: export CELL_FRONTIER_MODEL=grok-4.7
cargo run -q -p estate-control -- specialist --driver frontier \
  --prompt "Reply with the single word pong."
```

Expect exit 0, `"allow": true`, `"job": "complete"`, `"reason": "frontier completion"`,
and a non-empty `"completion"`. The key must not appear in stdout.

This command **PASSed** (`completion` `pong`, reason `frontier completion`,
model `grok-4.7`). The key was not printed. `reasoning_effort` xhigh stays
the cloud-agent standing default and was not part of this factory POST.

`READY_FOR_LIVE_TEST`: **no**. That surface is recorded.

`probes --live` can print `live ok (openai /v1/models)` while
`/v1/chat/completions` returns empty `message.content`. Specialist now
falls through to Ollama `/api/chat` instead of dying on that 200.

## Jason Linux / 5090 retry (empty OpenAI content)

Same command as Mac. This is the retry after the 5090 box hit
`driver unreachable: openai chat: empty message.content`.

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
export CELL_LOCAL_MODEL=llama3
cargo run -q -p estate-control -- specialist --driver ollama \
  --prompt "Reply with the single word pong."
```

Expect exit 0, `"allow": true`, `"job": "complete"`, and a **non-empty
`completion`**. Sample:

```
{
  "allow": true,
  "redacted_text": "Reply with the single word pong.",
  "reason": "compat completion",
  "job": "complete",
  "completion": "pong"
}
```

If both OpenAI and `/api/chat` are empty, the refuse names HTTP status,
the model id used, and `ollama pull llama3` / `CELL_LOCAL_MODEL`.

This 5090 retry **PASSed** (`completion` `Pong`). `READY_FOR_LIVE_TEST`
for that row: **no**.

## Jason Mac (Apple Silicon) - Ollama-on-Mac

Supported path. Native MLX stays stub. Same commands as #23.

```bash
# Terminal 1 - if ollama is not already a service
ollama serve
```

```bash
# Terminal 2
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
cargo run -q -p estate-control -- probes --live
```

Expect ollama / llama.cpp / http-remote: `live_probed=true` and
`live ok (openai /v1/models)` or `live ok (ollama /api/tags)`.
mlx stays `live_probed=false` and `not live-ok`. That fallback is not
native MLX.

Optional OpenAI-compatible MLX *server* (still not native MLX):

```bash
export CELL_MLX_ENDPOINT=http://127.0.0.1:8080
cargo run -q -p estate-control -- probes --live
```

Expect mlx: `status=stub bindable=false live_probed=false` and
`not live-ok`. `specialist()` on the mlx card still refuses Stub.

## Jason Linux / 5090-class (consumer-nvidia or rented-nvidia)

Never encode `5090` in a binding id.

```bash
# Terminal 1 - if ollama is not already a service
ollama serve
```

```bash
# Terminal 2 - desktop / laptop RTX = consumer-nvidia
# rented / lab GPU (including a 5090 host) = rented-nvidia
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
# or: export CELL_RENTED_ENDPOINT=http://127.0.0.1:11434
cargo run -q -p estate-control -- probes --live
```

Expect ollama / llama.cpp / http-remote `live_probed=true`. vLLM and
TRT stay SKIP when their env is unset, and `not live-ok` when it is set.

Optional vLLM (experimental card; not live-ok):

```bash
export CELL_VLLM_ENDPOINT=http://127.0.0.1:8000
cargo run -q -p estate-control -- probes --live
```

Optional llama.cpp server (OpenAI `/v1/models` + `/v1/chat/completions`):

```bash
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:8080
cargo run -q -p estate-control -- probes --live
# expect live ok (openai /v1/models)
```

## Down vs SKIP

Stop Ollama and re-run `probes --live` with the same env: expect
`live_probed=false` and `down:`. Unset the env: expect SKIP. Both exit 0.

Optional A8 after a model is pulled (`ollama pull llama3` or set
`CELL_LOCAL_MODEL`):

```bash
export CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434
cargo run -p model-estate -- task --estate examples/estate.yaml \
  --agent research --act tool --object notes-append --payload "append a note"
```

Down local is `local:down` / `model.local.down`. No silent `grok-4.7` fallback.

## What green is not

| You saw | What it is not |
| --- | --- |
| SKIP in CI or on this cloud box | A failed live test |
| `live_probed=true` from `mock-local` | MLX Supported, or weights green |
| mlx note `not live-ok` | Native MLX, or a catalog card flipped off stub |
| Empty models list `live ok` | A pulled model; only the HTTP server is up |
| `estate specialist` `completion` on mock-local (`mock:...`) | A live Ollama chat Jason ran |
| `estate specialist` `completion` against Ollama | Native MLX |
| 5090 `completion` `Pong` | Native MLX. The Mac `Pong` is a separate recorded row. |
| Mac `completion` `Pong` | Native MLX |
| Target C live uniqueness **PASS** | The factory training, converting, shelling out to ollama, or promoting. Not `estate probes --live`. Not the Mac `Pong` row. Not native MLX. Not `make smoke`, `make gate-90`, or Actions. |

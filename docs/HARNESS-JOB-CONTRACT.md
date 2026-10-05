# Harness job contract

Grok Bot drops one small JSON file. The harness on the 5090 runs that one job when `cell-job run` is invoked on the box. There is no second chat, no scheduler, no notification service, and no polling agent.

## Job file

Path: `~/harness-inbox/jobs/<id>.json`

Fields, and nothing else:

- `id`
- `step`: `gap`, `decontaminate`, `train`, `export`, `eval`, `quant`, `ask`, `pack-device`, `run`, `improve`, or `improve-apply`
- `config`: a path that already exists. Never a pasted corpus.
- `gpu`: `forbid` or `ask`
- `max_minutes`
- `nas_out`

A train, eval, or run step with `gpu: forbid` is a refuse, not a silent CPU fallback. `improve` and `improve-apply` are CPU steps. A forbid on those is allowed. A missing config is a refuse. Any other step name is a refuse.

Optional wrapper, which only writes the file and prints the id:

```
ssh 5090 cell-job submit
```

JSON on stdin. `scripts/cell-job.sh submit` is that wrapper.

## Status and card

Status: `~/harness-inbox/status/<id>.json` and `.cell/harness-job-<id>/status.json`. Under 2 KB. Fields: `id`, `state`, `step`, `pid_alive`, `gpu`, `reason`, `updated`, `reading_page`, `card`.

States: `queued`, `needs-approval`, `running`, `succeeded`, `failed`, `refused`.

The card is beside the status file and appended to `~/harness-inbox/outbox.jsonl` when the state changes. Under 1 KB. It has the one-sentence reason, pass / fail / refused, `needs_you`, the reading-page path, artifact names and sizes, and GPU minutes for that id. No file bodies.

## Approve and GPU

If `gpu` is `ask`, the job writes `needs-approval` and a card before it starts. The card says whether the GPU is idle or busy, the holder name if the guard can see one, and the minute cap. A missing approve file is not a yes. A `no` refuses the job and keeps the file. An approve file does not override a busy card (`refuse:gpu-busy`) or a minute cap (`refuse:gpu-cap`). This build does not start llamafactory-cli. The plan file names the existing `scripts/train-step.sh` or `scripts/export-step.sh` line. A person runs that script. This runner will not.

## What a cloud session may read

The status file, the card, and the last 30 error lines of that job's log. A multi-megabyte manifest, a cleaned page, or a batch report is a refuse. A cloud session does not run a job.

## Quota

`cell-job digest` rewrites `~/harness-inbox/quota.json` from local status files. It does not SSH and it does not open manifests. Fields: GPU minutes today, jobs running, `cloud_helper_turns_today`. A status check with `--via cloud` increments that last number. If it is above zero, the digest writes a pause note and new jobs wait until `cell-job clear-quota`.

## Other commands

- `prepare preference` and `prepare refusal` write cards. `auto_train` is false. They do not train.
- `base-pick` compares cached files only when a stand-in scorer is injected by a test. The CLI does not load weights and does not download.
- `pack-device` copies a GGUF that is already on disk into a NAS folder after an approve file. It does not SSH, does not run `ollama create`, and does not mark the registry ready.
- `improve` reads a key-fact score card and saved replies, groups misses, and writes a proposal under `nas_out/improve/`. It does not edit the training set. It does not need an approve file. Cloud drafting is off unless the config sets `allow_cloud`.
- `improve-apply` needs an approve file with decision yes. It appends the approved rows into a new dataset version folder and writes a receipt. It does not edit the original training file. A missing file is not a yes.
- `run` does gap, then decontaminate, then train, then eval. If the config sets `improve` true, it writes the proposal after eval. That flag defaults off. Apply is not part of the run.

The registry is `~/harness-inbox/registry.jsonl`. Every line has `ready: no`.

Item 15 (continued-pretrain prepare card) is not in this build.

`READY_FOR_LIVE_TEST: no`. Estate cksum stays `43770130 3391`. Not in `make smoke`, `make gate-90`, or GitHub Actions.

Exit codes for this package start at 30. 0 is ok or a wait. 30 job refuse. 31 gpu-busy. 32 gpu-cap. 34 gap. 35 decontaminate. 36 local-first. 37 judge. 38 rewrite. 39 launch not started. 41 route. 42 mix. 43 base-pick. 44 device pack. 45 improve.

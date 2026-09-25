# Changelog

Local wrap: `make smoke`. Hosted CI is compile-only (`cargo check --workspace --locked` on pull_request). Day 0–90 is on `main`.

## This slice — north star is the harness and custom AI creator suite

- The product is a next-generation harness and custom AI creator suite. Pillars, in order: agents under security-as-IaC, a Grok Bot–like harness (the UI may still be deferred), and on-spot specialty SLMs. A central learning brain stays parked. Train/enrich stays one facet. Overnight packing stays SLM-heavy. [`docs/NORTH-STAR.md`](docs/NORTH-STAR.md), the README opening, and `estate help north-star` use that sentence. Anti-shrink still refuses a thin Grok Bot clone without the estate, a UI-only shell, and undirected agent sprawl. This slice does not rebalance overnight work. It does not invent a live PASS. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — TRAIN-ENRICH names the purpose-build mid-software-build entry

- `docs/TRAIN-ENRICH.md` names `make purpose-build-journey` for mid-software-build. When an SLM fits mid-software-build, or on demand, that target is the print-only purpose-build on-demand entry. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. Walk: [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md) section 18. The journey runs the pick (section 17), then the checklist (section 15). DeepSeek-R1-Distill (section 19) and GLM-4 Chat (section 20) are sibling print-only journeys reachable from the pick and the checklist. They are not steps of this journey. The What exists today table names `make purpose-build-checklist` (operator section 15), `make purpose-build-pick` (operator section 17), `make purpose-build-journey` (operator section 18), `make deepseek-r1-distill-journey` and `make uniqueness-deepseek` plus the LoRA twins (operator section 19), and `make glm4-chat-journey` and `make uniqueness-glm` plus the LoRA twins (operator section 20). Those rows are print-only. They do not train, convert, seat, promote, or apply. They do not invent a live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). The re-prove card stays `make uniqueness-prove-checklist`. This slice does not move the GATE-90, Cell One, or live-probes tip header (that stays through PR #193, `86b1ad5d344006e7489b24fe14fef7b8b16215f4`). It does not wire those targets into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a train family, a fixture pack, a driver, or a make target. It does not add Kimi. It does not add DeepSeek or GLM journey code. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — tev1-style classify prepare and held-out eval

- `estate classify prepare` reads tev1-style JSONL (`state`, `question`, options lettered from A, answer letter) and writes a one-letter LLaMA-Factory dataset (`sharegpt` or `alpaca`, plus `dataset_info.json`) and a held-out JSONL. The split is a seeded shuffle of question groups (`group_id` when set, otherwise the question text), so variants of one question stay together. A non-empty `--out` is refused unless `--force`. `--strict` writes nothing when a row is bad. Otherwise bad rows are skipped, and the message names the count and the first line numbers. `estate classify eval` scores that held-out file on an OpenAI-compatible chat endpoint (Ollama `/v1` or a hosted endpoint), temperature 0, `max_tokens` 8, thinking off. It reports accuracy, per-label confusion, invalid outputs, and latency p50/p95. `--dry-run` and `--mock` do not use the network. `--api-key-env` names a variable and never prints the secret. The fixture is `examples/fixtures/tev1-decisions.jsonl` (hand-written; not copied public data). Credit [togethercomputer/tev1](https://github.com/togethercomputer/tev1) for the record shape. Together hosted fine-tune is an optional hosted driver only. `make classify-prepare` and `make classify-eval` are opt-in. No live classify run has been done. This slice does not invent a live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not move the GATE-90 or Cell One tip header (that stays through PR #193, `86b1ad5d344006e7489b24fe14fef7b8b16215f4`). It does not wire these targets into `make smoke`, `make gate-90`, or GitHub Actions. It does not add Kimi. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #193

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #193 (`86b1ad5d344006e7489b24fe14fef7b8b16215f4`). That tip names tip honesty through PR #191 (PR #192, `d260cb7145df00bec613bb0b34c52272cb97cd49`) and help that names mid-software-build purpose-build (PR #193). `estate help enrich`, `estate help train`, and the index Journey line name `make purpose-build-journey` for mid-software-build. When an SLM fits mid-software-build, or on demand, that target is the same print-only purpose-build on-demand entry (operator section 18). It runs `make purpose-build-pick`, then `make purpose-build-checklist`. It does not inline those bodies. The PR #191 tip stays `0e4223ea28a73c3931c7545f3040fd14ea8ae273`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add DeepSeek or GLM journey code. It does not add a runtime write path or an ollama shell-out. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — help names mid-software-build purpose-build

- `estate help enrich` and `estate help train` name `make purpose-build-journey` for mid-software-build. When an SLM fits mid-software-build, or on demand, that target is the same print-only purpose-build on-demand entry (operator section 18). The index Journey line names that same path. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. It does not inline those bodies. It does not train, fuse, convert, shell out to ollama, promote, or apply the estate. It does not invent a live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). The re-prove card stays `make uniqueness-prove-checklist`. This slice does not move the GATE-90 or Cell One tip header (that stays through PR #191, `0e4223ea28a73c3931c7545f3040fd14ea8ae273`). It does not wire `make purpose-build-journey` into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add DeepSeek or GLM journey code. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only.

## This slice — GATE-90 and Cell One tip honesty through PR #191

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #191 (`0e4223ea28a73c3931c7545f3040fd14ea8ae273`). That tip names tip honesty through PR #189 (PR #190, `ccf23829c8a62c070f98eff558d2f8fe2c1ccaaf`) and operator surfaces naming mid-software-build purpose-build (PR #191). When an SLM fits mid-software-build, or on demand, the same print-only entry is `make purpose-build-journey`. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. `docs/OPERATOR-DAY.md`, `docs/NORTH-STAR.md`, `docs/operator-enrich-journeys.md`, the print banners, and the README name that entry. The PR #189 tip stays `a857f05a256b214a2069de42e6918130eb080d66`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add DeepSeek or GLM journey code. It does not add a runtime write path or an ollama shell-out. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — operator surfaces name mid-software-build purpose-build

- When an SLM fits mid-software-build, or on demand, the same print-only entry is `make purpose-build-journey`. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. `docs/OPERATOR-DAY.md` §4, `docs/NORTH-STAR.md` (suite and the operator-loop purpose-build bullet), `docs/operator-enrich-journeys.md` sections 15–18, the README start-here line and opt-in one-liner, and the print banners of `scripts/purpose-build-journey.sh`, `scripts/purpose-build-pick.sh`, and `scripts/purpose-build-checklist.sh` name that entry. This slice does not invent a new live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). The re-prove card stays `make uniqueness-prove-checklist`. This slice does not move the GATE-90 or Cell One tip header (that stays through PR #189, `a857f05a256b214a2069de42e6918130eb080d66`). It does not wire those targets into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add DeepSeek or GLM journey code. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only.

## This slice — GATE-90 and Cell One tip honesty through PR #189

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #189 (`a857f05a256b214a2069de42e6918130eb080d66`). That tip names tip honesty through PR #187 (PR #188, `eb5ff41865d62db85355dbea7747ef4571941d43`) and operator surfaces naming `make purpose-build-journey` (PR #189). `docs/OPERATOR-DAY.md` and `docs/NORTH-STAR.md` name that target as the print-only purpose-build on-demand entry. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining for `make purpose-build-pick` names DeepSeek-R1-Distill chat print-only and GLM-4 Chat print-only. The picker does not run them. The PR #187 tip stays `4df2c56d5ab622ea3843ade97e4a0dec9ea5b01a`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #187

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #187 (`4df2c56d5ab622ea3843ade97e4a0dec9ea5b01a`). That tip names tip honesty through PR #185 (PR #186, `ef839830b9de29caa963cece687e35c83dd77af6`) and `make purpose-build-checklist` step 1 naming `make deepseek-r1-distill-journey`, `make uniqueness-deepseek`, the LoRA twins, `make glm4-chat-journey`, `make uniqueness-glm`, and the LoRA twins (operator section 15, PR #187). The checklist does not run them. `estate help enrich` and `estate help train` name those print pointers on the purpose-build path. The checklist still prints seven ordered steps. The PR #185 tip stays `54c28b968879fccbc157dd7d9fdf7c10e9d0d58c`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Operator section 15 stays print-only. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — operator surfaces name purpose-build on demand

- `docs/OPERATOR-DAY.md` and `docs/NORTH-STAR.md` name `make purpose-build-journey` as the print-only purpose-build on-demand entry. It runs `make purpose-build-pick`, then `make purpose-build-checklist`. Those two targets are the parts. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining for `make purpose-build-pick` names DeepSeek-R1-Distill chat print-only (`make deepseek-r1-distill-journey` and `make uniqueness-deepseek`, plus the LoRA twin `make deepseek-r1-distill-lora-journey` and `make uniqueness-deepseek-lora`) and GLM-4 Chat print-only (`make glm4-chat-journey` and `make uniqueness-glm`, plus the LoRA twin `make glm4-chat-lora-journey` and `make uniqueness-glm-lora`). The picker does not run them. This slice does not invent a new live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). The re-prove card stays `make uniqueness-prove-checklist`. This slice does not move the GATE-90 or Cell One tip header (that stays through PR #187, `4df2c56d5ab622ea3843ade97e4a0dec9ea5b01a`). It does not wire those targets into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — checklist names DeepSeek and GLM print journeys

- `make purpose-build-checklist` step 1 names `make deepseek-r1-distill-journey` and `make uniqueness-deepseek`, plus the LoRA twin `make deepseek-r1-distill-lora-journey` and `make uniqueness-deepseek-lora`, and `make glm4-chat-journey` and `make uniqueness-glm`, plus the LoRA twin `make glm4-chat-lora-journey` and `make uniqueness-glm-lora`. The checklist does not run them. `estate help enrich` and `estate help train` name those print pointers on the purpose-build path (operator section 15). The checklist still prints seven ordered steps. This slice does not invent a new live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not move the GATE-90 or Cell One tip header (that stays through PR #185, `54c28b968879fccbc157dd7d9fdf7c10e9d0d58c`). It does not wire those targets into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #185

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #185 (`54c28b968879fccbc157dd7d9fdf7c10e9d0d58c`). That tip names tip honesty through PR #183 (PR #184, `87482dbc847c364056df778fa6170a22efad4c8b`) and print-only `make glm4-chat-journey`, `make uniqueness-glm`, `make glm4-chat-lora-journey`, and `make uniqueness-glm-lora` (operator section 20, PR #185). `estate help enrich` and `estate help train` name that path as the print-only GLM-4 Chat ladder. The index help, the README opt-in one-liner, and the Makefile comments name that same path. The journeys print `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. They do not train, merge, convert, shell out to ollama, or promote. The PR #183 tip stays `b93e89f1983027a13008cc4be23f44756af0f22e`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Operator section 20 stays print-only. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #183

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #183 (`b93e89f1983027a13008cc4be23f44756af0f22e`). That tip names tip honesty through PR #181 (PR #182, `7e098a9b0ec360c952b2b1c2273654e44de6776c`) and print-only `make deepseek-r1-distill-journey`, `make uniqueness-deepseek`, `make deepseek-r1-distill-lora-journey`, and `make uniqueness-deepseek-lora` (operator section 19, PR #183). `estate help enrich` and `estate help train` name that path as the print-only DeepSeek-R1-Distill chat ladder. The index help, the README opt-in one-liner, and the Makefile comments name that same path. The journeys print `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. They do not train, merge, convert, shell out to ollama, or promote. The PR #181 tip stays `c4a6d255a08146613c9c6cba262959d913f5cac0`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Operator section 19 stays print-only. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add GLM. It does not add a runtime write path or an ollama shell-out. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — print-only GLM-4 Chat journey

- `make glm4-chat-journey` (`scripts/glm4-chat-journey.sh`) and `make uniqueness-glm` (`scripts/uniqueness-glm.sh`) are the print-only GLM-4 Chat QLoRA ladder (operator section 20). The card is `llamafactory-qlora`. The fixture is `examples/fixtures/glm4-chat.pack.json`. Seat tag `llama3`. Train base `zai-org/glm-4-9b-chat`. Template `glm4`. That id is the DEFAULT DownloadSource for GLM-4-9B-Chat in the LLaMA-Factory template `glm4` group. `make uniqueness-glm` runs the prepare-assert phase, then the seat-print phase. It does not run `make qlora-journey` or `make uniqueness-full`. `make glm4-chat-lora-journey` and `make uniqueness-glm-lora` (`scripts/uniqueness-glm-lora.sh`) are the non-quant twin on `llamafactory-lora` and `examples/fixtures/glm4-chat-lora.pack.json`. Rank 8. No quantization. That chain does not run `make uniqueness-glm`. `estate help enrich` and `estate help train` name these targets. The index help, the README opt-in one-liner, and the Makefile comments name that same path. Both print `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. This slice does not invent a new live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not move the GATE-90 or Cell One tip header (that stays through PR #183, `b93e89f1983027a13008cc4be23f44756af0f22e`). It does not wire these targets into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a trainer, a fixture pack, or a driver. It does not add Kimi. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #181

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #181 (`c4a6d255a08146613c9c6cba262959d913f5cac0`). That tip names tip honesty through PR #179 (PR #180, `3fae4d53821966acfa69f688ba8de5405b20513e`) and print-only `make purpose-build-journey` (operator section 18, PR #181). `estate help enrich` and `estate help train` name that path as the print-only purpose-build on-demand entry. The index help, the README opt-in one-liner, and the Makefile comment name that same path. The journey runs `make purpose-build-pick`, then `make purpose-build-checklist`. It calls those targets through make and does not inline their bodies. It does not train, fuse, convert, shell out to ollama, promote, or apply the estate. The PR #179 tip stays `4772e0f001a9422cefa8e6f2a9378836285068c7`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Operator section 18 stays print-only. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — print-only DeepSeek-R1-Distill journey

- `make deepseek-r1-distill-journey` (`scripts/deepseek-r1-distill-journey.sh`) and `make uniqueness-deepseek` (`scripts/uniqueness-deepseek.sh`) are the print-only DeepSeek-R1-Distill chat QLoRA ladder (operator section 19). The card is `llamafactory-qlora`. The fixture is `examples/fixtures/deepseek-r1-distill.pack.json`. Seat tag `llama3`. Train base `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`. Template `deepseekr1`. `make uniqueness-deepseek` runs the prepare-assert phase, then the seat-print phase. It does not run `make qlora-journey` or `make uniqueness-full`. `make deepseek-r1-distill-lora-journey` and `make uniqueness-deepseek-lora` (`scripts/uniqueness-deepseek-lora.sh`) are the non-quant twin on `llamafactory-lora` and `examples/fixtures/deepseek-r1-distill-lora.pack.json`. Rank 8. No quantization. That chain does not run `make uniqueness-deepseek`. `estate help enrich` and `estate help train` name these targets. The index help, the README opt-in one-liner, and the Makefile comments name that same path. Both print `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. This slice does not invent a new live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not move the GATE-90 or Cell One tip header (that stays through PR #181, `c4a6d255a08146613c9c6cba262959d913f5cac0`). It does not wire these targets into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a trainer, a fixture pack, or a driver. It does not add Kimi. It does not add GLM. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — print-only purpose-build journey

- `make purpose-build-journey` (`scripts/purpose-build-journey.sh`) is the print-only purpose-build on-demand entry (operator section 18). It runs `make purpose-build-pick`, then `make purpose-build-checklist`. It calls those targets through make and does not inline their bodies. It does not resolve or execute estate beyond what those targets already do. It does not train, fuse, convert, shell out to ollama, promote, or apply the estate. `estate help enrich` and `estate help train` name that path. The index help, the README opt-in one-liner, and the Makefile comment name that same path. `make purpose-build-checklist`, `make purpose-build-pick`, `make mlx-lm-lora-journey`, and `make uniqueness-mlx` stay. The re-prove card stays `make uniqueness-prove-checklist`. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. This slice does not invent a new live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not move the GATE-90 or Cell One tip header (that stays through PR #179, `4772e0f001a9422cefa8e6f2a9378836285068c7`). It does not wire `make purpose-build-journey` into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a train family, a fixture pack, or a driver. It does not add a uniqueness-* alias. It does not add Kimi. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #179

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #179 (`4772e0f001a9422cefa8e6f2a9378836285068c7`). That tip names tip honesty through PR #177 (PR #178, `215933daa8a0c6789ab449ffd80fa9c786e5905e`) and print-only `make purpose-build-pick` (operator section 17, PR #179). `estate help enrich` and `estate help train` name that path as the print-only host and stack picker for purpose-build journeys. The index help, the README opt-in one-liner, and the Makefile comment name that same path. The picker prints a host and stack table and does not run the named targets. It does not train, fuse, convert, shell out to ollama, or promote. The PR #177 tip stays `3fb3e8d48b1d6fcc92a88d2b2038faff98dae0be`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Operator section 17 stays print-only. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — print-only purpose-build host picker

- `make purpose-build-pick` (`scripts/purpose-build-pick.sh`) prints a host and stack table and does not run the named targets. Nvidia / CUDA primary is `make lf-beachhead-prepare`, `make qlora-journey`, and `make uniqueness-full`. Unsloth on that host stays optional (`make unsloth-qlora-journey`, `make uniqueness-unsloth`). Axolotl on that host stays integration (`make axolotl-qlora-journey`, `make uniqueness-axolotl`). Apple Silicon is `make mlx-lm-lora-journey` and `make uniqueness-mlx`, status optional. A stock pack whose `host_class_affinity` is `any` is `refuse:host` for `mlx-lm-lora`. Target A LoRA twins that already exist are `make lora-journey` / `make uniqueness-full-lora`, `make unsloth-lora-journey` / `make uniqueness-unsloth-lora`, and `make axolotl-lora-journey` / `make uniqueness-axolotl-lora`. `estate help enrich` and `estate help train` name `make purpose-build-pick` as the print-only host and stack picker for purpose-build journeys (operator section 17). The index help, the README opt-in one-liner, and the Makefile comment name that same path. Ordered steps after the pick stay `make purpose-build-checklist`. The re-prove card stays `make uniqueness-prove-checklist`. The picker does not resolve or execute estate. It does not train, fuse, convert, shell out to ollama, promote, or apply the estate. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. This slice does not invent a new live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not move the GATE-90 or Cell One tip header (that stays through PR #177, `3fb3e8d48b1d6fcc92a88d2b2038faff98dae0be`). It does not wire `make purpose-build-pick` into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #177

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #177 (`3fb3e8d48b1d6fcc92a88d2b2038faff98dae0be`). That tip names tip honesty through PR #175 (PR #176, `b31461e3cbe910dbbcd146a58d3335f37adced39`) and help that names `make mlx-lm-lora-journey` and `make uniqueness-mlx` (operator section 16, PR #177). `estate help enrich`, `estate help train`, the help index, the README opt-in one-liner, and the Makefile comments name that path as the print-only Apple Silicon mlx-lm LoRA journey. The journeys still assert the `MLX.md` handoff and print fuse, seat, and import against fixture stubs. They do not train, fuse, convert, shell out to ollama, or promote. The PR #175 tip stays `7cc330801c3b7f87f2b9ecd23a21d823d5b87b12`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Operator section 16 stays print-only. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — help names the mlx-lm LoRA journey

- `estate help enrich` and `estate help train` name `make mlx-lm-lora-journey` as the print-only Apple Silicon mlx-lm LoRA journey (operator section 16) and `make uniqueness-mlx` as the print-only chain of that journey. The index help names that same path. The README opt-in one-liner names both targets. The Makefile comments name operator section 16. The journeys still assert the `MLX.md` handoff and print fuse, seat, and import against fixture stubs. They do not train, fuse, convert, shell out to ollama, or promote. They do not invent a live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not move the GATE-90 or Cell One tip header (that stays through PR #175, `7cc330801c3b7f87f2b9ecd23a21d823d5b87b12`). It does not wire `make mlx-lm-lora-journey` or `make uniqueness-mlx` into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. Not native MLX. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #175

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #175 (`7cc330801c3b7f87f2b9ecd23a21d823d5b87b12`). That tip names tip honesty through PR #173 (PR #174, `2234e95f6aebd219e5e6733f108400782e6a623b`) and print-only `make mlx-lm-lora-journey` and `make uniqueness-mlx` (operator section 16, PR #175). The throwaway pack sets `host_class_affinity` to `apple-silicon`. The stock overnight pack stays `any` and is `refuse:host`. Seat tag `llama3`. Train base `Qwen/Qwen2.5-0.5B-Instruct`. The card writes `MLX.md` and does not call mlx-lm. `merge-adapt` prints `mlx_lm.fuse` with `--adapter-path`, `--save-path` `fused_model`, and `--export-gguf` (`ggml-model-f16.gguf`). `gguf-convert` stays `refuse:seat`. A fused MLX directory is `refuse:adapter` and `refuse:seat`. `local-seat --adapter` stays `refuse:adapter`. `import-trained` records `trained_shape` `gguf`. The proposal stays `auto_apply=false`. `make uniqueness-mlx` runs the prepare-assert phase, then the seat-print phase. It does not run `make unsloth-qlora-journey` or `make uniqueness-unsloth`. Both print `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `mlx-lm-lora` stays `status=optional` and `live=false`. Axolotl stays `status=integration` and `live=false`. Unsloth stays `status=optional` and `live=false`. The catalog already says optional. This slice does not relabel that card `integration`. The PR #173 tip stays `1002abcdaf648277e15750a44a16e53b324a251d`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. This slice does not wire the mlx journey into `make smoke`, `make gate-90`, or GitHub Actions. It does not add Kimi. It does not add a train family, a fixture pack, a driver, or a journey. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #173

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #173 (`1002abcdaf648277e15750a44a16e53b324a251d`). That tip names tip honesty through PR #171 (PR #172, `0839d6372160ba46763e50cc47e336b2daf6345f`) and `estate doctor` and `estate status` that print `mlx-lm-lora` as `status=optional` and `live=false` once each (PR #173). Axolotl stays `status=integration` and `live=false`. Unsloth stays `status=optional` and `live=false`. The catalog already says optional. This slice does not relabel that card `integration`. The card writes `MLX.md` and does not call mlx-lm. The PR #171 tip stays `651f31a4dd3a84dad270d40dbdd3892081b9380e`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. This slice does not add an mlx journey makefile target. It does not wire mlx into `make smoke`, `make gate-90`, or GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — print-only mlx-lm LoRA uniqueness and seat journey

- `make mlx-lm-lora-journey` (`scripts/mlx-lm-lora-journey.sh`) and `make uniqueness-mlx` (`scripts/uniqueness-mlx.sh`) are the print-only Apple Silicon card for the optional NEXT driver `mlx-lm-lora`. The throwaway pack sets `host_class_affinity` to `apple-silicon`. The stock overnight pack stays `any` and is `refuse:host`. Seat tag `llama3`. Train base `Qwen/Qwen2.5-0.5B-Instruct`. The card writes `MLX.md` and does not call mlx-lm. `merge-adapt` prints `mlx_lm.fuse` with `--adapter-path`, `--save-path` `fused_model`, and `--export-gguf` (`ggml-model-f16.gguf`). The adapter stub is `adapter_config.json` plus `adapters.safetensors`. `adapter_model.safetensors` is `refuse:adapter`. `gguf-convert` stays `refuse:seat`. A fused MLX directory is `refuse:adapter` and `refuse:seat`. `local-seat --adapter` stays `refuse:adapter`. `import-trained` records `trained_shape` `gguf`. The proposal stays `auto_apply=false`. `make uniqueness-mlx` runs the prepare-assert phase, then the seat-print phase. It does not run `make unsloth-qlora-journey` or `make uniqueness-unsloth`. Both print `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. This slice does not invent a live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove. Not native MLX. Not in `make smoke`, `make gate-90`, or GitHub Actions. operator section 16 of [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md). `make purpose-build-checklist` names that journey as a print pointer and does not run it. This slice does not move the GATE-90 or Cell One tip header (that stays through PR #173, `1002abcdaf648277e15750a44a16e53b324a251d`). It does not add Kimi. It does not add a train family. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — mlx-lm doctor and status train honesty

- `estate doctor` and `estate status` print `mlx-lm-lora` as `status=optional` and `live=false` once each. The catalog already says optional. This slice does not relabel that card `integration`. Axolotl stays `status=integration` and `live=false`. Unsloth stays `status=optional` and `live=false`. They report prepare records and do not claim a train, a promote, a live PASS, or a prepare count. Garbage or promoted `prepare.json` still fails closed. The card writes `MLX.md` and does not call mlx-lm. It is included under `--job train` only when `host_class_affinity` is `apple-silicon`. Another affinity is `refuse:host` when that driver is named, and `--all-drivers` omits it. This slice does not add an mlx journey makefile target. It does not wire mlx into `make smoke`, `make gate-90`, or GitHub Actions. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. It does not move the GATE-90 or Cell One tip header (that stays through PR #171, `651f31a4dd3a84dad270d40dbdd3892081b9380e`; tip honesty PR #172, `0839d6372160ba46763e50cc47e336b2daf6345f`). A later pack owns a full tip-honesty rewrite. It does not add Kimi. It does not add a train family, a fixture pack, a driver, or a journey. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #171

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #171 (`651f31a4dd3a84dad270d40dbdd3892081b9380e`). That tip names tip honesty through PR #169 (PR #170, `54a049cc9983ffe1d8b3f25827367d4058f47b68`) and help that names `make purpose-build-checklist` (operator section 15, PR #171). `estate help enrich`, `estate help train`, the help index, the README opt-in one-liner, and the Makefile comment name that target as the print-only operator path for purpose-building an SLM on demand. The checklist still prints seven ordered steps and does not run them. The re-prove card stays `make uniqueness-prove-checklist`. The PR #169 tip stays `aa374875f221be908ff473c8096eeea1c0f32846`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Operator section 15 stays print-only. That target stays off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — help names the purpose-build checklist

- `estate help enrich` and `estate help train` name `make purpose-build-checklist` as the print-only operator path for purpose-building an SLM on demand (operator section 15). The index help names that same path. The README opt-in one-liner names it. The Makefile comment on `purpose-build-checklist` names operator section 15. The checklist still prints seven ordered steps and does not run them. It does not train, convert, shell out to ollama, promote, or apply the estate. It does not invent a live PASS. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). The re-prove card stays `make uniqueness-prove-checklist`. This slice does not move the GATE-90 or Cell One tip header (that stays through PR #169, `aa374875f221be908ff473c8096eeea1c0f32846`). It does not wire `make purpose-build-checklist` into `make smoke`, `make gate-90`, or GitHub Actions. It does not add a train family, a fixture pack, a driver, or a journey. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #169

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #169 (`aa374875f221be908ff473c8096eeea1c0f32846`). That tip names tip honesty through PR #167 (PR #168, `45f8808dbb9d8c837619ae5015aa26777f46f0a6`) and print-only `make purpose-build-checklist` (operator section 15, PR #169). The checklist prints seven ordered steps (choose a train card, `SKIP live train`, merge and export print, gguf-convert print, local-seat print, `import-trained` with `trained_shape` `gguf` and `auto_apply=false`, Standing next (estate)) and does not run them. The re-prove card stays `make uniqueness-prove-checklist`. The PR #167 tip stays `c244e721d7275651ecfa1c8f4578ba008a668aa9`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. Operator section 15 stays print-only. That target stays off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — print-only purpose-build operator checklist

- `make purpose-build-checklist` (`scripts/purpose-build-checklist.sh`) prints the ordered operator steps for purpose-build on demand. Choose a train card (`make lf-beachhead-prepare`, or `make qlora-journey` / `make lora-journey`, with `make axolotl-qlora-journey`, `make axolotl-lora-journey`, `make unsloth-qlora-journey`, and `make unsloth-lora-journey` as optional paths). Then a train-next style handoff (`SKIP live train`), merge-adapt and export print honesty, `gguf-convert`, the local-seat print (it prints the Modelfile and does not write `$PREPARED/Modelfile`), and `import-trained` (`trained_shape` `gguf`, `auto_apply=false`). After that it prints Standing next (estate). The proposal stays `auto_apply=false`. The factory does not apply the estate without an explicit operator `--require-plan` path. The curator path is `packs accept --curator jason`. No promote and no auto-promote. The coda names `apply-proposal`, `plan`, `apply --require-plan`, and `reconcile` and does not execute them. The checklist does not run those print journeys. The factory does not train, convert, shell out to ollama, or promote. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The re-prove card stays `make uniqueness-prove-checklist`. The recorded section name is Target C live uniqueness (5090-class). operator section 15 of [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md). Not native MLX. Not in `make smoke`, `make gate-90`, or GitHub Actions. It does not move the GATE-90 or Cell One tip header (that stays through PR #167, `c244e721d7275651ecfa1c8f4578ba008a668aa9`). A later pack owns a full tip-honesty rewrite through PR #168. It does not add a train family, a fixture pack, a driver, or a journey. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #167

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #167 (`c244e721d7275651ecfa1c8f4578ba008a668aa9`). That tip names tip honesty through PR #165 (PR #166, `f5be4c896babc8be60e6bbcad46196acfa533fdc`) and Unsloth doctor and status train honesty (PR #167). `estate doctor` and `estate status` print `unsloth-qlora` and `unsloth-lora` as `status=optional` and `live=false` once each. Axolotl stays `status=integration`. Unsloth is not relabeled integration. They report prepare records and do not claim a train, a promote, a live PASS, or a prepare count. The PR #165 tip stays `7a1b3d54d37b38977c5570679d7f3922b9404d4a`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. operator section 12 and operator section 14 stay print-only. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — Unsloth doctor and status train honesty

- `estate doctor` and `estate status` print `unsloth-qlora` and `unsloth-lora` as `status=optional` and `live=false`. The catalog already says optional. This slice does not relabel those cards `integration`. They report prepare records and do not claim a train, a promote, a live PASS, or a prepare count. Garbage or promoted `prepare.json` still fails closed. Operator section 12 (`make unsloth-qlora-journey`, `make uniqueness-unsloth`) and operator section 14 (`make unsloth-lora-journey`, `make uniqueness-unsloth-lora`) stay print-only. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. It does not move the GATE-90 or Cell One tip header (that stays through PR #165, `7a1b3d54d37b38977c5570679d7f3922b9404d4a`). A later pack owns a full tip-honesty rewrite. It does not add Kimi. It does not add a train family, a fixture pack, a driver, or a journey. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #165

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #165 (`7a1b3d54d37b38977c5570679d7f3922b9404d4a`). That tip names tip honesty through PR #161 (PR #163, `7a08b76b7f9f95922758c260351b310de66a74e4`), Axolotl doctor and status train honesty (PR #164, `bd5a515901852b95d1415bb55eb557e5ed424d8d`), and print-only `make unsloth-lora-journey` and `make uniqueness-unsloth-lora` (operator section 14, PR #165). `estate doctor` and `estate status` print `axolotl-lora` and `axolotl-qlora` as `status=integration` and `live=false`. They report prepare records and do not claim a train or a promote. The PR #161 tip stays `0cc8b9e5bf2423d90f54f222a182804b3b337d94`. The recorded 5090-class Target C live uniqueness PASS stays the only live uniqueness prove, in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143, and names the print-only `make unsloth-lora-journey` and `make uniqueness-unsloth-lora` rows. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — print-only Unsloth LoRA uniqueness and seat journey

- `make unsloth-lora-journey` (`scripts/unsloth-lora-journey.sh`) prepares `unsloth-lora` on a throwaway copy of `examples/estate.yaml` (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, pack `examples/fixtures/specialist-overnight.pack.json`). `unsloth-lora` is an optional NEXT card (`status=optional`), the non-quant twin of `unsloth-qlora`. It writes `UNSLOTH.md`, `PREPARE.md`, `NEXT.md`, and `prepare.json` with the seat tag / train base split. It does not write a script, a recipe, `dataset.jsonl`, or `load_in_4bit`. A seat tag with no train base is `refuse:train-base`. `--official-scale` on this card alone is `refuse:official-scale`. `--from-feed` on this card alone is `refuse:dataset`. A missing adapter, an adapter directory without `adapter_model.safetensors`, and `adapters.safetensors` are `refuse:adapter`. `local-seat --adapter` stays `refuse:adapter`. A missing merged directory or GGUF is `refuse:seat`. Before the good stub, a Qwen-shaped merged directory is `refuse:tokenizer`. `merge-adapt` prints `save_pretrained_merged` with `save_method` `merged_16bit` and the documented LoRA save (`save_method` `lora`). `gguf-convert`, `local-seat`, and `import-trained` then print against fixture stubs. `import-trained` records `trained_shape` `gguf`. The proposal stays `auto_apply=false`. The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` and `CELL_TRAIN_LIVE=1` stay print-only. It does not call Unsloth, does not convert, does not shell out to ollama, and does not promote. `make uniqueness-unsloth-lora` (`scripts/uniqueness-unsloth-lora.sh`) chains the prepare-assert phase, then the seat-print phase. That chain does not run `make unsloth-qlora-journey`, `make uniqueness-unsloth`, `make axolotl-lora-journey`, `make uniqueness-axolotl-lora`, `make qlora-journey`, `make seat-journey`, `make train-next`, or `make lf-beachhead-prepare`. `make uniqueness-unsloth` stays the QLoRA card. This slice does not invent an Unsloth stack, a recipe YAML, or a dataset writer. It does not add Kimi. It does not weaken refuse paths. It does not rewrite `docs/GATE-90.md`. It does not move the GATE-90 or Cell One tip header (that stays through PR #161, `0cc8b9e5bf2423d90f54f222a182804b3b337d94`). Operator docs and the Cell One body may name this journey. A separate tip-honesty pack owns a full GATE-90 rewrite. Not in `make smoke`, `make gate-90`, or GitHub Actions. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #161

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #161 (`0cc8b9e5bf2423d90f54f222a182804b3b337d94`). That tip names tip honesty for the Standing next coda (PR #158, `11ff8a84f2ffe37b663ed293d9b00ff415f69c8e`), print-only `make uniqueness-full-lora` (PR #159, `9bc8db52ee157bc7c50261362c7790d4248ed4b4`), print-only `make axolotl-qlora-journey` and `make uniqueness-axolotl` (PR #160, `1b06ea067f7cec44649fbc3aba6650dc0ad6b0f9`), print-only `make unsloth-qlora-journey` and `make uniqueness-unsloth` (operator section 12, PR #162, `e60d201af8c852e7b54eda7e2e37deb52428e38a`), and print-only `make axolotl-lora-journey` and `make uniqueness-axolotl-lora` (operator section 13, PR #161). The Standing next (estate) coda stays PR #157 (`6a43ff12a91295739c5a9c8a8f1dc9cc9c084466`). The recorded 5090-class Target C live uniqueness PASS stays in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143, and keeps the print-only journey rows that already exist, including `make uniqueness-full-lora`, `make axolotl-qlora-journey`, `make uniqueness-axolotl`, `make unsloth-qlora-journey`, `make uniqueness-unsloth`, `make axolotl-lora-journey`, and `make uniqueness-axolotl-lora`. Those targets stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — print-only Axolotl LoRA uniqueness and seat journey

- `make axolotl-lora-journey` (`scripts/axolotl-lora-journey.sh`) prepares `axolotl-lora` on a throwaway copy of `examples/estate.yaml` (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, pack `examples/fixtures/specialist-overnight.pack.json`). It checks `axolotl.yml` against the documented card (`adapter: lora`, `load_in_8bit: false`, `load_in_4bit: false`, `sequence_len: 2048`, `lora_r: 16`, train base in `base_model`, matching Axolotl `examples/llama-3/lora-1b.yml`). A seat tag with no train base is `refuse:train-base` and writes nothing. A missing adapter is `refuse:adapter`. A missing merged directory or GGUF is `refuse:seat`. The seat print then uses fixture stubs: `outputs/adapter_config.json`, `outputs/merged` with `config.json` plus a non-adapter `.safetensors` file, and `outputs/merged.gguf` starting with GGUF magic. Before the good stub, a Qwen-shaped merged directory is `refuse:tokenizer` and does not print the convert line. `merge-adapt` prints `axolotl merge-lora` without `--dequant`. `gguf-convert` prints `python3 convert_hf_to_gguf.py` with `--outtype auto`. `local-seat` prints `ollama create` and does not write the Modelfile. `import-trained` records `trained_shape` `gguf`. The proposal stays `auto_apply=false`. The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` and `CELL_TRAIN_LIVE=1` stay print-only. It does not run Axolotl, does not convert, does not shell out to ollama, and does not promote. `make uniqueness-axolotl-lora` (`scripts/uniqueness-axolotl-lora.sh`) chains the prepare-assert phase, then the seat-print phase, and exits nonzero before the seat print if prepare fails. That chain does not run `make axolotl-qlora-journey`, `make uniqueness-axolotl`, `make qlora-journey`, `make seat-journey`, `make train-next`, or `make lf-beachhead-prepare`. `make uniqueness-axolotl` stays the 4-bit card. This slice does not invent an Axolotl, Unsloth, or llama.cpp trainer stack. It does not add Kimi. It does not weaken refuse paths. It does not move tip framing (that stays through PR #157, `6a43ff12a91295739c5a9c8a8f1dc9cc9c084466`). Not in `make smoke`, `make gate-90`, or GitHub Actions. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — print-only Unsloth QLoRA uniqueness and seat journey

- `make unsloth-qlora-journey` (`scripts/unsloth-qlora-journey.sh`) prepares `unsloth-qlora` on a throwaway copy of `examples/estate.yaml` (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, pack `examples/fixtures/specialist-overnight.pack.json`). It asserts `UNSLOTH.md`, `PREPARE.md`, `NEXT.md`, and the `prepare.json` seat tag / train base split. The card stays an operator-owned handoff. It does not write a script, a recipe, or `dataset.jsonl`. A seat tag with no train base is `refuse:train-base` and writes nothing. A missing adapter, an adapter directory without `adapter_model.safetensors`, and `adapters.safetensors` are `refuse:adapter`. A merged directory or a GGUF passed as `--adapter` is `refuse:adapter`. `local-seat --adapter` stays `refuse:adapter`. A missing merged directory or GGUF is `refuse:seat`. The seat print then uses fixture stubs: `adapter/adapter_config.json` plus `adapter_model.safetensors`, `merged` with `config.json` plus a non-adapter `.safetensors` file, and `merged.gguf` starting with GGUF magic. Before the good stub, a Qwen-shaped merged directory is `refuse:tokenizer` and does not print the convert line. `merge-adapt` prints `save_pretrained_merged` with `save_method` `merged_16bit`. `gguf-convert` prints `python3 convert_hf_to_gguf.py` with `--outtype auto` and the three manual lines (`f16`, `bf16`, `q8_0`). `local-seat` prints `ollama create` and does not write the Modelfile. `import-trained` records `trained_shape` `gguf`. The proposal stays `auto_apply=false`. The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` and `CELL_TRAIN_LIVE=1` stay print-only. It does not call Unsloth, does not convert, does not shell out to ollama, and does not promote. `make uniqueness-unsloth` (`scripts/uniqueness-unsloth.sh`) chains the prepare-assert phase, then the seat-print phase, and exits nonzero before the seat print if prepare fails. That chain does not run `make qlora-journey`, `make seat-journey`, `make train-next`, `make axolotl-qlora-journey`, or `make lf-beachhead-prepare`. Status stays `optional`. This slice does not invent an Unsloth stack, a recipe YAML, or a dataset writer. It does not add Kimi. It does not weaken refuse paths. It does not rewrite `docs/GATE-90.md` or `docs/CELL-ONE-STATUS.md`. It does not move tip framing (that stays through PR #157, `6a43ff12a91295739c5a9c8a8f1dc9cc9c084466`). `make uniqueness-axolotl` stays the Axolotl QLoRA chain. Not in `make smoke`, `make gate-90`, or GitHub Actions. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — print-only Axolotl QLoRA uniqueness and seat journey

- `make axolotl-qlora-journey` (`scripts/axolotl-qlora-journey.sh`) prepares `axolotl-qlora` on a throwaway copy of `examples/estate.yaml` (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, pack `examples/fixtures/specialist-overnight.pack.json`). It checks `axolotl.yml` against the documented card (`adapter: qlora`, `load_in_4bit: true`, `sequence_len: 4096`, `lora_r: 32`, train base in `base_model`, matching Axolotl `examples/llama-3/qlora.yml`). A seat tag with no train base is `refuse:train-base` and writes nothing. A missing adapter is `refuse:adapter`. A missing merged directory or GGUF is `refuse:seat`. The seat print then uses fixture stubs: `outputs/adapter_config.json`, `outputs/merged` with `config.json` plus a non-adapter `.safetensors` file, and `outputs/merged.gguf` starting with GGUF magic. Before the good stub, a Qwen-shaped merged directory is `refuse:tokenizer` and does not print the convert line. `merge-adapt` prints `axolotl merge-lora`, including `--dequant`. `gguf-convert` prints `python3 convert_hf_to_gguf.py` with `--outtype auto`. `local-seat` prints `ollama create` and does not write the Modelfile. `import-trained` records `trained_shape` `gguf`. The proposal stays `auto_apply=false`. The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `CELL_SEAT_LIVE=1` and `CELL_TRAIN_LIVE=1` stay print-only. It does not run Axolotl, does not convert, does not shell out to ollama, and does not promote. `make uniqueness-axolotl` (`scripts/uniqueness-axolotl.sh`) chains the prepare-assert phase, then the seat-print phase, and exits nonzero before the seat print if prepare fails. That chain does not run `make qlora-journey`, `make seat-journey`, `make train-next`, or `make lf-beachhead-prepare`. This slice does not invent an Axolotl, Unsloth, or llama.cpp trainer stack. It does not add Kimi. It does not weaken refuse paths. It does not rewrite `docs/GATE-90.md` or `docs/CELL-ONE-STATUS.md`. It does not move tip framing (that stays through PR #157, `6a43ff12a91295739c5a9c8a8f1dc9cc9c084466`). `make uniqueness-full-lora` stays the Target A chain. Not in `make smoke`, `make gate-90`, or GitHub Actions. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — print-only Target A uniqueness-full-lora

- `make uniqueness-full-lora` (`scripts/uniqueness-full-lora.sh`) runs the Target A print chain in order: `make lora-journey`, then `make train-next-lora`, then `make seat-journey-lora`. If one step fails, the script exits nonzero before the next step. Print-only. It does not train, merge, convert, seat, or promote. It does not run `make lf-beachhead-prepare`. `make uniqueness-full` stays `make qlora-journey`, then `make train-next`, then `make seat-journey`. `make uniqueness-ladder` stays `make qlora-journey` then `make seat-journey` and does not run `make train-next`.
- `make train-next-lora` prepares `llamafactory-lora` on a throwaway copy of `examples/estate.yaml` (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`, template `qwen`, rank 8, no quantization) and prints the `NEXT.md` train recipe: `pip install llamafactory` and `llamafactory-cli train` on that `recipe.yaml`. It also prints the `llamafactory-cli export` line and does not run it. It does not print `pip install 'bitsandbytes>=0.49'`. `CELL_TRAIN_LIVE=1` stays print-only. A missing `llamafactory-cli` is an informational SKIP. `make train-next` stays the QLoRA card.
- `make seat-journey-lora` runs `scripts/seat-journey.sh` with `SEAT_CARD=llamafactory-lora`. `make seat-journey` pins `SEAT_CARD=llamafactory-qlora`. The LoRA path keeps `refuse:adapter`, `refuse:tokenizer`, and `refuse:seat`, then the same fixture stubs (`adapter_config.json`, the 5090-shaped export, `config.json` plus `model.safetensors`, and a GGUF that starts with GGUF magic). `import-trained` records `trained_shape` `gguf` and the proposal stays `auto_apply=false`. The refuses are not weakened.
- Live train, live convert, and live seat still need a human GPU host and stay skipped. This slice does not invent a live PASS. Not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no. `examples/estate.yaml` stays hash-locked (`43770130 3391`).

## This slice — GATE-90 and Cell One tip honesty through PR #157

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #157 (`6a43ff12a91295739c5a9c8a8f1dc9cc9c084466`). That tip names tip honesty through PR #155 (PR #156, `87072dbf79b302dc2dce42e49937abbbdd6ac689`) and the Standing next (estate) coda on `make uniqueness-prove-checklist` (PR #157). The recorded 5090-class Target C live uniqueness PASS stays in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. The tip story still names the estate PATH / `ESTATE_BIN` fallback for journeys (PR #148, `ac8348a2d5cd0979ecf285c51153d12b3233442c`), the tokenizer restore dereference after `refuse:tokenizer` (PR #149, `a272d39b731996524883e1124478dddc3f76935c`), the print-only local-seat Modelfile that does not write `$PREPARED/Modelfile` (PR #150, `663c806ae72826cff664ea32e8a370e059ba83d4`), help/docs that scope that write line to GGUF print-only seats (PR #151, `ecbe8a1c9e5ebe80d581f2c82a2c194cf165aa67`), docs-and-locks tip honesty for that line (PR #152, `b4f9c321f2c21c4cac1eb24b8c5b002035318ce4`), tip honesty through PR #153 (PR #154, `4f0a2096dcf768ce988f320706ad7319aa7d489e`), and the print-only uniqueness prove checklist (PR #155, `cbecb0b554a655a5276e0c75b8fdc59d55c77f76`). The uniqueness print chain stays `make uniqueness-ladder` (PR #145), `make train-next` (PR #146), and `make uniqueness-full` (PR #147). `make uniqueness-ladder`, `make uniqueness-full`, `make train-next`, `make qlora-journey`, `make lora-journey`, `make seat-journey`, `make uniqueness-prove-checklist`, and `make lf-beachhead-prepare` stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — Standing next (estate) after import-trained

- `make uniqueness-prove-checklist` (`scripts/uniqueness-prove-checklist.sh`) prints Standing next (estate) after checklist step 8 (`import-trained`, `trained_shape` `gguf`, `auto_apply=false`). The proposal stays `auto_apply=false`. The factory does not apply the estate without an explicit operator `--require-plan` path. The curator path is `packs accept --curator jason`. No promote and no auto-promote. `examples/estate.yaml` stays unchanged unless the operator deliberately applies a plan. The coda names the existing entrypoints and does not execute them: `estate enrich apply-proposal`, `estate plan`, `estate plan diff`, `estate plan export-pr`, `estate apply --dry-run`, `estate apply` without `--require-plan` (source estate unchanged), `estate apply --require-plan`, `estate apply --require-plan --require-fresh-plan`, `estate reconcile`, `estate reconcile --suggest` (patch file only, never auto-applies), and `estate packs accept --curator jason`. `estate packs promote` and `estate feed promote` always fail. Auto-promote stays locked off. The recorded PASS stays in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (Target C live uniqueness (5090-class)). The re-prove card is `make uniqueness-prove-checklist`. Phrase-checks refuse a coda that claims the factory applied, promoted, trained, converted, or shelled out to ollama. The factory does not train, convert, shell out to ollama, or promote. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. This slice does not invent a new live PASS. Not native MLX. Not in `make smoke`, `make gate-90`, or GitHub Actions. Tip framing stays through PR #155 (`cbecb0b554a655a5276e0c75b8fdc59d55c77f76`). This slice does not move that tip. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #155

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #155 (`cbecb0b554a655a5276e0c75b8fdc59d55c77f76`). That tip names tip honesty through PR #153 (PR #154, `4f0a2096dcf768ce988f320706ad7319aa7d489e`) and the print-only uniqueness prove checklist (`make uniqueness-prove-checklist`, PR #155). The recorded 5090-class Target C live uniqueness PASS stays in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153, `16cea97d56079a60c033c7a10468ddd7092a2ef1`). This slice does not invent a new live PASS. The factory does not train, convert, shell out to ollama, or promote. The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. The tip story still names the estate PATH / `ESTATE_BIN` fallback for journeys (PR #148, `ac8348a2d5cd0979ecf285c51153d12b3233442c`), the tokenizer restore dereference after `refuse:tokenizer` (PR #149, `a272d39b731996524883e1124478dddc3f76935c`), the print-only local-seat Modelfile that does not write `$PREPARED/Modelfile` (PR #150, `663c806ae72826cff664ea32e8a370e059ba83d4`), help/docs that scope that write line to GGUF print-only seats (PR #151, `ecbe8a1c9e5ebe80d581f2c82a2c194cf165aa67`), and docs-and-locks tip honesty for that line (PR #152, `b4f9c321f2c21c4cac1eb24b8c5b002035318ce4`). The uniqueness print chain stays `make uniqueness-ladder` (PR #145), `make train-next` (PR #146), and `make uniqueness-full` (PR #147). `make uniqueness-ladder`, `make uniqueness-full`, `make train-next`, `make qlora-journey`, `make lora-journey`, `make seat-journey`, `make uniqueness-prove-checklist`, and `make lf-beachhead-prepare` stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — print-only Target C uniqueness prove checklist

- `make uniqueness-prove-checklist` (`scripts/uniqueness-prove-checklist.sh`) prints the ordered operator steps for the recorded Target C live uniqueness ladder in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md). Prepare is the LLaMA-Factory QLoRA card (`llamafactory-qlora`, seat `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`). Train and export stay outside the factory (`NEXT.md` recipe, `SKIP live train`). After `refuse:tokenizer`, tokenizer restore uses dereference (`cp -aL` or `cp --dereference`). Then `estate enrich gguf-convert`, `estate enrich local-seat` (print-only; does not write `$PREPARED/Modelfile`), the operator writes the Modelfile from the printed contents, `ollama create` outside the factory, `estate enrich import-trained` (`trained_shape` `gguf`, `auto_apply=false`), and cleanup (`ollama rm`) with `examples/estate.yaml` unchanged. The factory does not train, convert, shell out to ollama, or promote. `CELL_TRAIN_LIVE=1` and `CELL_SEAT_LIVE=1` stay print-only. The recorded PASS stays on that LIVE-PROBES section. This checklist does not invent a new live PASS. Not native MLX. Not in `make smoke`, `make gate-90`, or GitHub Actions. Tip framing stays through PR #153 (`16cea97d56079a60c033c7a10468ddd7092a2ef1`). `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no.

## This slice — GATE-90 and Cell One tip honesty through PR #153

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #153 (`16cea97d56079a60c033c7a10468ddd7092a2ef1`). That tip names the docs-and-locks tip honesty for PR #151 (PR #152, `b4f9c321f2c21c4cac1eb24b8c5b002035318ce4`) and the recorded 5090-class Target C live uniqueness PASS in [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) (PR #153). The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. The tip story still names the estate PATH / `ESTATE_BIN` fallback for journeys (PR #148, `ac8348a2d5cd0979ecf285c51153d12b3233442c`), the tokenizer restore dereference after `refuse:tokenizer` (PR #149, `a272d39b731996524883e1124478dddc3f76935c`), the print-only local-seat Modelfile that does not write `$PREPARED/Modelfile` (PR #150, `663c806ae72826cff664ea32e8a370e059ba83d4`), and help/docs that scope that write line to GGUF print-only seats (PR #151, `ecbe8a1c9e5ebe80d581f2c82a2c194cf165aa67`). `make uniqueness-ladder`, `make uniqueness-full`, `make train-next`, `make qlora-journey`, `make lora-journey`, `make seat-journey`, and `make lf-beachhead-prepare` stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — Target C live uniqueness prove on a 5090-class host

- `docs/LIVE-PROBES.md` records the 2026-09-23 Target C live uniqueness **PASS** on a Linux 5090-class host (`consumer-nvidia` / `rented-nvidia`). The outside-factory ladder was prepare, LLaMA-Factory train and export, tokenizer restore with dereference, `gguf-convert`, `local-seat` print, write the Modelfile from the printed contents, `ollama create` outside the factory (`cell-target-c-qlora-prove`), `import-trained` (`trained_shape` `gguf`, `auto_apply=false`), and `ollama rm`. Workdir `/tmp/cell-one-target-c-live-20260923`. The factory did not train, convert, shell out to ollama, or promote. `examples/estate.yaml` stays hash-locked (`43770130 3391`). `READY_FOR_LIVE_TEST`: no. Not in `make smoke`, `make gate-90`, or GitHub Actions. Not native MLX. Not `estate probes --live`. Tip framing stays through PR #151. This slice does not add a train family, a journey, or a runtime write path.

## This slice — GATE-90 and Cell One tip honesty through PR #151

- `docs/GATE-90.md` and `docs/CELL-ONE-STATUS.md` name tip through PR #151 (`ecbe8a1c9e5ebe80d581f2c82a2c194cf165aa67`). The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare` via PR #143. The tip story also names the estate PATH / `ESTATE_BIN` fallback for journeys (PR #148, `ac8348a2d5cd0979ecf285c51153d12b3233442c`), the tokenizer restore dereference after `refuse:tokenizer` (PR #149, `a272d39b731996524883e1124478dddc3f76935c`), the print-only local-seat Modelfile that does not write `$PREPARED/Modelfile` (PR #150, `663c806ae72826cff664ea32e8a370e059ba83d4`), and help/docs that scope that write line to GGUF print-only seats (PR #151). `make uniqueness-ladder`, `make uniqueness-full`, `make train-next`, `make qlora-journey`, `make lora-journey`, `make seat-journey`, and `make lf-beachhead-prepare` stay off `make smoke`, `make gate-90`, and GitHub Actions. This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It does not add a runtime write path or an ollama shell-out. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — on-disk Modelfile is not a rewrite

- `estate help enrich`, `docs/local-seat.md`, `docs/TRAIN-ENRICH.md`, and `docs/operator-enrich-journeys.md` keep the GGUF print-only sentence: local-seat is print-only. It prints the Modelfile and does not write `$PREPARED/Modelfile`. Write that file from the printed contents before `ollama create`. When `modelfile_on_disk=true`, the report uses the on-disk Modelfile when FROM already names the artifact. Do not write `$PREPARED/Modelfile` again. That covers a merged `--weights` export whose LLaMA-Factory Modelfile is already on disk, and a GGUF whose FROM already names that file. The command does not write the file, does not shell out to ollama, and does not promote. `make seat-journey` stays off `make smoke`, `make gate-90`, and GitHub Actions. GATE-90 and Cell One tip stay through PR #143. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — print-only local-seat Modelfile

- `estate enrich local-seat` on a GGUF says local-seat is print-only. It prints the Modelfile and does not write `$PREPARED/Modelfile`. Write that file from the printed contents before `ollama create`. The same line is on `estate help enrich`, `NEXT.md`, `make seat-journey`, `docs/local-seat.md`, `docs/TRAIN-ENRICH.md`, and `docs/operator-enrich-journeys.md`. The command does not write the file, does not shell out to ollama, and does not promote. Two seating passages (`docs/TRAIN-ENRICH.md` step 2 and `docs/operator-enrich-journeys.md` section 8) name `refuse:tokenizer` before the re-run. `make seat-journey` stays off `make smoke`, `make gate-90`, and GitHub Actions. GATE-90 and Cell One tip stay through PR #143. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — name dereference when restoring tokenizer files

- After `refuse:tokenizer`, the printed restore (`tokenizer_restore_sentence`, `export_tokenizer_guidance`, `estate help enrich`, `NEXT.md`, `PREPARE.md`, and the LLaMA-Factory `merge-adapt` report) says HF hub snapshots are often symlinks into the HF cache. Copy tokenizer files with dereference (`cp -aL` or `cp --dereference`, or the equivalent) so the files in the export directory are real files, not symlinks. A plain `cp -a` leaves `tokenizer_config.json` as a symlink. Enrich does not follow a symlinked `tokenizer_config.json`. The same sentence is on `docs/TRAIN-ENRICH.md`, `docs/local-seat.md`, `docs/operator-enrich-journeys.md`, and `docs/CELL-ONE-STATUS.md`. JSON list, JSON null, and a Qwen-family export missing `vocab.json` or `merges.txt` stay the same refuse. The command does not download weights, does not copy tokenizer files, does not follow that symlink, does not write `tokenizer_config.json.bak`, and does not print `python3 convert_hf_to_gguf.py` on that refuse. `make seat-journey` still asserts the refuse on the 5090-shaped export and stays off `make smoke`, `make gate-90`, and GitHub Actions. GATE-90 and Cell One tip stay through PR #143. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — local estate binary for print journeys

- Target C print journeys resolve `estate` fail-closed when `cargo` is not on PATH. `scripts/qlora-journey.sh`, `scripts/train-next.sh`, `scripts/seat-journey.sh`, `scripts/lora-journey.sh`, `scripts/lf-beachhead-prepare.sh`, and `scripts/train-prepare.sh` use an executable `ESTATE_BIN` first, then `target/release/estate`, then `target/debug/estate`, then `cargo run -q -p estate-control --` when `cargo` is on PATH. A set `ESTATE_BIN` that is not executable does not fall through. If none resolve, the script exits nonzero and names `ESTATE_BIN`, `target/release/estate`, and `target/debug/estate`. `make uniqueness-full` and `make uniqueness-ladder` still only call `make`. Print-only. `READY_FOR_LIVE_TEST`: no. `examples/estate.yaml` stays hash-locked. Not in `make smoke`, `make gate-90`, or GitHub Actions.

## This slice — print-only Target C uniqueness-full

- `make uniqueness-full` (`scripts/uniqueness-full.sh`) runs the Target C print chain in order: `make qlora-journey`, then `make train-next`, then `make seat-journey`. If one step fails, the script exits nonzero before the next step. Print-only. It does not train, merge, convert, seat, or promote. It does not run `make lf-beachhead-prepare`. `make uniqueness-ladder` stays `make qlora-journey` then `make seat-journey` and does not run `make train-next`. Live train, live convert, and live seat still need a human GPU host and stay skipped. Not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no. `examples/estate.yaml` stays hash-locked.

## This slice — print-only Target C train-next

- `make train-next` (`scripts/train-next.sh`) prepares `llamafactory-qlora` on a throwaway copy of `examples/estate.yaml` (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`) and prints the `NEXT.md` train recipe: `pip install llamafactory`, `pip install 'bitsandbytes>=0.49'`, and `llamafactory-cli train` on that `recipe.yaml`. It also prints the `llamafactory-cli export` line and does not run it. `CELL_TRAIN_LIVE=1` stays print-only. A missing `llamafactory-cli` or bitsandbytes is an informational SKIP. `make uniqueness-ladder` stays `make qlora-journey` then `make seat-journey` and does not run this target. Not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no. `examples/estate.yaml` stays hash-locked.

## This slice — print-only Target C uniqueness ladder

- `make uniqueness-ladder` (`scripts/uniqueness-ladder.sh`) runs the existing Target C print journeys in order: `make qlora-journey`, then `make seat-journey`. Print-only. It does not train, merge, convert, seat, or promote. It does not run `make lf-beachhead-prepare`. Live train, live convert, and live seat still need a human GPU host and stay skipped. Not in `make smoke`, `make gate-90`, or GitHub Actions. `READY_FOR_LIVE_TEST`: no.

## This slice — Cell One status tip honesty through PR #143

- `docs/CELL-ONE-STATUS.md` names tip through PR #143 (`3acdec3983ea581976649ba4b7cc41a4cd22d31d`). The LLaMA-Factory beachhead matrix stays PR #140. The print-only prepare walk stays PR #142. [`docs/GATE-90.md`](docs/GATE-90.md) Remaining names `make lf-beachhead-prepare`. `READY_FOR_LIVE_TEST`: no.

## This slice — Day-90 gate tip honesty through PR #142

- `docs/GATE-90.md` points the live tip story at [`docs/CELL-ONE-STATUS.md`](docs/CELL-ONE-STATUS.md) through PR #142 (`d2dcdb97c2c960e8b93715391d77075055a8b0ce`, the print-only LLaMA-Factory beachhead prepare walk). `make gate-90` stays local `cargo test`. Hosted CI stays compile-only. The Remaining table names `make lf-beachhead-prepare`: an opt-in print-only prepare walk of the 16 beachhead matrix fixtures. It checks prepare artifacts. It does not train. It is not in smoke or Actions. It is not a live train. `make qlora-journey`, `make lora-journey`, and `make seat-journey` stay on that table. `READY_FOR_LIVE_TEST`: no.
- This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. `lf-beachhead-prepare` stays off `make smoke`, `make gate-90`, and GitHub Actions. `examples/estate.yaml` stays hash-locked.

## This slice — LLaMA-Factory beachhead prepare walk

- `make lf-beachhead-prepare` (`scripts/lf-beachhead-prepare.sh`) reads [`docs/lf-beachhead-matrix.md`](docs/lf-beachhead-matrix.md) and prepares each smoke fixture with `estate enrich prepare` on a throwaway copy of `examples/estate.yaml`. The driver is the row card (`llamafactory-qlora` or `llamafactory-lora`). The pack is the row fixture. `prepare.json` keeps `train_base_model` from that fixture and `seat_tag` from the fixture `model_hint` (`llama3` on every current row). `recipe.yaml` matches the row template and knobs. QLoRA rows keep `lora_rank` 16, `packing` true, `quantization_method` `bnb`, and `quantization_bit` 4. LoRA rows keep `lora_rank` 8, `packing` false, and omit quantization keys. `export.yaml` stays unquantized on both cards. The script prints `SKIP live train`. It does not train, merge, convert, seat, or promote.
- Phi-3-small stays QLoRA-only. The walk does not add it as a matrix row. A throwaway copy of the Phi-3 Instruct pack with train base `microsoft/Phi-3-small-8k-instruct` prepares template `phi_small` on both cards. The QLoRA card still gets the Phi QLoRA reproduce line. The LoRA card does not get the Phi-3 Instruct LoRA line. That id is already on the tip template scan. It is not a new family and not a new matrix row.
- `estate help enrich` and `estate help train` name the walk. `docs/lf-beachhead-matrix.md`, `docs/TRAIN-ENRICH.md`, `docs/operator-enrich-journeys.md`, and `docs/CELL-ONE-STATUS.md` name it. The lock reuses the matrix table as the fixture inventory. `READY_FOR_LIVE_TEST`: no.
- This slice does not add a train family, a fixture pack, a driver, or Kimi. It is not on `make smoke`, `make gate-90`, or GitHub Actions. `examples/estate.yaml` stays hash-locked.

## This slice — Cell One status tip honesty through PR #140

- `docs/CELL-ONE-STATUS.md` names tip `35a88139dea58528e4bc5c7b31708b9716ce091b` (PR #140, the LLaMA-Factory LoRA and QLoRA beachhead matrix). The uniqueness section states the printed ladder (prepare, the `NEXT.md` train and export lines, `merge-adapt`, `gguf-convert`, `local-seat`, `import-trained`). The seat tag stays separate from the Hugging Face train base. A bare Ollama tag is `refuse:train-base`. The section points at [`docs/lf-beachhead-matrix.md`](docs/lf-beachhead-matrix.md): 8 families, each with a LoRA row and a QLoRA row. DeepSeek-R1-Distill chat and GLM-4 Chat pairs are closed. Kimi is not a row. `estate help enrich` and `estate help train` print that file. `make qlora-journey` and `make lora-journey` print `SKIP live train`. `make seat-journey` prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. `gguf-convert` still returns `refuse:tokenizer` for a JSON list or JSON null under `extra_special_tokens`, and for a Qwen-family export missing `vocab.json` or `merges.txt`, and still names the HF-cache restore. `READY_FOR_LIVE_TEST`: no.
- This slice does not add a train family, a fixture pack, a driver, or a journey. It does not add Kimi. It is not on `make smoke`, `make gate-90`, or GitHub Actions. `examples/estate.yaml` stays hash-locked. The recorded 5090 tokenizer prove stays on `docs/local-seat.md`. This slice does not add a live GPU result.

## This slice — LLaMA-Factory LoRA and QLoRA beachhead matrix

- `docs/lf-beachhead-matrix.md` lists each current LLaMA-Factory reproduce target: Phi-3 Instruct, Llama-3.2 Instruct, Gemma-2 Instruct, Mistral Instruct, Qwen2.5 Instruct, Qwen3 Instruct, DeepSeek-R1-Distill chat, and GLM-4 Chat. Each family has a `llamafactory-qlora` row and a `llamafactory-lora` row. The columns are family, card, train-base beachhead id, template, knobs (rank / packing / quant), and the smoke fixture path. QLoRA rows keep `lora_rank` 16, `packing` true, `quantization_method` `bnb`, and `quantization_bit` 4. LoRA rows keep `lora_rank` 8, `packing` false, and omit quantization keys. The smoke seat tag stays `llama3`. A bare Ollama seat tag stays `refuse:train-base`.
- `estate help enrich` and `estate help train` print that file. `docs/TRAIN-ENRICH.md` and `docs/operator-enrich-journeys.md` point at it. The ladder stays the commands that already exist: prepare, the `NEXT.md` train and export lines, `merge-adapt`, `gguf-convert`, `local-seat`, and `import-trained`. This slice does not add a train family, a driver, or a journey. It does not add Kimi. It is not on `make smoke`, `make gate-90`, or GitHub Actions. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — GLM-4 Chat LoRA reproduce target

- `llamafactory-lora` infers LLaMA-Factory template `glm4` for `zai-org/glm-4-9b-chat`, `zai-org/glm-4-9b-chat-1m`, `zai-org/GLM-4-9B-0414`, and `zai-org/GLM-4-32B-0414`, including those ids as nested path segments and HF cache directories (`models--zai-org--glm-4-9b-chat`). That is the same stem-bounded chat group as the GLM-4 Chat QLoRA prepare. `constants.py` registers that group with `template="glm4"`. The DEFAULT DownloadSource for GLM-4-9B-Chat is `zai-org/glm-4-9b-chat`. `template.py` registers `glm4`. There is no `glm_4` template. `examples/train_lora` does not ship a GLM-4 yaml. ChatGLM3 stays `chatglm3`. A GLM-4 base stays `glm4` and is not this reproduce target. GLM-Z1 stays `glmz1`. GLM-4.1V stays `glm4v`. GLM-4.5 stays `glm4_moe`. GLM-4.5V and GLM-4.6V stay `glm4_5v`. GLM-4.7 stays `glm4_7`. GLM-OCR stays `glm_ocr`. Kimi is not in this group. GPTQ, AWQ, and GGUF checkpoints are not this reproduce target. `zai-org/glm-4-9b-chat-hf` is not listed in that group. Qwen2.5 Instruct stays `qwen`. Qwen3 Instruct stays `qwen3_nothink`. DeepSeek-R1-Distill chat stays `deepseekr1`.
- The LoRA card still omits `quantization_bit` and `quantization_method`. `lora_rank` stays 8. `packing` stays false. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the GLM repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as the non-quant twin of the GLM-4 Chat QLoRA prepare only when the winning segment is that GLM-4 Chat shape, and only on `llamafactory-lora`. The QLoRA note stays on `llamafactory-qlora`. That QLoRA note still opens beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, Qwen3 Instruct, and DeepSeek-R1-Distill chat, and keeps `quantization_method` `bnb` and `quantization_bit` 4. The LoRA note names `examples/train_lora`, keeps `lora_rank` 8 and `packing` false, and says the QLoRA note stays on the QLoRA card. Llama-3.2 Instruct, Gemma-2 Instruct, Mistral Instruct, Qwen3 Instruct, Phi-3 Instruct, Qwen2.5 Instruct, and DeepSeek-R1-Distill chat LoRA lines are different train bases. An Ollama tag such as `glm4`, `glm4:9b`, `glm4:latest`, or `glm-4:9b` is a seat tag. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/glm4-chat-lora.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `zai-org/glm-4-9b-chat`). That 9B chat id is the smallest popular chat checkpoint in the official `glm4` group. Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — GLM-4 Chat QLoRA reproduce target

- `llamafactory-qlora` infers LLaMA-Factory template `glm4` for `zai-org/glm-4-9b-chat`, `zai-org/glm-4-9b-chat-1m`, `zai-org/GLM-4-9B-0414`, and `zai-org/GLM-4-32B-0414`, including those ids as nested path segments and HF cache directories (`models--zai-org--glm-4-9b-chat`). `constants.py` registers that group with `template="glm4"`. The DEFAULT DownloadSource for GLM-4-9B-Chat is `zai-org/glm-4-9b-chat`. `template.py` registers `glm4`. There is no `glm_4` template. ChatGLM3 stays `chatglm3`. A GLM-4 base (`zai-org/glm-4-9b`, `zai-org/GLM-4-32B-Base-0414`) stays `glm4` and is not this reproduce target. GLM-Z1 stays `glmz1`. GLM-4.1V stays `glm4v`. GLM-4.5 stays `glm4_moe`. GLM-4.5V and GLM-4.6V stay `glm4_5v`. GLM-4.7 stays `glm4_7`. GLM-OCR stays `glm_ocr`. Kimi is not in this group. GPTQ, AWQ, and GGUF checkpoints are not this reproduce target. `zai-org/glm-4-9b-chat-hf` is not listed in that group. Qwen2.5 Instruct stays `qwen`. Qwen3 Instruct stays `qwen3_nothink`. DeepSeek-R1-Distill chat stays `deepseekr1`.
- The QLoRA card still writes `quantization_method: bnb` and `quantization_bit: 4`. `lora_rank` stays 16. `packing` stays true. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the GLM repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as a reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, Qwen3 Instruct, and DeepSeek-R1-Distill chat only when the winning segment is that GLM-4 Chat shape, and only on `llamafactory-qlora`. `llamafactory-lora` writes the same template, `lora_rank` 8, and `packing` false, and does not get that line. This card does not add the LoRA twin. An Ollama tag such as `glm4`, `glm4:9b`, `glm4:latest`, or `glm-4:9b` is a seat tag. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/glm4-chat.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `zai-org/glm-4-9b-chat`). That 9B chat id is the smallest popular chat checkpoint in the official `glm4` group. Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — DeepSeek-R1-Distill chat LoRA reproduce target

- `llamafactory-lora` infers LLaMA-Factory template `deepseekr1` for `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`, `deepseek-ai/DeepSeek-R1-Distill-Qwen-7B`, `deepseek-ai/DeepSeek-R1-Distill-Llama-8B`, `deepseek-ai/DeepSeek-R1-Distill-Qwen-14B`, `deepseek-ai/DeepSeek-R1-Distill-Qwen-32B`, and `deepseek-ai/DeepSeek-R1-Distill-Llama-70B`, including those ids as nested path segments and HF cache directories (`models--deepseek-ai--DeepSeek-R1-Distill-Qwen-1.5B`). That is the same stem-bounded chat group as the DeepSeek-R1-Distill chat QLoRA prepare. `constants.py` registers that distill group with `template="deepseekr1"`. `template.py` registers `deepseekr1` as a `ReasoningTemplate`. There is no `deepseek_r1` template. `examples/train_lora` does not ship a DeepSeek yaml. A Qwen or Llama substring in those ids stays `deepseekr1`. The Qwen and Llama student checkpoints stay `qwen` or `llama3` and are not this reproduce target. DeepSeek-R1, DeepSeek-R1-Zero, and DeepSeek-R1-0528 stay `deepseekr1` and are not this reproduce target. DeepSeek-V3 stays `deepseek3`. DeepSeek-Coder stays `deepseekcoder`. Qwen2.5 Instruct stays `qwen`. Qwen3 Instruct stays `qwen3_nothink`.
- The LoRA card still omits `quantization_bit` and `quantization_method`. `lora_rank` stays 8. `packing` stays false. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the DeepSeek repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as the non-quant twin of the DeepSeek-R1-Distill chat QLoRA prepare only when the winning segment is that distill shape, and only on `llamafactory-lora`. The QLoRA note stays on `llamafactory-qlora`. That QLoRA note still names `examples/train_qlora` and keeps `quantization_method` `bnb` and `quantization_bit` 4. The LoRA note names `examples/train_lora`, keeps `lora_rank` 8 and `packing` false, and says the QLoRA note stays on the QLoRA card. An Ollama tag such as `deepseek-r1` or `deepseek-r1:1.5b` is a seat tag. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/deepseek-r1-distill-lora.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`). That 1.5B chat id is the smallest checkpoint in the official R1-Distill group. Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — DeepSeek-R1-Distill chat QLoRA reproduce target

- `llamafactory-qlora` infers LLaMA-Factory template `deepseekr1` for `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`, `deepseek-ai/DeepSeek-R1-Distill-Qwen-7B`, `deepseek-ai/DeepSeek-R1-Distill-Llama-8B`, `deepseek-ai/DeepSeek-R1-Distill-Qwen-14B`, `deepseek-ai/DeepSeek-R1-Distill-Qwen-32B`, and `deepseek-ai/DeepSeek-R1-Distill-Llama-70B`, including those ids as nested path segments and HF cache directories (`models--deepseek-ai--DeepSeek-R1-Distill-Qwen-1.5B`). `constants.py` registers that distill group with `template="deepseekr1"`. `template.py` registers `deepseekr1` as a `ReasoningTemplate`. There is no `deepseek_r1` template. A Qwen or Llama substring in those ids stays `deepseekr1`. The Qwen and Llama student checkpoints stay `qwen` or `llama3` and are not this reproduce target. DeepSeek-R1, DeepSeek-R1-Zero, and DeepSeek-R1-0528 stay `deepseekr1` and are not this reproduce target. DeepSeek-V3 stays `deepseek3`. DeepSeek-Coder stays `deepseekcoder`. Qwen2.5 Instruct stays `qwen`. Qwen3 Instruct stays `qwen3_nothink`.
- The QLoRA card still writes `quantization_method: bnb` and `quantization_bit: 4`. `lora_rank` stays 16. `packing` stays true. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the DeepSeek repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as a reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, Qwen2.5 Instruct, and Qwen3 Instruct only when the winning segment is that distill shape, and only on `llamafactory-qlora`. `llamafactory-lora` writes the same template and does not get that line. An Ollama tag such as `deepseek-r1` or `deepseek-r1:1.5b` is a seat tag. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/deepseek-r1-distill.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `deepseek-ai/DeepSeek-R1-Distill-Qwen-1.5B`). That 1.5B chat id is the smallest checkpoint in the official R1-Distill group. Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — name import-trained after a GGUF local-seat

- After a GGUF `estate enrich local-seat` print, the standing next step is `estate enrich import-trained` for that file. `trained_shape` is `gguf`. The proposal stays `auto_apply=false`. The same step stands when `ollama create` already ran outside this factory. The report says this factory did not run `ollama create` and that `import-trained` does not apply the estate. `gguf-convert`, LLaMA-Factory and Axolotl `NEXT.md` / `PREPARE.md`, `estate help enrich`, and the operator pages name that step. A fused MLX directory stays `refuse:adapter`.
- `make seat-journey` asserts that print on the GGUF stub and still records `trained_shape` `gguf`. It stays off `make smoke`, `make gate-90`, and GitHub Actions. `examples/estate.yaml` stays hash-locked. The commands do not spawn `ollama`, do not apply, and do not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — name the HF-cache tokenizer restore after refuse:tokenizer

- After `refuse:tokenizer`, `NEXT.md`, `PREPARE.md`, the LLaMA-Factory `merge-adapt` report, and `estate help enrich` name the operator restore. Copy tokenizer files from the HF cache snapshot for the train base already on disk, or the equivalent base checkout, into the export directory. Keep the export `tokenizer_config.json` as `tokenizer_config.json.bak`. Then re-run `estate enrich gguf-convert` on that export directory. JSON list, JSON null, and a Qwen-family export missing `vocab.json` or `merges.txt` stay the same refuse. The command does not download weights, does not copy tokenizer files, does not write `tokenizer_config.json.bak`, and does not print `python3 convert_hf_to_gguf.py` on that refuse. `make seat-journey` still asserts the refuse on the 5090-shaped export and stays off `make smoke`, `make gate-90`, and GitHub Actions. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — name JSON null in tokenizer guidance

- `export_tokenizer_guidance` (the standing note on LLaMA-Factory `NEXT.md`, `PREPARE.md`, and the `merge-adapt` report) and `tokenizer_pass_note` name JSON null under `extra_special_tokens` as `refuse:tokenizer` beside a JSON list. JSON null is a non-object: transformers calls `.keys()` on that value. The list case still names `AttributeError: 'list' object has no attribute 'keys'`. `estate help enrich` names the same case. `docs/TRAIN-ENRICH.md`, `docs/local-seat.md`, `docs/operator-enrich-journeys.md`, and `docs/CELL-ONE-STATUS.md` name JSON null on the refuse rule beside the list. The 5090-shaped fixture stays a JSON list. The refuse check is unchanged: JSON null already returns `refuse:tokenizer`. An absent key and a JSON object still pass. The commands stay print-only. They do not download weights, do not copy tokenizer files, do not write `tokenizer_config.json.bak`, and do not spawn `convert_hf_to_gguf.py`. `READY_FOR_LIVE_TEST`: no.

## This slice — refuse JSON null extra_special_tokens

- `estate enrich gguf-convert` returns `refuse:tokenizer` when `extra_special_tokens` in `tokenizer_config.json` under `--weights` is JSON `null`. That value is a non-object, the same refuse as a list, a string, a bool, or a number. An absent key and a JSON object still pass this check. The command does not print `python3 convert_hf_to_gguf.py`, does not download weights, does not copy tokenizer files, and does not write `tokenizer_config.json.bak`. `READY_FOR_LIVE_TEST`: no.

## This slice — Qwen2.5 Instruct LoRA reproduce target

- `llamafactory-lora` infers LLaMA-Factory template `qwen` for `Qwen/Qwen2.5-0.5B-Instruct`, `Qwen/Qwen2.5-1.5B-Instruct`, `Qwen/Qwen2.5-3B-Instruct`, `Qwen/Qwen2.5-7B-Instruct`, `Qwen/Qwen2.5-14B-Instruct`, `Qwen/Qwen2.5-32B-Instruct`, `Qwen/Qwen2.5-72B-Instruct`, `Qwen/Qwen2.5-7B-Instruct-1M`, and `Qwen/Qwen2.5-14B-Instruct-1M`, including those ids as nested path segments and HF cache directories (`models--Qwen--Qwen2.5-0.5B-Instruct`). That is the same stem-bounded text group as the Qwen2.5 Instruct QLoRA prepare. `constants.py` registers that text group with `template="qwen"`. `template.py` registers `qwen`. There is no `qwen2_5` template. A Qwen2.5 base uses `qwen` and is not this reproduce target. A name that contains `thinking` is not this reproduce target. Qwen2 Instruct stays `qwen` and is not this reproduce target. Qwen2.5-Coder and Qwen2.5-Math stay `qwen` and are not this reproduce target. Qwen2.5-VL is the `qwen2_vl` group and is not this reproduce target. GPTQ and AWQ checkpoints are not this reproduce target. Qwen3 Instruct stays `qwen3_nothink`.
- The LoRA card still omits `quantization_bit` and `quantization_method`. `lora_rank` stays 8. `packing` stays false. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Qwen repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as the non-quant twin of the Qwen2.5 Instruct QLoRA prepare only when the winning segment is that Qwen2.5 Instruct shape, and only on `llamafactory-lora`. The QLoRA note stays on `llamafactory-qlora`. A Qwen2.5 base, a thinking-shaped id, Qwen2, Qwen2.5-Coder, and Qwen2.5-VL do not get the LoRA line. A Qwen3 Instruct id keeps `qwen3_nothink` and does not get that line. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/qwen25-instruct-lora.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `Qwen/Qwen2.5-0.5B-Instruct`). That 0.5B Instruct id is the 5090 smoke train base. Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — seat-journey exercises refuse:tokenizer

- `make seat-journey` writes a 5090-shaped merged export before the good stubs. `config.json` sets `model_type` to `qwen2` and `architectures` to `Qwen2ForCausalLM`. `tokenizer_config.json` sets `extra_special_tokens` to a JSON list. `vocab.json` and `merges.txt` are absent. `model.safetensors` is the merged-weight marker. `estate enrich gguf-convert` on that directory returns `refuse:tokenizer`, names the list and both missing BPE files, and does not print `python3 convert_hf_to_gguf.py`. The script does not download weights, does not copy tokenizer files, and does not write `tokenizer_config.json.bak`.
- The script then removes that fixture and writes the existing good stubs (`export/config.json` is `{}`, plus `export/model.safetensors`). `gguf-convert` prints the convert line. `local-seat` and `import-trained` continue as before. `CELL_SEAT_LIVE` stays print-only. Not in `make smoke`, `make gate-90`, or GitHub Actions. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — refuse a broken LLaMA-Factory export tokenizer

- A live 5090 Target C prove on 2026-09-23 ran `llamafactory-cli export` for `Qwen/Qwen2.5-0.5B-Instruct`, then `python3 convert_hf_to_gguf.py <prepared>/export --outfile <prepared>/export.gguf --outtype auto`. The convert failed. `tokenizer_config.json` had `extra_special_tokens` as a list, and transformers raised `AttributeError: 'list' object has no attribute 'keys'`. The export also omitted `vocab.json` and `merges.txt`, which that train-base tokenizer includes. Restoring the tokenizer files from the HF cache snapshot already on disk, and keeping the export `tokenizer_config.json` as `tokenizer_config.json.bak`, let the convert write a 949M BF16 GGUF. The same export shape is [LLaMA-Factory issue 10169](https://github.com/hiyouga/LlamaFactory/issues/10169). This factory still does not run convert, does not download weights, and does not copy those files.
- `estate enrich gguf-convert` returns `refuse:tokenizer` when `tokenizer_config.json` under `--weights` has list-shaped `extra_special_tokens` (or another non-object), or when that directory is a Qwen-family export and `vocab.json` or `merges.txt` is missing. Qwen-family is `config.json` `model_type` or `architectures`, or `tokenizer_class`, naming Qwen. A symlinked `tokenizer_config.json` is the same refuse. An object `extra_special_tokens` with those two files present still prints `python3 convert_hf_to_gguf.py` with `--outtype auto`. The check reads files already in the export directory.
- `NEXT.md`, `PREPARE.md`, and the `merge-adapt` report on `llamafactory-lora` and `llamafactory-qlora` name that restore step. `docs/TRAIN-ENRICH.md`, `docs/local-seat.md`, and `docs/operator-enrich-journeys.md` name it too.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Target C seat ladder (print-only fixture stubs)

- `make seat-journey` prepares `llamafactory-qlora` (seat tag `llama3`, train base `Qwen/Qwen2.5-0.5B-Instruct`) on a throwaway copy of `examples/estate.yaml`, then prints `merge-adapt`, `gguf-convert`, `local-seat`, and `import-trained` once fixture stubs exist. The adapter stub is `outputs/adapter_config.json`. The merged stub is `export/config.json` plus `export/model.safetensors` (a name that does not start with `adapter_model`). The GGUF stub is `export.gguf` and starts with GGUF magic. `local-seat` prints `ollama create` plus `llama-cli -m` and `llama-server -m`. `import-trained` records `trained_shape` `gguf`. A missing adapter is `refuse:adapter`. A missing export or GGUF is `refuse:seat`.
- The script prints `SKIP live train`, `SKIP live convert`, and `SKIP live seat`. It does not run `llamafactory-cli`, `convert_hf_to_gguf.py`, or `ollama create`. `CELL_SEAT_LIVE=1` does not start those programs. Live train, merge, convert, and seat stay on the operator host (section 8 of the journeys page). `make enrich-live-prove` still covers a from-pack Modelfile.
- The walk is section 10 of `docs/operator-enrich-journeys.md`. `docs/TRAIN-ENRICH.md` and `estate help enrich` / `estate help train` name the same check.
- Not in `make smoke`, `make gate-90`, or GitHub Actions. `examples/estate.yaml` stays hash-locked. `READY_FOR_LIVE_TEST`: no.

## This slice — Qwen2.5 Instruct QLoRA reproduce target

- `llamafactory-qlora` infers LLaMA-Factory template `qwen` for `Qwen/Qwen2.5-0.5B-Instruct`, `Qwen/Qwen2.5-1.5B-Instruct`, `Qwen/Qwen2.5-3B-Instruct`, `Qwen/Qwen2.5-7B-Instruct`, `Qwen/Qwen2.5-14B-Instruct`, `Qwen/Qwen2.5-32B-Instruct`, `Qwen/Qwen2.5-72B-Instruct`, `Qwen/Qwen2.5-7B-Instruct-1M`, and `Qwen/Qwen2.5-14B-Instruct-1M`, including those ids as nested path segments and HF cache directories (`models--Qwen--Qwen2.5-0.5B-Instruct`). `constants.py` registers that text group with `template="qwen"`. `template.py` registers `qwen`. There is no `qwen2_5` template. A Qwen2.5 base uses `qwen` and is not this reproduce target. A name that contains `thinking` is not this reproduce target. Qwen2 Instruct stays `qwen` and is not this reproduce target. Qwen2.5-Coder and Qwen2.5-Math stay `qwen` and are not this reproduce target. Qwen2.5-VL is the `qwen2_vl` group and is not this reproduce target. GPTQ and AWQ checkpoints are not this reproduce target. Qwen3 Instruct stays `qwen3_nothink`.
- The QLoRA card still writes `quantization_method: bnb` and `quantization_bit: 4`. `lora_rank` stays 16. `packing` stays true. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Qwen repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as a reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen3 Instruct only when the winning segment is that Qwen2.5 Instruct shape, and only on `llamafactory-qlora`. A Qwen2.5 base, a thinking-shaped id, Qwen2, Qwen2.5-Coder, and Qwen2.5-VL do not get that line. A Qwen3 Instruct id keeps `qwen3_nothink` and does not get that line. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/qwen25-instruct.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `Qwen/Qwen2.5-0.5B-Instruct`). That 0.5B Instruct id is the 5090 smoke train base. Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — Target A Qwen LoRA operator journey

- The unquantized path is one ladder of commands that already exist: `estate enrich prepare --driver llamafactory-lora` (seat tag separate from the train base), the `NEXT.md` `llamafactory-cli train` and `llamafactory-cli export` lines, `estate enrich merge-adapt` (prints `llamafactory-cli export`), `estate enrich gguf-convert` (prints `python3 convert_hf_to_gguf.py` with `--outtype auto`), `estate enrich local-seat` (prints `ollama create`), and `estate enrich import-trained` (records `trained_shape` and `trained_paths`). A 5090 smoke seated `llama3` and trained `Qwen/Qwen2.5-0.5B-Instruct` on this card, then exported and seated that gauge from the files prepare wrote. `template` is `qwen`. The recipe omits `quantization_bit` and `quantization_method`. `lora_rank` is 8. `packing` is false. This path does not require bitsandbytes. A missing train base is `refuse:train-base`.
- The walk is section 9 of `docs/operator-enrich-journeys.md`. `docs/TRAIN-ENRICH.md` and `estate help enrich` / `estate help train` print the same ladder.
- `make lora-journey` prints that ladder and checks the prepare artifacts on a throwaway estate copy. A missing export is `refuse:seat` and writes no GGUF. The script does not run LLaMA-Factory, llama.cpp, or Ollama, and it does not promote. It is not in `make smoke`, `make gate-90`, or GitHub Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Phi-3 Instruct LoRA reproduce target

- `llamafactory-lora` infers LLaMA-Factory template `phi` for `microsoft/Phi-3-mini*`, `microsoft/Phi-3-medium*`, and Phi-3.5 (`microsoft/Phi-3.5-mini-instruct`, `microsoft/Phi-3.5-MoE-instruct`), including those ids as nested path segments and HF cache directories (`models--microsoft--Phi-3-mini-4k-instruct`). That is the same text group as the Phi-3 Instruct QLoRA prepare. Phi-3-small stays `phi_small`. Phi-4 stays `phi4`. Phi-4-mini stays `phi4_mini`. The names follow LLaMA-Factory `register_model_group` in `constants.py`.
- The LoRA card still omits `quantization_bit` and `quantization_method`. `lora_rank` stays 8. `packing` stays false. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Phi repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as the non-quant twin of the Phi-3 Instruct QLoRA prepare only when the winning segment is that Instruct shape, and only on `llamafactory-lora`. The QLoRA note stays on `llamafactory-qlora`, including Phi-3-small. Phi-3-small, Phi-4, and Phi-4-mini do not get the LoRA line. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/phi3-instruct-lora.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `microsoft/Phi-3-mini-4k-instruct`). Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — Mistral Instruct LoRA reproduce target

- `llamafactory-lora` infers LLaMA-Factory template `mistral` for `mistralai/Mistral-7B-Instruct-v0.1`, `mistralai/Mistral-7B-Instruct-v0.2`, and `mistralai/Mistral-7B-Instruct-v0.3`, including those ids as nested path segments and HF cache directories (`models--mistralai--Mistral-7B-Instruct-v0.3`). That is the same text group as the Mistral Instruct QLoRA prepare. `constants.py` registers that Mistral-7B group with `template="mistral"`. `template.py` registers `mistral`. There is no `mistral_7` template. A Mistral-7B base uses `mistral` and is not this reproduce target. Mistral-Small uses `mistral_small`. Mistral-Nemo uses `ministral`. Mixtral uses `mistral` and is not this reproduce target. LLaVA-NeXT-Mistral uses `llava_next_mistral`. Ministral, Ministral-3, Codestral, Devstral, and Pixtral stay off `mistral`.
- The LoRA card still omits `quantization_bit` and `quantization_method`. `lora_rank` stays 8. `packing` stays false. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Mistral repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as the non-quant twin of the Mistral Instruct QLoRA prepare only when the winning segment is that Instruct shape, and only on `llamafactory-lora`. The QLoRA note stays on `llamafactory-qlora`. A Mistral-7B base, Mistral-Small, Mistral-Nemo, Mixtral, and LLaVA-NeXT-Mistral do not get the LoRA line. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/mistral-instruct-lora.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `mistralai/Mistral-7B-Instruct-v0.3`). Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — Gemma-2 Instruct LoRA reproduce target

- `llamafactory-lora` infers LLaMA-Factory template `gemma2` for `google/gemma-2-2b-it`, `google/gemma-2-9b-it`, and `google/gemma-2-27b-it`, including those ids as nested path segments and HF cache directories (`models--google--gemma-2-2b-it`). That is the same text group as the Gemma-2 Instruct QLoRA prepare. `constants.py` registers that group with `template="gemma2"`. `template.py` registers `gemma2`. There is no `gemma_2` template. `examples/train_lora` does not ship a Gemma-2 yaml. A Gemma-2 base uses `gemma2` and is not this reproduce target. Original Gemma (`gemma-2b`, `gemma-7b`) stays `gemma`. Gemma-3 stays off `gemma2`.
- The LoRA card still omits `quantization_bit` and `quantization_method` and keeps `lora_rank` 8. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Gemma repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as the non-quant twin of the Gemma-2 Instruct QLoRA prepare only when the winning segment is that Instruct shape, and only on `llamafactory-lora`. The QLoRA note stays on `llamafactory-qlora`. A Gemma-2 base, original Gemma, and Gemma-3 do not get the LoRA line. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/gemma2-instruct-lora.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `google/gemma-2-2b-it`). That 2B Instruct id is the smallest checkpoint in the Gemma-2 Instruct group. Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — Llama-3.2 Instruct LoRA reproduce target

- `llamafactory-lora` infers LLaMA-Factory template `llama3` for `meta-llama/Llama-3.2-1B-Instruct` and `meta-llama/Llama-3.2-3B-Instruct`, including those ids as nested path segments and HF cache directories (`models--meta-llama--Llama-3.2-3B-Instruct`). That is the same text group as the Llama-3.2 Instruct QLoRA prepare. There is no `llama3_2` template. Llama-3.2 vision uses `mllama`. `llama3-llava-next` uses `llava_next_llama3`. `llama-30b` stays `default`. A Llama-3.2 base, Llama-3.1 Instruct, and Llama-3.3 Instruct stay `llama3` and are not this reproduce target.
- The LoRA card still omits `quantization_bit` and `quantization_method`. `lora_rank` stays 8. `packing` stays false. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Llama repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as the non-quant twin of the Llama-3.2 Instruct QLoRA prepare only when the winning segment is that Instruct shape, and only on `llamafactory-lora`. The QLoRA note stays on `llamafactory-qlora`. A Llama-3.2 vision id, a Llama-3.2 base, and a Llama-3.1 Instruct id do not get the LoRA line. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/llama32-instruct-lora.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `meta-llama/Llama-3.2-3B-Instruct`). Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — refuse fused MLX on import-trained

- `estate enrich import-trained` on an `mlx-lm-lora` train prepare refuses a fused MLX directory (`refuse:adapter`). That directory is `config.json` and a `.safetensors` file whose name does not start with `adapter_model`. `mlx_lm.fuse` writes `model.safetensors` and `config.json`. A directory that also holds `ggml-model-f16.gguf` is the same refuse. The command does not record `trained_shape` `merged` and does not write a proposal.
- The same command still records an adapter directory (`adapter_config.json`) or a GGUF path (one `.gguf` file, or a directory with exactly one top-level `.gguf`). Pass the file when the fused directory also holds the weights. `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, `axolotl-qlora`, and `unsloth-qlora` still record a merged Hugging Face directory.
- `MLX.md`, `PREPARE.md`, and `NEXT.md` name the adapter directory and the GGUF file. They do not tell the operator to point `import-trained` at the fused directory. `local-seat` and `gguf-convert` still refuse that directory (`refuse:seat`). `READY_FOR_LIVE_TEST`: no.

## This slice — Qwen3 Instruct LoRA reproduce target

- `llamafactory-lora` infers LLaMA-Factory template `qwen3_nothink` for `Qwen/Qwen3-4B-Instruct-2507`, `Qwen/Qwen3-30B-A3B-Instruct-2507`, `Qwen/Qwen3-235B-A22B-Instruct-2507`, and `Qwen/Qwen3-Next-80B-A3B-Instruct`, including those ids as nested path segments and HF cache directories (`models--Qwen--Qwen3-4B-Instruct-2507`). That is the same text group as the Qwen3 Instruct QLoRA prepare. `examples/train_lora/qwen3_lora_sft.yaml` sets `model_name_or_path` to `Qwen/Qwen3-4B-Instruct-2507` and `template: qwen3_nothink`, with `lora_rank: 8` and no quantization keys. A Qwen3 thinking or base checkpoint (`Qwen/Qwen3-4B`, `Qwen/Qwen3-4B-Thinking-2507`, `Qwen/Qwen3-4B-Base`, `Qwen/Qwen3-Next-80B-A3B-Thinking`) uses `qwen3`. Qwen2 and Qwen2.5, including `Qwen/Qwen2.5-0.5B-Instruct`, stay `qwen`.
- The LoRA card still omits `quantization_bit` and `quantization_method`. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Qwen repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as the non-quant twin of the Qwen3 Instruct QLoRA prepare only when the winning segment is that Instruct shape, and only on `llamafactory-lora`. The QLoRA note stays on `llamafactory-qlora`. A Qwen3 thinking id and a Qwen2.5 Instruct id do not get the LoRA line. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/qwen3-instruct-lora.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `Qwen/Qwen3-4B-Instruct-2507`). That 4B Instruct id is the smallest checkpoint in the `qwen3_nothink` group and the id in the official LoRA yaml. Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — print the LLaMA-Factory export ladder

- `estate enrich merge-adapt` on a `llamafactory-lora` or `llamafactory-qlora` train prepare prints `llamafactory-cli export` for that prepare's `export.yaml`. The report lists the keys from `examples/merge_lora/qwen3_lora_sft.yaml` (`model_name_or_path`, `adapter_name_or_path`, `template`, `trust_remote_code`, `export_dir`, `export_size`, `export_device`, `export_legacy_format`) with the values that file sets. The README command is `llamafactory-cli export examples/merge_lora/qwen3_lora_sft.yaml` (https://llamafactory.readthedocs.io/en/latest/getting_started/merge_lora.html and https://github.com/hiyouga/LlamaFactory/blob/main/examples/README.md). LLaMA-Factory writes `export_dir`. The report then prints `gguf-convert` and `local-seat` for that directory, including `python3 convert_hf_to_gguf.py` with `--outtype auto`. The command does not merge, does not shell out, does not write the directory, and does not promote.
- The merge example says not to use a quantized model or `quantization_bit` when merging LoRA adapters. A real key `quantization_bit`, `quantization_method`, `export_quantization_bit`, or `export_quantization_dataset` is `refuse:export`. A comment that names those words is not a key. `llamafactory-qlora` still trains with `quantization_bit: 4`. The export card stays unquantized. `adapter_name_or_path` must be the `--adapter` directory. An early stop still means point that key at `checkpoint-<step>` and pass that directory. This factory does not rewrite `export.yaml`.
- A missing `export.yaml`, a missing or mismatched train base, a template that disagrees with `recipe.yaml`, a missing documented key, `export_device` outside `cpu` and `auto`, a symlink, a sacred token, a hardware SKU, the wrong job, and a promoted prepare still refuse. `READY_FOR_LIVE_TEST`: no.
- `PREPARE.md` and `NEXT.md` on both LLaMA-Factory cards name this print before `gguf-convert` and `local-seat`. Axolotl, mlx-lm, and Unsloth ladders are unchanged.

## This slice — print the Unsloth save ladder

- `estate enrich merge-adapt` on an `unsloth-qlora` train prepare prints Unsloth's documented `model.save_pretrained_merged(..., save_method = "merged_16bit")` when `--adapter` holds `adapter_config.json` and `adapter_model.safetensors` (or `adapter_model.bin`). The directory argument is `merged` beside the prepare. The vLLM guide and the saving-to-gguf page publish that call (https://unsloth.ai/docs/basics/inference-and-deployment/vllm-guide, https://unsloth.ai/docs/basics/inference-and-deployment/saving-to-gguf). The command does not merge, does not shell out, does not write a Python file, and does not promote.
- The same report prints Unsloth's LoRA save lines, the inference-page reload with `model_name` set to the adapter directory, the three manual `python llama.cpp/convert_hf_to_gguf.py` lines (`f16`, `bf16`, `q8_0`, `--split-max-size 50G`), and the three `model.save_pretrained_gguf` examples (`q4_k_m`, `q8_0`, `f16`, directory string `directory`). It then names `gguf-convert` and `local-seat` for that merged directory. `gguf-convert` still prints `python3 convert_hf_to_gguf.py` with `--outtype auto`, the llama.cpp script default. Unsloth's page does not publish `--outtype auto`.
- `local-seat --weights` accepts the merged Hugging Face directory or a GGUF file. `local-seat --adapter` is `refuse:adapter`. Unsloth's Ollama page seats a GGUF. It does not publish an Ollama adapter line for the PEFT directory. `--weights` on that adapter directory is `refuse:seat` and does not point the operator at `--adapter`.
- A missing train base, a missing or mismatched `UNSLOTH.md`, a symlinked `UNSLOTH.md`, a missing weight file, `adapters.safetensors` alone, a symlink, a sacred token, a hardware SKU, the wrong job, and a promoted prepare still refuse. Status stays `optional`. The card does not write a recipe.
- `PREPARE.md`, `NEXT.md`, and `UNSLOTH.md` name this print ladder after the operator-owned train step.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Qwen3 Instruct QLoRA reproduce target

- `llamafactory-qlora` infers LLaMA-Factory template `qwen3_nothink` for `Qwen/Qwen3-4B-Instruct-2507`, `Qwen/Qwen3-30B-A3B-Instruct-2507`, `Qwen/Qwen3-235B-A22B-Instruct-2507`, and `Qwen/Qwen3-Next-80B-A3B-Instruct`, including those ids as nested path segments and HF cache directories (`models--Qwen--Qwen3-4B-Instruct-2507`). That is the `template="qwen3_nothink"` group in `constants.py`. `examples/train_lora/qwen3_lora_sft.yaml` and `examples/train_qlora/qwen3_lora_sft_otfq.yaml` use `Qwen/Qwen3-4B-Instruct-2507` and `template: qwen3_nothink`. `template.py` registers `qwen3_nothink`. A Qwen3 thinking or base checkpoint (`Qwen/Qwen3-4B`, `Qwen/Qwen3-4B-Thinking-2507`, `Qwen/Qwen3-4B-Base`, `Qwen/Qwen3-Next-80B-A3B-Thinking`) uses `qwen3`. Qwen2 and Qwen2.5, including `Qwen/Qwen2.5-0.5B-Instruct`, stay `qwen`.
- The QLoRA card still writes `quantization_method: bnb` and `quantization_bit: 4`. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Qwen repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as a reproduce target beside Phi-3, Llama-3.2, Gemma-2, Mistral, and Qwen2.x LoRA/QLoRA only when the winning segment is that Qwen3 Instruct shape. A Qwen3 thinking id and a Qwen2.5 Instruct id do not get that line. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/qwen3-instruct.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `Qwen/Qwen3-4B-Instruct-2507`). That 4B Instruct id is the smallest checkpoint in the `qwen3_nothink` group and the id in the official yaml. Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — print the mlx-lm fuse ladder

- `estate enrich merge-adapt` on an `mlx-lm-lora` train prepare prints the documented `mlx_lm.fuse` line when `--adapter` is the directory `mlx_lm.lora` writes (`adapter_config.json` and `adapters.safetensors`). `--model` is `prepare.json` `train_base_model`. `--adapter-path` is that directory. `--save-path` is `fused_model` beside the prepare (the `mlx_lm.fuse` default directory name). The same report prints `mlx_lm.fuse --export-gguf`. mlx-lm writes `ggml-model-f16.gguf` inside that directory (https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md). LORA.md limits that GGUF export to Mistral, Mixtral, and Llama style models in fp16. The report then names `estate enrich local-seat` for that file. The command does not fuse, does not shell out, does not write weights, and does not promote.
- The fused directory is MLX weights. `gguf-convert` and `local-seat` do not treat it as a merged Hugging Face directory and do not print a llama.cpp convert line for it. A `.gguf` file on this prepare still prints the Ollama seat (`local-seat`) and is `refuse:seat` on `gguf-convert` because the file is already GGUF. `local-seat --adapter` stays `refuse:adapter` for this driver. `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, and `axolotl-qlora` keep their merge, convert, and seat lines. `unsloth-qlora` stays `refuse:driver`.
- A missing `adapters.safetensors`, a PEFT `adapter_model` file, a symlink, a sacred token, a hardware SKU, the wrong job, and a host other than `apple-silicon` still refuse. A non-apple-silicon prepare still writes nothing (`refuse:host`).
- `PREPARE.md`, `NEXT.md`, and `MLX.md` name this print ladder after the handoff train step.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Mistral Instruct QLoRA reproduce target

- `llamafactory-qlora` infers LLaMA-Factory template `mistral` for `mistralai/Mistral-7B-Instruct-v0.1`, `mistralai/Mistral-7B-Instruct-v0.2`, and `mistralai/Mistral-7B-Instruct-v0.3`, including those ids as nested path segments and HF cache directories (`models--mistralai--Mistral-7B-Instruct-v0.3`). A Mistral-7B base uses that same template. Mistral-Small uses `mistral_small`. Mistral-Nemo uses `ministral`. Mixtral uses `mistral` and is not this reproduce target. LLaVA-NeXT-Mistral uses `llava_next_mistral`. Ministral, Ministral-3, Codestral, Devstral, and Pixtral stay off `mistral`. A short `mistral` stem does not take those other names. The names follow LLaMA-Factory `register_model_group` in `constants.py` and `mistral` in `template.py`. There is no `mistral_7` template.
- The QLoRA card still writes `quantization_method: bnb` and `quantization_bit: 4`. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Mistral repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as a reproduce target beside Phi-3, Llama-3.2, Gemma-2, and Qwen LoRA/QLoRA only when the winning segment is Mistral-7B Instruct. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/mistral-instruct.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `mistralai/Mistral-7B-Instruct-v0.3`). Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — print the adapter merge into a Hugging Face directory

- `estate enrich merge-adapt --prepared <dir> --adapter <adapter-dir>` prints the documented external merge for an `axolotl-lora`, `axolotl-qlora`, `llamafactory-lora`, or `llamafactory-qlora` train prepare. `--adapter` is a directory with `adapter_config.json`.
- Axolotl's line is `axolotl merge-lora <axolotl.yml> --lora-model-dir=<adapter>` (https://docs.axolotl.ai/docs/getting-started.html section 4.4 and https://docs.axolotl.ai/docs/cli.html). Axolotl writes `{output_dir}/merged`. This prepare sets `output_dir` to `outputs`, so the directory is `outputs/merged`. `axolotl merge-lora` does not take `--out`. `axolotl-qlora` also prints that line with `--dequant`, the CLI flag that writes a bf16 checkpoint. Both lines write that same directory.
- When a LLaMA-Factory prepare has no `export.yaml`, the print is the PEFT `merge_and_unload` snippet and `save_pretrained` to `{prepared}/merged`, beside `outputs/`. When `export.yaml` is present, the report also names `llamafactory-cli export` and that file's `export_dir`. The PEFT snippet stays the printed merge for the adapter argument.
- The report then prints `estate enrich gguf-convert` and `estate enrich local-seat` with `--weights` pointing at that merged directory, and the `python3 convert_hf_to_gguf.py` line with `--outtype auto`. The command does not merge, does not shell out, does not write the directory, and does not promote.
- A merged Hugging Face directory or a GGUF passed as `--adapter` is `refuse:adapter`. A missing `adapter_config.json`, a symlink, a sacred token, a hardware SKU, the wrong driver, and the wrong job refuse on the same paths `local-seat` and `gguf-convert` already use. `unsloth-qlora` and `mlx-lm-lora` stay `refuse:driver`.
- `PREPARE.md` and `NEXT.md` on the Axolotl cards name `merge-adapt` after `axolotl train`.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Gemma-2 Instruct QLoRA reproduce target

- `llamafactory-qlora` infers LLaMA-Factory template `gemma2` for `google/gemma-2-2b-it`, `google/gemma-2-9b-it`, and `google/gemma-2-27b-it`, including those ids as nested path segments and HF cache directories (`models--google--gemma-2-2b-it`). A Gemma-2 base uses that same template. Original Gemma (`gemma-2b`, `gemma-7b`) stays `gemma`. Gemma-3 stays off `gemma2`. A short `gemma` stem does not take the Gemma-2 names. The name follows LLaMA-Factory `register_model_group` in `constants.py` and `gemma2` in `template.py`. There is no `gemma_2` template.
- The QLoRA card still writes `quantization_method: bnb` and `quantization_bit: 4`. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Gemma repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as a reproduce target beside Phi-3, Llama-3.2, and Qwen LoRA/QLoRA only when the winning segment is Gemma-2 Instruct. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/gemma2-instruct.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `google/gemma-2-2b-it`). Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — Llama-3.2 Instruct QLoRA reproduce target

- `llamafactory-qlora` infers LLaMA-Factory template `llama3` for `meta-llama/Llama-3.2-1B-Instruct` and `meta-llama/Llama-3.2-3B-Instruct`, including those ids as nested path segments and HF cache directories (`models--meta-llama--Llama-3.2-3B-Instruct`). Llama-3.2 vision infers `mllama`. `llama3-llava-next` infers `llava_next_llama3`. `llama-30b` stays `default`. A short `llama-3` stem does not take those names. The names follow LLaMA-Factory `register_model_group` in `constants.py`. There is no `llama3_2` template.
- The QLoRA card still writes `quantization_method: bnb` and `quantization_bit: 4`. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Llama repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as a reproduce target beside Phi-3 and Qwen LoRA/QLoRA. A bare Ollama tag stays `refuse:train-base`.
- `examples/fixtures/llama32-instruct.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `meta-llama/Llama-3.2-3B-Instruct`). Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — print a llama.cpp seat for an existing GGUF

- `estate enrich local-seat` on a GGUF file, or a directory that holds one `.gguf` file, still prints the Ollama `ollama create` line. It also prints the documented llama.cpp lines `llama-cli -m <file>` and `llama-server -m <file> --port 8080`. `-m` names that GGUF. `--port 8080` is the llama-server example port. `--runtime llama.cpp` prints those lines first and still prints the Ollama line. Ollama stays the default print. The command does not run either program, does not write a file, and does not promote.
- A merged Hugging Face directory still points at `gguf-convert` (`python3 convert_hf_to_gguf.py` with `--outtype auto`). The report does not print `llama-cli` or `llama-server` for that directory. `--runtime llama.cpp` on the directory says the same: convert first, then seat the sibling GGUF.
- `--adapter` stays the Ollama `ADAPTER` print. llama.cpp does not load an `adapter_config.json` directory in one line, so `--runtime llama.cpp` with `--adapter` is `refuse:runtime` after the shape checks. A symlink, a bad shape, a sacred token, a hardware SKU, the wrong driver, and the wrong job still refuse on those existing paths. Another runtime name is `refuse:runtime`.
- `READY_FOR_LIVE_TEST`: no.

## This slice — print the no-merge adapter seat

- `estate enrich local-seat --adapter <output_dir>` prints a Modelfile for an adapter directory (`adapter_config.json`, the same marker `import-trained` accepts for `trained_shape=adapter`, plus adapter weights when the train wrote them). `FROM` is `prepare.json` `seat_tag` (the same string as `base_model`). `ADAPTER` is that directory. The report then prints `ollama create cell-enrich-{pack} -f <dir>/Modelfile`. The command does not write the file, does not run the line, does not merge, and does not promote.
- `--weights` stays the merged export or GGUF path and still refuses an adapter directory (`refuse:seat`). A merged export or a GGUF passed to `--adapter` is `refuse:adapter`. A missing `adapter_config.json` is `refuse:adapter`. A symlinked adapter path or a symlinked marker is refused with `O_NOFOLLOW`, the same way `local-seat` and `import-trained` already refuse symlinks.
- `PREPARE.md` and `NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` name this print beside the merged and GGUF seat. `READY_FOR_LIVE_TEST`: no.

## This slice — Axolotl convert and seat print path

- `estate enrich gguf-convert` and `estate enrich local-seat` accept an `axolotl-lora` or `axolotl-qlora` train prepare plus a merged Hugging Face directory (`config.json` and a `.safetensors` file whose name does not start with `adapter_model`). `gguf-convert` prints `python3 convert_hf_to_gguf.py <dir> --outfile <sibling>.gguf --outtype auto`. `local-seat` prints the same `ollama create` line it prints for a LLaMA-Factory export, including a sibling `.gguf`. Neither command shells out, writes a GGUF, or promotes.
- The operator owns the merge into that directory. In-tree Axolotl docs name `axolotl train` and do not name a merge command, so `PREPARE.md` and `NEXT.md` do not invent one. Axolotl does not write GGUF. The ladder is train, operator merge, `gguf-convert`, `local-seat`, `import-trained`.
- An adapter directory, `config.json` plus only `adapter_model*.safetensors`, a directory that only holds `export.yaml`, and a symlink stay `refuse:seat`. `unsloth-qlora` and `mlx-lm-lora` stay `refuse:driver`. Sacred, SKU, job, and promoted refuses are unchanged.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Target C Qwen QLoRA operator journey

- The popular path is one ladder of commands that already exist: `estate enrich prepare --driver llamafactory-qlora` (seat tag separate from the train base), the `NEXT.md` `llamafactory-cli train` and `llamafactory-cli export` lines, `estate enrich gguf-convert` (prints `python3 convert_hf_to_gguf.py` with `--outtype auto`), `estate enrich local-seat` (prints `ollama create`), and `estate enrich import-trained` (records `trained_shape` and `trained_paths`). A 5090 smoke seated `llama3` and trained `Qwen/Qwen2.5-0.5B-Instruct`. `template` is `qwen`. A missing train base is `refuse:train-base`.
- The walk is section 8 of `docs/operator-enrich-journeys.md`. `docs/TRAIN-ENRICH.md` and `estate help enrich` / `estate help train` print the same ladder.
- `make qlora-journey` prints that ladder and checks the prepare artifacts on a throwaway estate copy. A missing export is `refuse:seat` and writes no GGUF. The script does not run LLaMA-Factory, llama.cpp, or Ollama, and it does not promote. It is not in `make smoke`, `make gate-90`, or GitHub Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — status and doctor report the enrich prepare tree

- `estate status` and `estate doctor` read `{state_dir}/enrich` when that directory is present. Each `prepare.json` prints pack, driver, job, `seat_tag` and `train_base` when those fields are present, `trained_shape` when `import-trained` recorded it, and the out path. The line is the prepare record. The factory did not train, merge, convert, or seat that model.
- A missing enrich directory stays silent. Status and doctor do not invent a prepare count or claim zero packs. An empty directory notes that no `prepare.json` is present.
- A `prepare.json` that does not parse, or whose schema or required fields fail, refuses before the status page and FAILs doctor before `factory ready`. A symlink in the enrich tree is `refuse:enrich-index`. The walk opens `prepare.json` with `O_NOFOLLOW` and keeps the opened file inside the cell state directory.
- Train catalog lines print the in-tree card status (`integration`, `optional`, or `portable`) with `live=false`. `mlx-lm-lora` is optional, the same way `unsloth-qlora` is. A prepare probe is not live. `READY_FOR_LIVE_TEST`: no.

## This slice — optional mlx-lm LoRA handoff

- `mlx-lm-lora` is an optional `TrainEnrichDriver` card (`status=optional`). `estate enrich prepare --driver mlx-lm-lora` writes `MLX.md`, `PREPARE.md`, `NEXT.md`, and `prepare.json` when `host_class_affinity` is `apple-silicon`. `MLX.md` is an operator-owned handoff. It records the Ollama seat tag, the train base, and `host_class_affinity: apple-silicon`. It is not an mlx-lm config and not a training script.
- The card is the Apple Silicon LoRA handoff. It is not the product, and it does not make the `mlx` local-runtime card live. `NEXT.md` points at the public mlx-lm LoRA page (https://github.com/ml-explore/mlx-lm/blob/main/mlx_lm/LORA.md), the install line that page publishes (`pip install "mlx-lm[train]"`), the command name `mlx_lm.lora`, and the fuse command that page publishes (`mlx_lm.fuse --model <path_to_model>`). This factory does not run that install, does not call mlx-lm, does not choose ranks or iterations, and does not shell out.
- Another affinity (`any`, `consumer-nvidia`, `rented-nvidia`, or any other string) is `refuse:host` and writes nothing. `--all-drivers` omits this card unless the affinity is `apple-silicon`, and it prints that omission. The other train cards still prepare. A missing train base, a bare Ollama tag, or a seat-looking local leaf is `refuse:train-base` and writes nothing. `import-trained` accepts the prepare and refuses when `MLX.md` `train_base_model`, `seat_tag`, or `host_class_affinity` does not match `prepare.json`, and when that affinity is not `apple-silicon`.
- This card does not write `dataset.jsonl`, a YAML recipe, or `max_steps`. `--from-feed` on this card alone is `refuse:dataset`. `--official-scale` on this card alone is `refuse:official-scale`. `--max-steps 0` is still `refuse:max-steps`. `--job enrich` is `refuse:job`. LLaMA-Factory `recipe.yaml` is unchanged. `NEXT.md` on the LLaMA-Factory and Axolotl cards names this handoff. `gguf-convert` and `local-seat` still read a LLaMA-Factory prepare.
- Deliberately not invented: an MLX script, a recipe DSL, ranks, iterations, a dataset writer, a download, a process spawn, or a trainer call. `READY_FOR_LIVE_TEST`: no.

## This slice — optional Unsloth QLoRA handoff

- `unsloth-qlora` is an optional `TrainEnrichDriver` card (`status=optional`). `estate enrich prepare --driver unsloth-qlora` writes `UNSLOTH.md`, `PREPARE.md`, `NEXT.md`, and `prepare.json`. `UNSLOTH.md` is an operator-owned handoff. It records the Ollama seat tag and the train base. It is not an Unsloth config and not a training script.
- The card is the Nvidia-only QLoRA alternate for a faster single-GPU run. It is not the product. Portable local runtimes stay swappable. `NEXT.md` points at the Unsloth install page, the fine-tuning guide, and the README install line `uv pip install unsloth --torch-backend=auto`. This factory does not run that install, does not call Unsloth, and does not shell out.
- A missing train base, a bare Ollama tag, or a seat-looking local leaf is `refuse:train-base` and writes nothing. `import-trained` accepts the prepare and refuses when `UNSLOTH.md` `train_base_model` or `seat_tag` does not match `prepare.json`.
- This card does not write `dataset.jsonl`, a YAML recipe, or `max_steps`. `--from-feed` on this card alone is `refuse:dataset`. `--official-scale` on this card alone is `refuse:official-scale`. `--max-steps 0` is still `refuse:max-steps`. `--job enrich` is `refuse:job`. `--all-drivers --job train` writes the handoff beside the recipe cards. The enrich default skips it.
- Deliberately not invented: an Unsloth script, a recipe DSL, ranks, sequence length, save knobs, a dataset writer, an official-scale cutoff, a download, or a trainer call. `READY_FOR_LIVE_TEST`: no.

## This slice — print the llama.cpp GGUF convert line

- `estate enrich gguf-convert --prepared <dir> --weights <merged-export-dir>` checks a `llamafactory-lora` or `llamafactory-qlora` train prepare and prints the llama.cpp line `python3 convert_hf_to_gguf.py <dir> --outfile <sibling>.gguf --outtype auto`. `--outtype auto` is that script's default (highest-fidelity 16-bit float). The outfile is a sibling of the merged directory. The report then prints `estate enrich local-seat` for that sibling file. The command does not convert, does not shell out, and does not write a GGUF.
- An adapter directory, `config.json` plus only `adapter_model*.safetensors`, a symlinked `--weights` path, a symlinked marker, a path that is already a GGUF, and a directory that matches more than one shape are `refuse:seat`. The same classify path as `local-seat` decides those stops. A GGUF points at `local-seat`. `refuse:train-base`, `--from-feed`, and `import-trained` are unchanged.
- `PREPARE.md` and `NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` name `gguf-convert` and the same convert line. `local-seat` on a merged directory prints that line too. `READY_FOR_LIVE_TEST`: no.

## This slice — official SFT scale and an honest merge card

- `--official-scale` on `estate enrich prepare` and `estate enrich from-pack` applies to LLaMA-Factory cards only. It writes the `examples/train_lora/qwen3_lora_sft.yaml` scale into `llamafactory-lora` and `llamafactory-qlora`: `cutoff_len` 2048, `num_train_epochs` 3.0, `gradient_accumulation_steps` 8, `warmup_ratio` 0.1. Rank, packing, and quantization stay on the selected card. `axolotl-lora` and `axolotl-qlora` stay on their example files (`examples/llama-3/lora-1b.yml` and `examples/llama-3/qlora.yml`). Omit the flag for the short LLaMA-Factory recipe (`cutoff_len` 512, one epoch, grad accum 4, warmup 0.03). `--max-steps` still overrides `num_train_epochs` and still lowers `save_steps` when the count is under 50.
- `--official-scale` with no train recipe card is `refuse:official-scale` and writes nothing. `--all-drivers --job train` still prepares the Modelfile and the external manifest.
- `export.yaml` stays the merge card. `adapter_name_or_path` equals recipe `output_dir`. Comments record `merge_status: not_run`. `NEXT.md` and `PREPARE.md` say this prepare did not merge and that `llamafactory-cli export` has not run. An early stop can leave the adapter under `checkpoint-<step>`. Point `adapter_name_or_path` at that directory. Prepare does not rewrite `export.yaml` after train. A Modelfile written into the export directory by `llamafactory-cli export` belongs to that tool.
- `import-trained` refuses `export.yaml` when a real key `quantization_bit` or `quantization_method` is set (`refuse:export`). A comment line does not trip that refuse. Retargeting `adapter_name_or_path` at a checkpoint directory stays allowed. The file still omits quantization when prepare writes it.
- The factory does not run `llamafactory-cli` and does not merge. `READY_FOR_LIVE_TEST`: no.

## This slice — import-trained records the artifact shape

- `estate enrich import-trained` accepts three shapes from a `llamafactory-lora`, `llamafactory-qlora`, `axolotl-lora`, or `axolotl-qlora` train prepare: an adapter `output_dir` (`adapter_config.json`, plus adapter weights when they are there), a merged `export_dir` (`config.json` and at least one `.safetensors` file, optional `Modelfile`), or a `.gguf` file. A file that is not GGUF, a directory that matches none of those shapes, or a directory that matches more than one, is `refuse:adapter` and writes no proposal.
- `prepare.json` and `binding-proposal.json` record `trained_shape` and `trained_paths` for the shape that was accepted. `promoted`, `auto_apply`, and `estate_rewritten` stay false. The command does not rewrite the estate and does not promote. Curator stays `jason`. Sacred, SKU, and frontier-without-a-binding still refuse.
- `NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` prints the import-trained command for the recipe `outputs/` directory, the `export/` directory, and a GGUF path. `estate help enrich` shows the same command for both cards with those directories filled in under `.cell/enrich`.
- A merged export counts `.safetensors` files whose names do not start with `adapter_model`. `config.json` plus `adapter_model.safetensors` and no `adapter_config.json` is `refuse:adapter`.
- Marker files are regular files. `import-trained` opens each one with `O_NOFOLLOW` and pins that handle inside the artifact directory, the same spirit as `--from-feed`. A symlinked marker or a symlinked `--adapter` path is `refuse:adapter`. The Modelfile and the primary file are read from those handles. The suite does not reproduce a concurrent swap; a path that becomes a symlink before the open fails closed.
- Classify and scan finish before any write. The proposal and the `prepare.json` trained fields publish together. A failed publish restores the previous prepare and proposal bytes and removes a partial proposal, so apply cannot accept an import that omitted `trained_shape` and `trained_paths`.
- A GGUF directory with more than one top-level `.gguf` is `refuse:adapter`. Point `--adapter` at one `.gguf` file, or at a directory that holds exactly one. A `.gguf` in a subdirectory does not count.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Axolotl LoRA and QLoRA prepare

- `axolotl-lora` writes bf16 LoRA. `axolotl.yml` sets `adapter: lora`, `load_in_8bit: false`, and `load_in_4bit: false`. `sequence_len` is 2048, `micro_batch_size` is 2, `gradient_accumulation_steps` is 2, and `lora_r` is 16, matching Axolotl `examples/llama-3/lora-1b.yml`. `lora_alpha` is 32, `num_epochs` is 1, and `optimizer` is `adamw_8bit`, matching that file.
- `axolotl-qlora` is the 4-bit card. `axolotl.yml` sets `load_in_8bit: false`, `load_in_4bit: true`, and `adapter: qlora`. `sequence_len` is 4096, `micro_batch_size` is 2, `gradient_accumulation_steps` is 4, and `lora_r` is 32, matching Axolotl `examples/llama-3/qlora.yml`. `lora_alpha` is 16, `num_epochs` is 4, and `optimizer` is `paged_adamw_32bit`, matching that file.
- Both cards write `base_model` from the train base. `lora_target_linear` is true. `sample_packing` is true. `val_set_size` is 0.0 and `evals_per_epoch` is 0 so a short scaffold does not ask Axolotl to eval an empty split. Flash attention and a Llama pad token stay unset. The QLoRA note names upstream `attn_implementation: flash_attention_2` and leaves that field unset. The default recipe leaves `max_steps` unset and writes `saves_per_epoch: 1`.
- `--max-steps 10` writes `max_steps` and `save_steps` and omits `saves_per_epoch`. Axolotl refuses those two save fields together, and `max_steps` precedes `num_epochs`. `--max-steps 0` is `refuse:max-steps`.
- `--from-feed` applies to `axolotl-lora` and `axolotl-qlora` the same way it applies to `llamafactory-lora` and `llamafactory-qlora`. A prepare with no train recipe card refuses and names those four drivers.
- `NEXT.md` names `axolotl train` and the Axolotl quickstart. The factory does not run Axolotl and does not install a GPU stack. `refuse:train-base`, sacred, SKU, and frontier are unchanged. `READY_FOR_LIVE_TEST`: no.

## This slice — local seat after LLaMA-Factory export

- `estate enrich local-seat` validates a merged export directory (`config.json` and at least one `.safetensors` file, optional `Modelfile`) or a `.gguf` file and prints the `ollama create` line for `cell-enrich-{pack}`. The seat tag is `prepare.json` `seat_tag`. A GGUF prints a Modelfile whose `FROM` is that file, copying TEMPLATE lines when a LLaMA-Factory Modelfile is in the same directory. The command does not write, does not shell out to ollama or llama.cpp, and does not promote.
- `prepare.json` records `export_yaml` when prepare wrote `export.yaml`, and `modelfile` when prepare wrote `Modelfile`. `PREPARE.md` and `NEXT.md` on `llamafactory-lora` and `llamafactory-qlora` name the chain: export, optional llama.cpp `convert_hf_to_gguf.py`, then `ollama create` FROM the GGUF or FROM the merged directory when LLaMA-Factory wrote the Modelfile. `import-trained` records that same merged directory or GGUF and writes `trained_shape` and `trained_paths`. An adapter directory stays on `import-trained`.
- A `.safetensors` name that starts with `adapter_model` is not merged evidence. `config.json` plus `adapter_model-00001-of-00002.safetensors` and no other merged weight is `refuse:seat` and does not print `ollama create`. A symlinked `--weights` path or a symlinked marker (`config.json`, `Modelfile`, `.gguf`, `.safetensors`) is `refuse:seat`. Opens use `O_NOFOLLOW`.
- Printed paths are single-quoted when they contain whitespace, a newline, a quote, or any of `;`, `|`, `&`, `<`, `>`, `(`, `)`, `!`, `*`, `?`. The command still does not run.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Phi-3 Instruct QLoRA reproduce target

- `llamafactory-qlora` infers LLaMA-Factory template `phi` for `microsoft/Phi-3-mini*`, `microsoft/Phi-3-medium*`, and Phi-3.5 (`microsoft/Phi-3.5-mini-instruct`, `microsoft/Phi-3.5-MoE-instruct`), including those ids as nested path segments and HF cache directories (`models--microsoft--Phi-3-mini-4k-instruct`). Phi-3-small infers `phi_small`. Phi-4 infers `phi4`. Phi-4-mini infers `phi4_mini`. The names follow LLaMA-Factory `register_model_group` in `constants.py`.
- The QLoRA card still writes `quantization_method: bnb` and `quantization_bit: 4`. `prepare.json` keeps the Ollama seat tag on `base_model` / `seat_tag` and the Phi repo on `train_base_model`. `NEXT.md` and `PREPARE.md` name this prepare as a reproduce target beside Qwen LoRA/QLoRA.
- `examples/fixtures/phi3-instruct.pack.json` is the smoke pack (`model_hint` `llama3`, `train_base_model` `microsoft/Phi-3-mini-4k-instruct`). Prepare does not download weights. `READY_FOR_LIVE_TEST`: no.

## This slice — LLaMA-Factory LoRA without quantization

- `llamafactory-lora` is a `TrainEnrichDriver` card beside `llamafactory-qlora`. `estate enrich prepare --driver llamafactory-lora` writes the same files (`recipe.yaml`, `export.yaml`, `dataset_info.json`, instruct chat `dataset.jsonl`). The recipe is SFT LoRA: `finetuning_type: lora`, no `quantization_bit`, no `quantization_method`, `lora_rank: 8`, `lora_alpha: 16`, `packing: false`. Rank 8 matches LLaMA-Factory `examples/train_lora/qwen3_lora_sft.yaml`. `cutoff_len` stays 512. `NEXT.md` names the official longer values (`cutoff_len` 2048, `num_train_epochs` 3.0, `gradient_accumulation_steps` 8, `warmup_ratio` 0.1).
- Select QLoRA with `--driver llamafactory-qlora`. That card still writes `quantization_bit: 4`, `quantization_method: bnb`, and rank 16, and `NEXT.md` still installs `pip install 'bitsandbytes>=0.49'`. The LoRA card's `NEXT.md` says that path does not require bitsandbytes.
- `template` is inferred by scanning path segments of the train base on both LLaMA-Factory cards, starting at the last segment. A leaf such as `weights` or an HF snapshot hash uses the nearest ancestor that names a family. A Qwen3 name that contains `instruct` and does not contain `thinking`, or that contains `nothink`, uses `qwen3_nothink` (`Qwen/Qwen3-4B-Instruct-2507`). Other Qwen3 names use `qwen3`. Older Qwen names stay `qwen`.
- `--from-feed` applies to `llamafactory-lora` the same way it applies to `llamafactory-qlora` and `axolotl-lora`. Default prepare still writes a scaffold (or a stub when `source_paths` is empty). `PREPARE.md` and `NEXT.md` on the LoRA card use that dataset honesty note and name `refuse:dataset`.
- `export.yaml` still omits quantization on both cards. `refuse:train-base` and the seat-tag versus train-base split are unchanged, and they apply to `llamafactory-lora`. `import-trained` accepts that prepare. `--all-drivers --job train` writes the LoRA directory beside QLoRA.
- The factory does not run `llamafactory-cli`. `READY_FOR_LIVE_TEST`: no.

## This slice — `--from-feed` checks the raw record, pins the opened file, and caps bytes

- A line with `kind` or `object_class` is classified before ShareGPT or Alpaca dispatch. A frontier event wrapped as `messages` or `instruction` is `refuse:frontier-invent` when the estate has no frontier binding. Sacred, SKU, and raw-secret checks see the raw record, including fields that are not copied into the row.
- Prepare opens each source and reads that same file handle. The opened file must stay inside the cell state directory. A path that cannot be pinned is `refuse:dataset`.
- Each source is at most 8 MiB. All sources together are at most 8 MiB. Each chat and Alpaca copy is at most 16 MiB. A total over that cap is `refuse:dataset`.
- `READY_FOR_LIVE_TEST`: no.

## This slice — dataset scaffold is honest, and `--from-feed` copies rows already on disk

- Default `llamafactory-qlora` and `axolotl-lora` prepare still writes a scaffold `dataset.jsonl` (or a three-row stub when `source_paths` is empty). `prepare.json` records `dataset_mode`, `dataset_rows`, `dataset_from_feed` false, `dataset_skipped`, and `dataset_read_paths`. `PREPARE.md` and `NEXT.md` on both cards say those rows are not training data and name `refuse:dataset`.
- `--from-feed` reads pack `source_paths` under `--state-dir` and copies instruct rows. ShareGPT `messages`, Alpaca `instruction` / `output`, and a scrubbed feed event with a `note` are the shapes it accepts. Events with no note are skipped. A missing file, a path outside the cell directory, or a line that is not an instruct row is `refuse:dataset` and writes nothing. Sacred, SKU, raw secret, and frontier-without-a-binding still refuse. This factory does not download pack sources and does not train.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Axolotl train base matches the seat split

- `axolotl-lora` writes `base_model` in `axolotl.yml` from the train base (pack `train_base_model`, or `params.train_base_model` on the local binding; the pack wins). That value is a Hugging Face repo id (`namespace/name`) or a local directory of HF weights. A relative directory is stored as an absolute path.
- The Ollama seat tag stays `base_model` and `seat_tag` in `prepare.json`, and in `NEXT.md`, for Modelfile `FROM` and the adapter join. `--all-drivers --job train` writes that train base into `axolotl.yml` and leaves Modelfile `FROM` as the seat tag.
- A missing train base, a bare Ollama tag, or a local path whose directory name is an Ollama seat tag is `refuse:train-base` and writes nothing. `import-trained` refuses the same gap, and refuses when `axolotl.yml` `base_model` does not match `prepare.json`. This factory does not map a seat tag onto a Hub repo and does not download weights.
- Unsloth stays a `NEXT.md` pointer. `READY_FOR_LIVE_TEST`: no.

## This slice — absolute local train base

- A relative LLaMA-Factory train base (`./…` or `../…`) is stored as an absolute path in `recipe.yaml`, `export.yaml`, `prepare.json`, and `NEXT.md`. A Hugging Face repo id stays as typed. The weights directory does not need to exist at prepare time.
- A local path whose directory name is an Ollama seat tag (`./llama3`, `../llama3`) is `refuse:train-base` and writes nothing.
- `READY_FOR_LIVE_TEST`: no.

## This slice — seat tag and LLaMA-Factory train base

- `llamafactory-qlora` keeps the Ollama seat tag (`prepare.json` `base_model` / `seat_tag`, Modelfile `FROM`) separate from the train base (`model_name_or_path`). Set pack `train_base_model` or `params.train_base_model` on the local binding to a Hugging Face repo id (`namespace/name`) or a local directory of HF weights. The pack field wins. `template` is inferred from the train base.
- A missing train base, or a bare Ollama tag such as `llama3`, is `refuse:train-base` and writes nothing. `import-trained` refuses the same gap, and refuses when `recipe.yaml` or `export.yaml` `model_name_or_path` does not match `prepare.json`. This factory does not map a seat tag onto a Hub repo.
- QLoRA install notes name `pip install 'bitsandbytes>=0.49'` because `pip install llamafactory` and `llamafactory[torch,metrics]` 0.9.5 did not pull it. A 5090 smoke used torch `2.11.0+cu128` and bitsandbytes 0.50.2. That install did not replace torch.
- `--max-steps N` writes a gauge recipe. The default recipe stays `num_train_epochs: 1.0`, `save_steps: 50`, and leaves `max_steps` unset. `quantization_method` stays `bnb`.
- `READY_FOR_LIVE_TEST`: no.

## This slice — LLaMA-Factory QLoRA method token

- `llamafactory-qlora` writes `quantization_method: bnb`. LLaMA-Factory 0.9 selects the 4-bit bitsandbytes branch only for that token.
- `READY_FOR_LIVE_TEST`: no.

## This slice — LLaMA-Factory QLoRA is the train card

- `llamafactory-qlora` is the primary train `TrainEnrichDriver`. `estate enrich prepare --driver llamafactory-qlora` writes `recipe.yaml` (SFT QLoRA, 4-bit, LoRA rank 16, `cutoff_len` 512, packing on), `export.yaml`, `dataset_info.json`, instruct chat `dataset.jsonl`, `PREPARE.md`, `NEXT.md` with `pip install llamafactory`, `llamafactory-cli train`, and `llamafactory-cli export`, and `prepare.json` (`job: train`). The factory does not run the CLI, does not download weights, and does not call CUDA.
- `axolotl-lora` stays the YAML recipe card for a config-driven or multi-GPU run. It still does not run Axolotl.
- Unsloth QLoRA is a `NEXT.md` pointer on the LLaMA-Factory card (Nvidia only). It is not a registered driver.
- `import-trained` accepts either train recipe and writes the same `local_slm` proposal. Ollama seating stays outside the factory. Merge drops `quantization_bit`. GGUF conversion stays with llama.cpp after that merge.
- Train hosts are `consumer-nvidia` and `rented-nvidia`. `apple-silicon` can prepare. `NEXT.md` says the card expects CUDA LLaMA-Factory. `make train-prepare` prints `SKIP live train` and is off smoke, `gate-90`, and Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Unsloth QLoRA is the train card

- `unsloth-qlora` is the primary train `TrainEnrichDriver`. `estate enrich prepare --driver unsloth-qlora` writes `train_unsloth.py` (QLoRA 4-bit, LoRA r=16, `max_seq_length` 512), instruct chat `dataset.jsonl`, `PREPARE.md`, `NEXT.md` with `pip install unsloth` and `python train_unsloth.py`, and `prepare.json` (`job: train`). The factory does not run the script, does not install Unsloth, and does not export GGUF.
- `axolotl-lora` stays the YAML recipe card for a config-driven or multi-GPU run. It still does not run Axolotl.
- `import-trained` accepts either train recipe and writes the same `local_slm` proposal. Ollama seating stays outside the factory. GGUF and Ollama export stay on Unsloth's docs (`save_pretrained_gguf`).
- Train hosts are `consumer-nvidia` and `rented-nvidia`. `apple-silicon` can prepare. `NEXT.md` says the card expects CUDA. `make train-prepare` prints `SKIP live train` and is off smoke, `gate-90`, and Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — Axolotl train recipe

- `axolotl-lora` is a `TrainEnrichDriver` card beside `ollama-modelfile` and `external-manifest`. `estate enrich prepare --driver axolotl-lora` writes `axolotl.yml` (QLoRA: `load_in_4bit` and `adapter: qlora`), `dataset.jsonl`, `PREPARE.md`, `NEXT.md` with the exact `axolotl train` line, and `prepare.json` (`job: train`). The factory does not run Axolotl, does not download a dataset, and does not rewrite `estate.yaml`.
- Default job for that card is `train`. `--job enrich` is `refuse:job` and writes nothing. `--all-drivers` includes the card when the job is `train`. The enrich default still prepares the other two cards.
- `estate enrich import-trained` checks an adapter directory or a merged GGUF and writes the same `local_slm` binding proposal as `import-prepared`. `apply-proposal`, `plan`, and `apply --require-plan` stay the join. Ollama stays the local-run seat.
- Train hosts for the card are `consumer-nvidia` and `rented-nvidia`. `apple-silicon` can prepare; `NEXT.md` says the Axolotl GPU path expects CUDA. No MLX trainer.
- Opt-in `make train-prepare` asserts the recipe, the `axolotl train` line, `prepare.json` `job=train`, and an untouched `examples/estate.yaml`. It prints `SKIP live train`. Off smoke, `gate-90`, and Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — seated FROM, from-pack, live prove

- Keeps `estate enrich apply-proposal`. The proposal still stages `{state}/enrich-stage/staged-estate.yaml` for `estate plan` and `estate apply --require-plan`. The source estate is written only when that apply succeeds. Status and doctor still name a pending join. `auto_apply` stays false.
- `ollama-modelfile` writes `FROM` as the seated model. That is `params.model` on the local binding, or a pack `model_hint` that is already a model tag (`llama3`). The binding id `local_slm` is not a model tag. A missing seated name is `refuse:base-model` and writes nothing. Sacred, SKU, curator, and frontier-invent still refuse before write.
- `estate enrich from-pack` prepares an accepted pack (id in `packs/` or `packs/accepted/`, or a pack JSON path) into `{state_dir}/enrich`. Omitting `--driver` prepares every card. Same refuses. Does not apply, train, or rewrite `estate.yaml`.
- Opt-in `make enrich-live-prove` copies the example estate into `/tmp/cell-one-enrich-live-prove`, sets `params.model` from the seat, runs `from-pack`, `ollama create`, `ollama show`, and `import-prepared`, then removes the tag. `examples/estate.yaml` stays untouched. Seat down prints `SKIP` and exits 0. Not in smoke, `gate-90`, or Actions. A hardware SKU anywhere in the output path still refuses.
- No third train/enrich card. llama.cpp already reads an INI preset, and that preset needs a GGUF path this estate does not carry.
- `READY_FOR_LIVE_TEST`: no. The live script is an opt-in seated-runtime enrich handoff. It is not a factory-wide live test.

## Day 0–30 (PR #1)

One-box factory proving A1–A4. Horizon / Research / Sanctum on separate lanes. Sacred exclusions (Cyera CI, Rust classroom) fail closed. Isolation is a driver (profile-dir today). No live provider required. Dual PE, vault, multi-box, and AI-gateway stay out of altitude.

## Day 31–60 (on `main` with #1)

Mixed model estate A7–A9. Equal-class frontier (`grok-4.7`) + local. Ollama-first, llama.cpp swap-proof. Fail-closed when local is down — no silent `grok-4.7` fallback. Hardware is a driver (`consumer-nvidia` / `apple-silicon` / `rented-nvidia`), not a product fork. GitHub is the only source of truth.

## Day 61–90 (PR #2)

Beachhead toward A10–A12. Feed packs never auto-promote. Cloud-agent is declared, not spawned. Overnight waves added convey mesh, dry-run apply, lease TTL, doctor, smoke, dual-layer sacred file, and the GATE-90 checklist. Curator is Jason / manual.

## PR #3 — live probes and the day90 loop

Optional `estate probes --live` pings a specialist endpoint when you set `CELL_*`. Unset endpoints print SKIP and exit 0. CI never needs a Mac or a GPU. `make day90` walks status → plan → dry-run → apply → reconcile on an isolated cell. Probe ids refuse hardware SKUs the same way bindings do.

## PR #4 — suggest and accept stay instructions-only

`estate reconcile --suggest` writes a patch file. It does not rewrite leases. `estate packs accept --curator jason` writes enrich-pack edit instructions. It does not rewrite `estate.yaml`. Wrong curator refuses. Jason still pastes by hand.

## PR #5 — doctor --strict and a thin gate-90

`estate doctor --strict` is the pre-merge operator check: compile-only CI body, locked sacred file, dual-layer demo, refuse fixtures, floor has no vendor needles. `make gate-90` is a thin local alias (smoke + day90 + that checklist). The dual-layer demo keeps Sanctum first-class; Sanctum is not Cyera. Omitting a locked sacred id from the overlay file still refuses Cyera CI.

## PR #6 — feed-loop and honest parking

README leads with `make gate-90` as the Day-90 operator entrypoint. `make feed-loop` walks scrubbed traces → pack → propose → accept on an isolated cell. The feed cursor stays on disk; rematerialize does not auto-promote. Placement-actual JSON round-trips every reconcile refuse code. `docs/DAY90-PLUS.md` parks live Mac MLX, live GPU, and cloud-spawn until Jason has boxes. Those rows are not green.

## PR #7 — help, backup prune, convey policy

`estate help [topic]` prints Day-90 pages for status, plan, apply, reconcile, feed-loop, and backup. Unknown topics refuse. `estate backup --prune N` keeps the newest N archives; `N=0` refuses. Convey `call` refuses on `policy-deny.yaml`.

## PR #8 — layout, local-only honesty, operator runbook

`docs/cell-layout.md` matches the paths the code writes. `docs/OPERATOR-DAY.md` walks `make gate-90` → `make feed-loop` → `estate backup --prune` on isolated cells. A hardening test locks `make gate-90` off Actions (it wraps `cargo test --workspace`). No leftover Origin URLs. No new `estate version` command.

## PR #9 — dual-layer-demo e2e and Cell One snapshot

Isolated dual-layer-demo loop: validate → plan → dry-run → apply → status → reconcile → backup → prune. README cross-links OPERATOR-DAY and FEED-LOOP. Dead leftover `ops.rs` wrappers removed. Snapshot: `docs/CELL-ONE-STATUS.md`.

## PR #10 — lease-refresh after expire --forget

Hole: `estate expire --forget` dropped leases, then apply treated that as `refuse:drift` and demanded `--force`. Apply now restamps (`lease-refresh`). Isolated TTL e2e on `examples/fixtures/ttl-short.yaml`. Not a new verb.

## PR #11 — sacred overlay e2e

Isolated sacred overlay e2e: `sacred-omit-locked.yaml` (`locked: []`) still refuses `cyera-ci` / `rust-classroom` on convey hop. `lab-notebook` refuses only with the overlay installed. Dual-layer-demo still validates. No new verb.

## After PR #11 (this slice)

- Hole: `estate convey expire --forget` dropped hop decls with the leases, then call returned `refuse:no-lease`. Forget now keeps decls; call restamps (`lease-refresh`). Mirrors placement apply after forget. Not a new verb.
- Isolated hop TTL e2e: declare `ttl-secs: 1` → expire lists → call `refuse:expired` → `--forget` → call restamps. No JSON mutation.
- Pause-kit still holds after an unchanged apply → suspend → resume.

## After PR #12 (this slice)

- Hole: `estate restore` treated empty/missing backup `sacred_ids` as a match and could write. Empty set is now `refuse:sacred-mismatch` (fail closed). Dry-run and live restore both write nothing.
- Packs propose → accept → promote still leaves `promoted=false` and does not write the estate. `reconcile --suggest` on drift still does not rewrite leases. Vanilla `doctor` stays thinner than `--strict`.

## After PR #13 (PR #14)

No new hole. `plan diff --allow-wider` / `plan export-pr` exits locked. `apply --dry-run` under refuse writes nothing (snapshot covers conveyor/sessions too). Curator: wrong → `refuse:curator`; accept missing flag is clap; import still defaults to jason. CELL-ONE-STATUS states #10–#13 in plain English.

## This slice — apply-proposal stages the local_slm join

- `estate enrich apply-proposal` reads `binding-proposal.json`. It checks schema, curator, sacred, hardware, frontier, and `prepare.json`. It writes `{state}/enrich-stage/staged-estate.yaml` and `stage.json` (`cell-one.enrich-binding-stage.v0`). That staged file is the input for the existing `estate plan` and `estate apply --require-plan`. The command does not apply. The source estate stays unchanged until that require-plan apply succeeds.
- The same tag and binding again is a no-op. A missing proposal, a prepare mismatch, a wrong tag, a stale estate hash, or a different pending stage refuses before any stage write.
- `--verify-local-tag` is off by default. When set, the seated runtime must list the tag (`GET /v1/models` or `/api/tags`). A failed probe is `refuse:local-tag` before any stage write. No new local server.
- `estate status` and `estate doctor` name a pending enrich join when `.cell/enrich` holds a prepare or proposal and `local_slm` is still unbound. An unreadable proposal, prepare, or stage refuses before the page. A pending note is not a factory-ready failure.
- Opt-in `make enrich-prepare` walks prepare, list, import-prepared, apply-proposal, plan, and require-plan apply on a throwaway lab copy. `examples/estate.yaml` stays unchanged. Off smoke, gate-90, and Actions.
- llama.cpp stays on the existing OpenAI-compat seat. `--verify-local-tag` uses that probe. `READY_FOR_LIVE_TEST`: no.

## This slice — prepare, list, import-prepared

- `estate enrich prepare --all-drivers` writes every `TrainEnrichDriver` card into sibling directories. One refuse writes none of them. Each directory gains `NEXT.md`: artifact paths, the exact handoff (`ollama create … -f <Modelfile>` or the external manifest files), the `import-prepared` line, and fail-closed reminders. Prepare still does not shell out, train, POST, promote, or rewrite `estate.yaml`.
- `estate enrich list` reads `{state_dir}/enrich/{pack}/{driver}/prepare.json` and prints pack, driver, job, tag, and out path. A missing directory is `refuse:enrich-index`. List does not create it and does not write.
- `estate enrich import-prepared` checks `prepare.json`, the tag `cell-enrich-{pack_id}`, and an operator file. It writes `binding-proposal.json` and `binding-proposal.md` (`cell-one.enrich-binding-proposal.v0`) for the existing `local_slm` seat. Paste the snippet, then `estate plan` and `estate apply --require-plan`. Sacred, SKU, curator, missing path, and frontier-invent refuse before that proposal exists. `auto_apply` stays false.
- Opt-in `make enrich-prepare` walks prepare, list, and import on a throwaway directory. Not in smoke, gate-90, or Actions.
- `READY_FOR_LIVE_TEST`: no.

## This slice — spine links

- North star, ubiquitous language, charter, and README point at the prepare page [`docs/TRAIN-ENRICH.md`](docs/TRAIN-ENRICH.md) and the walks [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md). `estate help enrich` names the journeys page. No new command. No trainer. `READY_FOR_LIVE_TEST`: no.

## This slice

- Train/enrich beachhead. `TrainEnrichDriver` in `model-estate`. Drivers: `ollama-modelfile` (Modelfile + `ollama create` steps) and `external-manifest` (JSON/YAML). `estate enrich prepare` writes artifacts under `.cell/enrich/` or `--out`. Sacred, SKU, curator, missing pack, and frontier-invent refuse before write. Does not train, POST, auto-promote, or rewrite `estate.yaml`.
- `estate help enrich` (alias `train`). Opt-in `make enrich-prepare`. Not in smoke, gate-90, or Actions.
- Schema [`schema/train-enrich.v0.json`](schema/train-enrich.v0.json) (`cell-one.enrich-prepare.v0`). Docs: [`docs/TRAIN-ENRICH.md`](docs/TRAIN-ENRICH.md).
- `READY_FOR_LIVE_TEST`: no.

## After PR #85

- Canonical glossary: [`docs/UBIQUITOUS_LANGUAGE.md`](docs/UBIQUITOUS_LANGUAGE.md). A local runtime is an ecosystem seat. Ollama is today's entrant; catalog/route/bind take the next process. Suite goal: facilitate train/enrich of purpose-built small models. Beachhead today: enrich packs and the specialist path. No training stack. Anti-shrink stays gateway, Ollama wrapper-as-product, LM Studio-alone, Grok Bot clone.
- `estate help north-star` / `charter` match that page. README and [`docs/NORTH-STAR.md`](docs/NORTH-STAR.md) use the same words.
- `READY_FOR_LIVE_TEST`: no.

## This slice — operator enrich journeys

- [`docs/operator-enrich-journeys.md`](docs/operator-enrich-journeys.md) walks train/enrich facilitation: an Ollama Modelfile and `ollama create` after pack accept, a later entrant on the same `local_slm` id, and an external manifest for a trainer outside the factory. Fail-closed stops stay sacred, SKU, `model.local.down`, and no auto-promote. Integrate-vs-invent stays the build rule. No new command. No training stack. `READY_FOR_LIVE_TEST`: no.

## After PR #83

- `estate help north-star` (alias `northstar`) and `estate help charter` print the locked product sentence, short anti-shrink bullets, and pointers to `charter.md`, `make gate-90`, `make day90`, and `docs/LIVE-PROBES.md`.
- Unknown help topics still refuse. Not on smoke or `gate-90`. No Actions change. `READY_FOR_LIVE_TEST`: no.

## After PR #84

- Vision reset. README leads with the one-box north-star. [`docs/NORTH-STAR.md`](docs/NORTH-STAR.md) is the one-page product story. Experimental catalog cards (MLX / vLLM / TRT), cloud-agent spawn, and the lease-bound hop stub sit under "Parked / not the product". Packs stay curator edit instructions. Control does not complete.
- Charter status line: Day 0–90 is on `main`. Day 90+ is real-world proof plus parked stubs. Locked defaults are unchanged.
- Opt-in `make real-world` (`scripts/real-world.sh`): north-star line, `cargo check --workspace --locked`, vanilla `estate doctor` on the checkout that holds `examples/estate.yaml`, then live probes and the Ollama specialist only when `CELL_LOCAL_ENDPOINT` is set. Unset prints SKIP and exits 0. Not in smoke, gate-90, or Actions. Does not print the frontier API key. Does not invent PASS.
- Mac specialist complete is recorded on the MacBook Air against tip `2ab78a4`: `"completion": "Pong"`, reason `compat completion`. `READY_FOR_LIVE_TEST` for that command is no. Frontier `pong` and the 5090 `Pong` stay as already recorded. Native MLX stays a stub.
- `READY_FOR_LIVE_TEST`: no.

## After PR #82

- `estate convey leases` refuses before it prints hop lease JSON when a cloud-mesh hop lease is spawned. An unspawned file still prints. A missing mesh still says there are no hop leases. The mesh is not rewritten.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #81

- `estate convey expire` refuses before it lists or forgets an expired cloud hop lease that is spawned. `expire --forget` does not drop that row.
- A missing mesh is not a spawned lease. An expired box hop still drops when that cloud row is not in the drop.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #80

- `estate convey declare` refuses before it writes a cloud hop lease when that placement lease is spawned. It does not record `spawned: false`.
- A missing placement file is not a spawned lease. An unspawned cloud hop still declares as not spawned. The placement file is not rewritten.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #79

- `estate convey call` does not say a cloud hop is not spawned when the placement lease is spawned. A missing hop lease is not restamped to `spawned: false`.
- A missing placement file is not a spawned lease. An unspawned cloud hop still refuses as declared, not spawned.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #78

- `estate convey sync` refuses before it writes a cloud hop lease when the placement lease is spawned. It does not record that hop as `spawned: false`.
- A missing placement file is not a spawned lease. An unspawned cloud placement still syncs. The placement file is not rewritten.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #76 (this slice)

- `estate expire` refuses before it lists leases or forgets them when an expired cloud-agent lease is spawned. `expire --forget` does not drop that row.
- A missing placement file is not a spawned lease. An expired box lease still drops when no spawned cloud row is in that drop.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #75 (this slice)

- `estate suspend` and `floor suspend` refuse before they drop sessions or rewrite leases when a cloud-agent lease is spawned. They do not restamp that lease to unspawned.
- A missing placement file is not a spawned lease. A wired box lease still drops `spawned` on suspend.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #74 (this slice)

- `estate pause-proof` does not print the pause-proof JSON when the cell is drifted. That drift is `pause-proof: drift (fail closed)` with the drift notes.
- The clean note stays on a proof that is in sync. A spawned cloud lease and a lost lease count already refuse before that JSON.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #73 (this slice)

- Apply and resume refuse before they write when a cloud-agent lease is spawned, or `placement-actual.json` does not parse. They do not restamp that lease to unspawned. `--force` does not.
- A missing file is not a spawned lease. An unspawned file still applies.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #72 (this slice)

- `estate leases` and `floor leases` refuse before they print placement JSON when a cloud-agent lease is spawned.
- An unspawned file still prints. A missing file still says there is no placement-actual. The lease file is not rewritten.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #71 (this slice)

- `estate status` refuses before it prints `cloud-agent: declared, not spawned` when a cloud-agent lease is spawned.
- An unspawned cell still prints that line. The lease file is not rewritten.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #70 (this slice)

- `estate backup` and `estate restore` refuse before they write when `placement-actual.json` does not parse, or a cloud-agent lease in that file is spawned.
- A missing file is not a spawned lease. The archive meta does not record `cloud_agent_spawned: false` over that file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #69 (this slice)

- Apply, resume, and suspend do not journal `from: suspended` when `lifecycle.json` was missing. That `from` stays empty.
- A present file still supplies `from`. A file that does not parse is a refuse before the write. The loader default is not a prior state.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #68 (this slice)

- `floor status` does not invent `suspended` when `lifecycle.json` is missing. It prints `lifecycle: -`.
- A present file that does not parse is a refuse before that line. A file that parses prints `lifecycle:` and `durable=` from the file. `estate status` already prints `paused: -` for a missing file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #67 (this slice)

- `estate status` does not invent `suspended` when `lifecycle.json` is missing. It prints `paused: -` and `lifecycle: -`.
- A present file that does not parse is a refuse before the status page. A file that parses prints `paused:` and `lifecycle:` from the file. `estate doctor` already refused that file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #66 (this slice)

- `estate status` refuses (`refuse:model-actual`) when a present `model-actual.json` does not parse. It does that before the status page. A missing file is not a failure, and status does not invent a binding count.
- A file that parses is not a new status line. `estate drift` already refused that file. `estate doctor` already FAILs it before `factory ready`.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #65 (this slice)

- `estate doctor` FAILs a present `model-actual.json` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a binding count.
- A file that parses prints `model-actual.json bindings=` from the file. `estate drift` already refused that file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #64 (this slice)

- `estate doctor` FAILs a present `desired-snapshot.yaml` that does not parse when no cell catalog is present. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a frontier model from it.
- A file that parses prints `desired-snapshot.yaml name=` from the file. When a cell catalog is present, doctor already reads this snapshot. `estate apply` already refused an unreadable snapshot.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #63 (this slice)

- `estate doctor` FAILs a present `actual-state.json` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a session count.
- A file that parses prints `actual-state.json sessions=` from the file. `estate status` already refused that file through drift.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #62 (this slice)

- `estate doctor` FAILs a present `sessions.jsonl` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a line count.
- A file that parses prints `sessions.jsonl lines=` from the file. `estate sessions` already refused that file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #61 (this slice)

- `estate doctor` FAILs a present `lifecycle.jsonl` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a line count.
- A file that parses prints `lifecycle.jsonl lines=` from the file. `estate history` already refused that file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #60 (this slice)

- `estate doctor` FAILs a present `apply-audit.jsonl` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent a line count.
- A file that parses prints `apply-audit.jsonl lines=` from the file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #59 (this slice)

- `estate doctor` FAILs a present `lifecycle.json` that does not parse. It does that before `factory ready`. A missing file is not a failure, and doctor does not invent `suspended`.
- A file that parses prints `lifecycle.json state=` from the file. `suspended` is not a failure.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #58 (this slice)

- `estate doctor` FAILs a present `conveyor-mesh.json` that does not parse, or whose hop `host_class` is not a class. It does that before `factory ready`. A missing mesh is not a failure.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #57 (this slice)

- `estate status` refuses (`refuse:proposal-unreadable`) when a `*.proposal.json` does not parse, or its id does not match the filename. It does that before the status page. A missing proposals directory is not a proposal. A parsed proposal still lists.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #56 (this slice)

- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) holds a Mac `estate specialist` result row. It is **Pending**, not **PASS**, until a completion is pasted. Do not invent `pong` or `Pong` for the Mac.
- The #55 Mac command stays the open hand-off. This slice does not add another live test.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #55 (this slice)

- `estate status` and `estate doctor` refuse (`refuse:frontier-model`) when the cell `catalog.json` does not parse. They do that before the cell catalog success line. Apply, resume, and pause-proof already refused. A missing catalog is not a disagreement.
- The schema card is not the binding. The refuse does not invent `grok-4.7`.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #54 (this slice)

- Mac `estate specialist` complete is still unrecorded. [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) has the copy-paste for the MacBook Air and the open 5090-class box: `PATH` includes `~/.cargo/bin`, `CELL_LOCAL_ENDPOINT=http://127.0.0.1:11434`, `CELL_LOCAL_MODEL=llama3`, then `estate specialist --driver ollama --prompt "Reply with the single word pong."`.
- The 5090 `Pong` stays recorded. Native MLX stays a stub. `mlx` / `vllm` / `trt` stay `not live-ok`.
- `READY_FOR_LIVE_TEST`: yes for that Mac command only. No new CLI. No new smoke or gate-90 step.

## After PR #53 (this slice)

- `estate probes --live` does not print `live ok` for `mlx`, `vllm`, or `trt`. An answering HTTP endpoint stays `not live-ok`. Ollama, llama.cpp, and http-remote still print `live ok` when their endpoint answers.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #52 (this slice)

- `docs/GATE-90.md` does not call the Ollama complete path ready. Mock completion stays green. The 5090 `Pong` is already recorded. Mac complete is not. `mlx` / `vllm` / `trt` refuse a frontier POST and are not live-ok.
- `docs/DAY90-PLUS.md` parks vLLM and TRT with native MLX. `READY_FOR_LIVE_TEST` is yes only when a concrete live-probe command is still unblocked. None is.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #51 (this slice)

- `estate backup` and `estate restore` refuse before they write. A cell catalog whose frontier model disagrees with the binding is `refuse:frontier-model`. A frontier `source_driver` with no frontier binding is `refuse:frontier-invent`. A desired snapshot whose sacred set disagrees is `refuse:sacred-mismatch`.
- A missing catalog is not a disagreement. A local-only pack still archives. `CELL_FRONTIER_MODEL` is not the binding. The refuse does not invent `grok-4.7` unless that model is already in the catalog file.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #50 (this slice)

- `estate packs import` and `estate feed import` refuse (`refuse:frontier-invent`) when the pack tags `frontier` and the estate has no frontier binding. They do that before an accepted pack, a redaction report, or an index rewrite.
- A local-only pack still imports. A mixed-fixture import keeps `source_drivers` frontier and local and writes a redaction report of kind counts. The report does not store a raw secret and does not invent `grok-4.7`. `CELL_FRONTIER_MODEL` is not the binding.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #49 (this slice)

- `estate resume` and `estate pause-proof` refuse (`refuse:frontier-model`) when the cell `catalog.json` frontier model disagrees with the binding. They do that before leases or a catalog write.
- A missing catalog is not a disagreement. A matching empty catalog still resumes, and the cell model stays empty. `pause-proof` does not invent a cell catalog. `CELL_FRONTIER_MODEL` is not the binding.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #48 (this slice)

- `make day90-mixed` prints `frontier plan: model=grok-4.7 source_drivers=frontier,local` when the mixed fixture is bound. Apply, status, and doctor keep that bound model. `CELL_FRONTIER_MODEL` does not become the binding.
- The same opt-in walk checks a throwaway local-only estate. Plan and dry-run are `refuse:frontier-invent`. Live apply writes no catalog and does not print `grok-4.7`. Status prints no frontier binding. Doctor does not print a cell catalog model.
- No new CLI. No new smoke or gate-90 step. `READY_FOR_LIVE_TEST`: no.

## After PR #47 (this slice)

- `estate models` prints `model=-` when the binding sets no model. `CELL_FRONTIER_MODEL=grok-4.7` does not become the binding.
- `estate catalog` labels the schema card `(schema card, not a binding)`. It refuses (`refuse:frontier-model`) before overwriting a catalog whose frontier model is not that card. A missing file still receives the schema dump. An unset binding stays empty.
- Operator-day and fixtures-check write that schema dump beside the cell catalog. Same checks. No new CLI. No new smoke or gate-90 step.
- `READY_FOR_LIVE_TEST`: no.

## After PR #46 (this slice)

- Live `estate apply` refuses (`refuse:frontier-model`) when the cell `catalog.json` frontier model disagrees with the binding. It does that before leases, an unchanged audit, or a catalog rewrite. A missing catalog is not a disagreement. `--force` does not overwrite it.
- A matching catalog still applies. Unset `params.model` stays empty. The schema card is not copied.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #45 (this slice)

- `estate reconcile` and `estate resume` refuse (`refuse:frontier-invent`) when the estate has no frontier binding. They do that before `reconcile.json`, a suggest patch, or a resume catalog write. They do not copy the schema card or `CELL_FRONTIER_MODEL`.
- An estate that already has a frontier binding still reconciles and resumes. Unset `params.model` stays empty.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #44 (this slice)

- `estate status` and `estate doctor` refuse (`refuse:frontier-model`) when the cell `catalog.json` frontier model disagrees with the binding. Empty and missing are the same (`model=-`). The schema card stays `grok-4.7` and is not treated as the binding.
- A matching cell catalog still prints `model=grok-4.7` or `model=-`.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #43 (this slice)

- `packs propose` and `packs accept` refuse (`refuse:frontier-invent`) when `source_drivers` names `frontier` and the estate has no frontier binding. They do not write the proposal or the enrich-edit file, and they do not copy the schema card.
- A pack that is only `local` still proposes and accepts as `local`. It does not gain a frontier driver.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #42 (this slice)

- `estate plan` and `apply --dry-run` print `frontier plan: model=` from the binding (`model=-` when `params.model` is unset). They do not copy the schema card and they do not read `CELL_FRONTIER_MODEL`.
- An estate with no frontier binding refuses (`refuse:frontier-invent`) before any plan file or dry-run preview. That refuse does not invent a frontier `source_driver` or `grok-4.7`.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #41 (this slice)

- Apply and resume write the cell `catalog.json` frontier model from the estate binding. No `params.model` stays empty. Status prints `catalog frontier: cell model=-`. Doctor does not report that file as `grok-4.7`.
- The schema catalog card stays `grok-4.7`. `estate catalog` still dumps that card. A binding that sets `grok-4.7` still writes it. Two different frontier models refuse.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #40 (this slice)

- `make day90-mixed` validates `examples/hosts/frontier-http.yaml` and prints status (`frontier: frontier_http model=grok-4.7`). No apply. No live key. A greenfield status does not invent a cell catalog. `examples/estate.yaml` cksum is unchanged.
- The host file stays off `scripts/fixtures-check.sh`, smoke, and gate-90.
- `estate models` prints `model=` from `params.model` when set, and `model=-` when it is not. The default estate does not invent `model=grok-4.7`.
- No new CLI.
- `READY_FOR_LIVE_TEST`: no.

## After PR #39 (this slice)

- `examples/hosts/frontier-http.yaml` names `model: grok-4.7` on a frontier `http-remote` binding, with a local `ollama` card. It is not a host-class alias and it is not on smoke or gate-90. `examples/estate.yaml` stays hash-locked.
- A frontier specialist prompt that mentions Cyera or Rust classroom still refuses as sacred when `CELL_FRONTIER_MODEL` is a hardware SKU. No POST. No invented completion. The SKU model path is not the refusal.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## #38–#39 in plain English

#38. `make feed-loop` checks that the pack, the proposal, and `enrich-edit.json` carry the same `source_drivers`. A missing tag with a nonzero frontier or local count refuses the index rewrite and leaves the previous INDEX. Accept serializes the enrich-edit JSON before it writes either file.

#39. Frontier and local `estate specialist` both refuse a Cyera or Rust classroom prompt before POST, and do not invent a completion. README and OPERATOR-DAY point at `estate help frontier` and `make day90-mixed`. Those stay off smoke and gate-90.

## After PR #38 (this slice)

- Frontier `estate specialist` refuses a Cyera or Rust classroom prompt before POST, and does not invent `"completion": "ok"`. The local specialist test locks the same two prompts.
- README start-here and `docs/OPERATOR-DAY.md` point at `estate help frontier` and `make day90-mixed`. Those stay off smoke and gate-90. `examples/fixtures/mixed-frontier-local.yaml` already names `model: grok-4.7` on the frontier `http-remote` binding. `examples/estate.yaml` stays hash-locked.
- No new CLI.
- `READY_FOR_LIVE_TEST`: no.

## #35–#37 in plain English

#35. Frontier refuse text names `CELL_FRONTIER_MODEL` and `CELL_FRONTIER_ENDPOINT`. A hardware SKU in the model id says which setting it came from and still refuses before any POST. GATE-90 and DAY90-PLUS keep green factory checks, recorded live proofs, and parked rows separate. Mac specialist stays optional. Native MLX and cloud-spawn stay parked.

#36. Pack INDEX and `estate feed list` print `drivers=` when `source_drivers` is present, and `drivers=-` when it is empty. A failed INDEX rewrite is an error. `make feed-loop` greps that line. There is no `make feed-loop-mixed`.

#37. `packs accept` copies that tag into the enrich-edit instructions. An empty list stays `-`. A tag that does not match `path_counts` refuses before the edit file is rewritten. A failed proposal INDEX rewrite is an error. `source_drivers` stays an additive v0 field.

## After PR #37 (this slice)

- `make feed-loop` checks the pack, the proposal, and `enrich-edit.json` carry the same `source_drivers` (`frontier` then `local`).
- Hole: a pack or proposal that omitted `source_drivers` while frontier or local counts were nonzero was indexed as `drivers=-`. The index rewrite now refuses and leaves the previous INDEX in place.
- Accept serializes the enrich-edit JSON before it writes either file, so a serialize failure does not leave a new markdown next to a stale JSON.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #36 (this slice)

- `packs accept` copies `source_drivers` from the proposal into the enrich-edit markdown, JSON, and paste comment. Empty stays `-` and is not invented as frontier. A tag that does not match `path_counts` refuses before the edit file is rewritten.
- Hole: `propose_enrich` swallowed a failed proposal INDEX rewrite, and an unreadable proposal was still listed by name. The rewrite is now an error, and the index line lists `drivers=`.
- Schema freeze: `source_drivers` stays additive on pack, specialist-pack, and enrich-proposal v0. Only `frontier` and `local`. Missing field defaults to `[]`. A rename is a v1.
- No new CLI. Smoke and gate-90 unchanged.
- `READY_FOR_LIVE_TEST`: no.

## After PR #35 (this slice)

- Pack INDEX and `estate feed list` / `estate packs list` print `drivers=` from `source_drivers` (`frontier,local` when both are present, `-` when empty). Empty packs do not invent a source.
- Hole: `write_drop_pack`, import, and `feed list` swallowed a failed INDEX rewrite. A failed rewrite is now an error.
- `make feed-loop` greps INDEX for `drivers=frontier,local`. Did not add `make feed-loop-mixed`; the existing walk already tags mixed frontier and local traces. Still off smoke and gate-90. No new CLI.
- `READY_FOR_LIVE_TEST`: no.

## After PR #34 (this slice)

- `estate specialist --driver frontier` refuse text names `CELL_FRONTIER_MODEL` and `CELL_FRONTIER_ENDPOINT`. A missing `XAI_API_KEY` already did. A hardware SKU in the model id now says whether it came from `CELL_FRONTIER_MODEL`, `XAI_MODEL`, or a binding model param, and still refuses before POST.
- `docs/GATE-90.md` and `docs/DAY90-PLUS.md` split green factory checks, recorded live proofs, and parked rows. Mac specialist complete is optional and not recorded. Native MLX stays a stub. Cloud-spawn stays off. The recorded 5090 proof is not called parked.
- CELL-ONE-STATUS states #32–#34 in plain English.
- `READY_FOR_LIVE_TEST`: no.

## After PR #33 (this slice)

- `make feed-loop` asserts the produced pack `source_drivers` is `frontier` then `local`, counts are non-zero, and propose copies the tag. Live keys are unset. Still off smoke and Actions.
- `estate help frontier` names `grok-4.7`, the `XAI_API_KEY` gate, and that local down does not POST frontier. `estate help day90-mixed` stays opt-in.
- Hole: `run_task` swallowed a failed feed append and could still complete. A failed audit now refuses before frontier.
- `READY_FOR_LIVE_TEST`: no.

## After PR #32 (this slice)

- Feed packs tag `source_drivers` (`frontier` and/or `local`) from the events. The tag must match `path_counts`. Unknown drivers, a tag with a zero count, and a count with no tag refuse.
- Explicit `object_class: local` stays local even when the kind or note mentions frontier. Local-down does not tag frontier.
- Propose copies `source_drivers` onto the diff. `auto_apply` stays false. Promote stays off. The estate file is not rewritten.
- `READY_FOR_LIVE_TEST`: no.

## After PR #31 (this slice)

- `make day90-mixed` walks the mixed fixture: status → plan → `apply --require-plan` → status → doctor. Isolated cell. No live key. Not in smoke or Actions.
- `estate status` prints `frontier: <id> model=…` only when the binding sets `params.model`. Catalog lines print `grok-4.7` when `frontier.model` is in the schema catalog or the cell `catalog.json`. Doctor prints the same. A SKU model fails doctor.
- The default estate binding has no model param, so status does not invent `frontier: xai_grok model=grok-4.7`.
- `READY_FOR_LIVE_TEST`: no.

## After PR #30 (this slice)

- Mixed fixture operator path: `estate plan` then `estate apply --require-plan` writes model-actual, placement-actual, and catalog. Mock only. No `XAI_API_KEY`. No frontier POST. Local `ollama` specialist after apply still skips frontier.
- Catalog file SoT sibling card: model `grok-4.7`, streaming/tools/vision false, completion budget 64. Not a local probe. Not a context window.
- Requested local specialist does not POST frontier: `ollama` up, `http-remote` up, `llama.cpp` down, `mlx` / `vllm` / `trt` refuse.
- `READY_FOR_LIVE_TEST`: no.

## After PR #29 (this slice)

- Recorded frontier specialist live PASS: `--driver frontier`, model `grok-4.7`, `completion` `pong`, reason `frontier completion`. Key never printed. Env-gated `XAI_API_KEY`. No box hostname.
- `READY_FOR_LIVE_TEST`: no for that surface.
- Mixed fixture `frontier_http` (`http-remote`, model `grok-4.7`) + local `ollama`: `validate` and `apply --dry-run` stay green with no live key and no POST.
- Local specialist (`--driver ollama`) down or unset does not POST to frontier even when `XAI_API_KEY` and `CELL_FRONTIER_ENDPOINT` are set.

## After PR #28 (this slice)

- Frontier model id is **`grok-4.7`** (`CELL_FRONTIER_MODEL` or `XAI_MODEL`). The old `grok-3-mini` default is gone.
- `estate specialist --driver frontier` requires `XAI_API_KEY`. Optional `CELL_FRONTIER_ENDPOINT` (default `https://api.x.ai/v1`). Unset key refuses. Sacred and SKU refuse before POST. Mock-locked. No key in CI.
- `--driver http-remote` stays the local `CELL_LOCAL_ENDPOINT` card. Local down does not fall through to frontier.
- Cloud-agent standing default `reasoning_effort` xhigh is documented only. The factory chat POST sends `grok-4.7`.
- `READY_FOR_LIVE_TEST`: yes. One command with a real `XAI_API_KEY`.

## After PR #27 (this slice)

- `--driver frontier` is env-gated on `CELL_FRONTIER_ENDPOINT` (or `--endpoint`). `CELL_LOCAL_ENDPOINT` and `XAI_API_KEY` do not unlock it. Mock-locked OpenAI chat. Not in CI / smoke.
- Operator fixture: `estate apply` records `local_slm` in `model-actual.json`, then `estate specialist` complete against mock-local.
- Driver is resolved before the local endpoint, so `--driver frontier` no longer dies as a missing `CELL_LOCAL_ENDPOINT`.
- `READY_FOR_LIVE_TEST`: no. Mac specialist is the same Ollama complete already proven on 5090.

## After PR #26 (this slice)

- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) records Mac `probes --live` PASS, 5090 `probes --live` PASS, and 5090 `estate specialist` `completion` `Pong`. Native MLX stays stub.
- `make live-specialist` requires `CELL_LOCAL_ENDPOINT` (SKU endpoint refuses). Not in `make smoke` / `make gate-90` / Actions.
- Mixed-estate apply `--dry-run` stays write-free; `local_slm` binds `HttpLocal` against in-process mock (`/v0/specialist`).
- Apply / resume no longer swallow `catalog.json` write failure.
- `READY_FOR_LIVE_TEST`: no. Recorded surfaces already ran. Mac specialist chat and live Grok are still unrecorded.

## After PR #25 (this slice)

- OpenAI `/v1/chat/completions` with empty / missing / whitespace `message.content` falls through to Ollama `/api/chat` (same as probes try both shapes).
- Accepts content-array parts, `text`, or reasoning-only when the text is clearly non-empty.
- Both chat paths fail: HTTP status, model id used, `ollama pull llama3` / `CELL_LOCAL_MODEL`.
- Unset `CELL_LOCAL_MODEL` prefers first `/api/tags` id, then `/v1/models`.
- `READY_FOR_LIVE_TEST`: yes. 5090 retry of `estate specialist --driver ollama --prompt "Reply with the single word pong."`

## After PR #24 (this slice)

- `estate specialist --driver ollama --prompt` is a thin `HttpLocal` delegate. Default job `complete` returns model `completion`. Same helper as `model-estate specialist --job complete`.
- Sacred / empty / SKU refuse before any HTTP POST. Empty `message.content` refuses. A bad OpenAI body still does not try Ollama.
- Mock-local complete is `mock:{text}`. Compat OpenAI / Ollama complete is the model body (`ok` in-process).
- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) has the exact Mac Ollama command.
- `READY_FOR_LIVE_TEST` for this complete verb: yes. Jason already PASSed `probes --live`.

## After PR #23 (PR #24 specialist round-trip)

- Specialist chat posts the real request text (not dummy `ping`) through `HttpLocal` against mock HTTP. Last-POST capture locks it.
- `model-estate specialist` was the data-plane equivalent before `estate specialist` existed.
- llama.cpp server OpenAI path smoke: same adapter, `HttpLocal { runtime: LlamaCpp }` + CLI `--runtime llama.cpp`.
- Fail-closed: v0 200 unparseable refuses (no compat fall-through); OpenAI choices require `message.content`; SKU `CELL_LOCAL_MODEL` / listed model ids refuse.
- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) now has exact SKIP / live ok / down / specialist JSON lines.
- `READY_FOR_LIVE_TEST` for that policy-precheck verb: no. Do not ping Jason.

## After PR #22 (PR #23 adapter)

- Live probes GET `/v1/models` or Ollama `/api/tags`. Empty models list is up. Garbage / empty body is down. No invent success.
- `HttpLocal` adapter: factory `/v0/specialist`, then OpenAI chat / Ollama chat, then factory-owned policy. Native MLX `specialist()` stays stub. Mac proof is Ollama-on-Mac.
- [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md) is copy-paste for Jason Mac and Jason Linux/5090. In-process mock HTTP locks the adapter. CI stays SKIP without env.

## After PR #21 (this slice)

- Hole: `record_bindings` used `unwrap_or_default` and could wipe `model-actual.json`. Isolation `session.json` was the same class. Serialize or refuse. Garbage model-actual is refuse, not "run apply".
- Hole: accept / import / propose compared estate bytes with `unwrap_or_default`, so an unreadable file looked unchanged. `read_estate_text` refuses.
- Live probe hand-off: [`docs/LIVE-PROBES.md`](docs/LIVE-PROBES.md). Dry fixture `schema/live-probe-shapes.v0.json` locks SKIP vs would-live without network. Not a live-box proof. Do not ping Jason yet.

## After PR #20 (this slice)

- Hole: `propose_enrich` used `unwrap_or_default` and could write an empty proposal. Serialize or refuse. No empty `.proposal.json`.
- Hole: `append_event` invented `"{}"` on serialize failure and could append junk to `events.jsonl`. Journal / audit lines serialize or refuse. Empty object is refuse.
- Hole: `write_placements` used `unwrap_or_default` and could wipe `placement-actual.json` empty. Same class closed on lifecycle, session journal, apply-audit, reconcile, backup meta, actual-state, mesh persist, and accept enrich-edit.
- CELL-ONE-STATUS records #20 and this hunt. Isolated locks stay crate tests plus `tests/day90_honesty.rs`.

## After PR #19 (this slice)

- Hole: `covering_plan` / `latest_plan` / `list_plans` treated unreadable plan JSON as empty. `apply --require-plan` could say "no plan"; `status` could invent last-plan. Present plan JSON parses or refuses. APIs return `Result`.
- Hole: `write_cursor` used `unwrap_or_default` and could write empty `feed-cursor.json`. A present garbage cursor could look missing. Load: missing → none; exists but not a file or parse fail → refuse. Write serializes or refuses.
- Hole: `apply` without `--force` could still walk a SKU `placement-actual`. `--force` now claims estate `host_class` (`rtx_consumer` → `consumer-nvidia`) and does not launder the SKU to `any`. Without `--force`, `refuse:bad-host-class` before dry-run.
- Remaining `.ok()` on Path-exists loads in estate-control / floor / conveyor / feed are listing skips, not SoT. `load_placements` stays permissive so `--force` can overwrite.
- CELL-ONE-STATUS records #19 and this hunt. Isolated lock stays `tests/day90_honesty.rs`.

## After PR #18 (this slice)

- Hole: `suspend` / `resume` / `apply` used `load_lifecycle(...).ok()`. A present-but-unreadable `lifecycle.json` was treated as greenfield and overwritten. Parse or refuse. Apply refuses before lease writes.
- Hole: Feed import wrote the accepted pack, then swallowed `append_import_audit` (`let _ =`). Audit append is fail-closed. Serialize no longer invents an empty line.
- Hole: `apply --dry-run --import-pack --curator robot` skipped the curator check. `refuse_import_pack` runs first. Live and dry-run write no leases, no accepted pack, no audit.
- `estate catalog` writes first, then prints. `estate probes` refuses every card first, then prints.
- Status no longer invents `expired=0` / empty proposals on reader failure. Plan `export-pr` no longer invents an empty expired list.
- CELL-ONE-STATUS records #18 and this hunt. Isolated lock stays `tests/day90_honesty.rs`.

## #15–#18 (SKU then honesty)

- #15: `convey sync` slim-parse laundered a SKU `host_class` to portable `any`.
- #16: `convey call` swallowed that refuse; floor / status / leases / reconcile stayed silent.
- #17: mesh readers and restore still copied a SKU. Production `canonical_host_class` callers gone.
- #18: print-then-refuse on leases; backup/restore estate `.ok()`; suspend swallowed actual-state / journal; feed redaction write was `let _ =`.

## After PR #17 (this slice)

- Hole: `estate leases` printed the SKU `placement-actual` JSON, then refused. Readers now refuse first. Status already refused before print; tests lock both.
- Hole: `backup` / `restore` swallowed a present-but-unreadable estate file (`load_estate(...).ok()`), so restore could invent a locked-only sacred set. A file that exists must parse or refuse.
- Hole: `suspend` swallowed `actual-state.json` parse errors and journal write failures, so unspawn lines could vanish. Load and journal fail closed.
- Feed import: `{id}.redaction.json` write is no longer `let _ =`. Report bytes are kind counts only; a raw secret in the report is `refuse:raw-secret`. Hand-written dirty packs write no accepted pack and no report.
- `lifecycle.jsonl` / `sessions.jsonl` stay append-only under suspend / resume / `expire --forget`. Forget does not truncate journals.
- Hole leftover from #17: floor `src` tests used `rtx-5090`, so `doctor --strict` failed the vendor-needle scan. Fixtures now use `not-a-host`.
- Sacred / curator / policy refuse paths had no new silent `Ok()` swallows.
- CELL-ONE-STATUS states the #15–#17 SKU launder story in plain English.
- Isolated lock: `tests/day90_honesty.rs`.

## After PR #16 (this slice)

- Hole: `convey call` / `list` / `expire` / `sync` loaded a tampered `conveyor-mesh.json` without checking `host_class`. A SKU hop/lease could allow a call or get written back. Readers now `refuse:bad-host-class` and write nothing.
- Hole: `restore` copied a SKU `placement-actual` (or mesh) onto disk. Restore is now `refuse:bad-host-class` (dry-run and live). Does not invent `any`.
- Remaining `canonical_host_class` is the trusted unwrap only. Hop declare stamps via opt after `refuse_hop`.
- Isolated lock: `tests/day90_mesh.rs`.

## After PR #15 (this slice)

- Hole: `convey call` swallowed slim-parse `refuse:bad-host-class`, so a tampered SKU `host_class` skipped the placement not-live check. Call now fails closed. Status / leases / reconcile use the same refuse. `record_placements` writes nothing; claim does not rewrite the SKU to `any`.
- Validate on multi-host + mixed fixtures stays green. Probe / catalog card ids still refuse SKUs the same way hop ids do.
- Isolated lock: `tests/day90_sku.rs`.

## After PR #14 (this slice)

- Hole: `convey sync` rewrote unknown / SKU `host_class` on a tampered `placement-actual.json` to `any` and seeded a hop. Slim-parse now `refuse:bad-host-class` and writes no mesh files. Alias round-trips stay (`rtx_consumer` → `consumer-nvidia`).
- Overlay with `locked: []` still cannot drop Cyera CI / Rust classroom (property lock).
- Two `apply --dry-run` after a real apply leave an identical `.cell` tree. Dual-layer-demo backup → restore with matching sacred writes the leases back.
- Makefile contract: `gate-90` / `smoke` / `day90` / `feed-loop` / `fixtures-check` / `doctor-strict` exist; `gate-90` does not invoke `gh`.

## Still stubbed

MLX stays a stub. vLLM and TRT stay experimental catalog cards, not live-ok. Cloud-agent spawn. Convey hop transport (lease-bound mesh only). Auto-promote. Curator UI. See `docs/DAY90-PLUS.md`.

## How to run

```bash
make gate-90    # Day-90 operator entrypoint (local)
make smoke      # doctor + fixtures-check + operator-day + cargo test + make day90
make day90      # operator loop only
make feed-loop  # scrubbed trace → pack → propose → accept (fixtures only)
estate help     # Day-90 topic pages
# walk: docs/OPERATOR-DAY.md
# snapshot: docs/CELL-ONE-STATUS.md
```

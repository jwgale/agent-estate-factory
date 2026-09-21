# Host-class matrix

Locked names stay `consumer-nvidia` | `apple-silicon` | `rented-nvidia` | `any`.
These fixtures use the operator aliases (`rtx_consumer`, `apple_silicon`, `nvidia_rental`) so normalize stays swap-proof.

| File | Alias on disk | Canonical |
| --- | --- | --- |
| `rtx-consumer.yaml` | `rtx_consumer` | `consumer-nvidia` |
| `apple-silicon.yaml` | `apple_silicon` | `apple-silicon` |
| `nvidia-rental.yaml` | `nvidia_rental` | `rented-nvidia` |

MLX stays a catalog stub (Apple card). These estates still bind `ollama` — Ollama-on-Mac is the Supported Apple path. Hardware is a driver choice, not a product fork.

`frontier-http.yaml` is not a fourth host class. It names `model: grok-4.7` on a frontier `http-remote` binding, with a local `ollama` card so the estate is not frontier-only. `examples/estate.yaml` stays hash-locked and does not gain that field. The file is not on the smoke or gate-90 walks.

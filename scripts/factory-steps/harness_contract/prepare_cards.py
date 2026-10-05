"""Preference and refusal prepare cards. Rank 12. They write files. They do not train."""

from __future__ import annotations

from pathlib import Path

from .common import now_iso, one_sentence, write_json


def write_preference(out: Path, rows: list[dict], stage: str = "dpo") -> dict:
    out.mkdir(parents=True, exist_ok=True)
    data = out / "dataset.jsonl"
    with open(data, "w", encoding="utf-8") as f:
        import json
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    recipe = out / "recipe.yaml"
    recipe.write_text(
        "\n".join([
            "# cell-one preference prepare card",
            "# auto_train: false",
            "# This file does not train. do_train is false.",
            f"stage: {stage}",
            "do_train: false",
            "finetuning_type: lora",
            "dataset: preference",
            "",
        ]),
        encoding="utf-8",
    )
    card = {
        "schema": "cell-one.harness-prepare.v0",
        "kind": "preference",
        "stage": stage,
        "auto_train": False,
        "train_invoked": False,
        "dataset": str(data),
        "recipe": str(recipe),
        "rows": len(rows),
        "at": now_iso(),
        "reason": one_sentence(
            "Wrote a preference prepare card. auto_train is false. No trainer was started."
        ),
    }
    write_json(out / "prepare.json", card)
    (out / "PREPARE.md").write_text(
        card["reason"] + "\n\nThe recipe names the preference stage. It does not launch a train.\n",
        encoding="utf-8",
    )
    return card


def write_refusal(out: Path, rows: list[dict], marker: str) -> dict:
    out.mkdir(parents=True, exist_ok=True)
    data = out / "refusal.jsonl"
    import json
    with open(data, "w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")
    rule = {
        "schema": "cell-one.harness-gap-rule.v0",
        "refusal_marker": marker,
        "target_field": "target",
        "rule": (
            "Refuse a train if a refusal row is mixed into the task set "
            "or a task line is mixed into the refusal set."
        ),
    }
    write_json(out / "gap-rule.json", rule)
    card = {
        "schema": "cell-one.harness-prepare.v0",
        "kind": "refusal",
        "auto_train": False,
        "train_invoked": False,
        "dataset": str(data),
        "marker": marker,
        "rows": len(rows),
        "at": now_iso(),
        "reason": one_sentence(
            "Wrote a refusal set in its own file. auto_train is false. No trainer was started."
        ),
    }
    write_json(out / "prepare.json", card)
    (out / "PREPARE.md").write_text(
        card["reason"] + "\n\n" + rule["rule"] + "\n",
        encoding="utf-8",
    )
    return card

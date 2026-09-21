# Reviewed plans

Copy a blast-radius markdown here (`estate plan --reviewed`) when a human should PR-review apply.

Generated `plan-*.md` / `.json` / `.security.md` stay gitignored under `plans/`. Commit a file in this directory when Security-as-IaC needs a review surface.

`estate apply --require-fresh-plan` refuses a greenfield plan after an apply. Take a new plan against the last snapshot first.

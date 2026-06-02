## Agent skills

### Issue tracker

Issues are tracked in GitHub Issues for this repo. See `docs/agents/issue-tracker.md`.

### Triage labels

Default label vocabulary (needs-triage, needs-info, ready-for-agent, ready-for-human, wontfix). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout — three layers: CONTEXT.md (ubiquitous language) + docs/ecology/ (domain) + docs/system-design/ (the design, which is self-justifying — no separate decision-record layer). See `docs/agents/domain.md`.

### Workspace architecture

How the crates divide responsibility (sim = pure deterministic stepper; genesis/search own parameter search; app = debugging instrument). Read before deciding where a change belongs. See `docs/agents/architecture.md`.

## Code style

Do **not** run `cargo fmt` (even `-p <crate>`). The committed code is not rustfmt-clean, so `cargo fmt` reformats large swaths of pre-existing, untouched code and turns a focused diff into thousands of lines of unrelated churn (`cargo fmt --check` on HEAD reports diffs — fmt is not enforced in CI). Match the surrounding code's style by hand; if you tidy formatting, do it only within the lines you're already editing.

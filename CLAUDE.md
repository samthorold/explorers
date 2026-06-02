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

The committed code is rustfmt-clean (`cargo fmt --check` passes on HEAD). Run `cargo fmt` freely and keep it clean — it no longer produces unrelated churn, so a focused diff stays focused. fmt is not enforced in CI, so it's on you to run it before committing.

# Delivery Workflow

How work moves from a design question to merged code in this repo.

## The pipeline

For an epic: `grill-with-docs` (settle the design against `CONTEXT.md`, `docs/ecology/` and `docs/system-design/`, updating the docs inline) → `to-issues` (tracer-bullet vertical slices, labelled `ready-for-agent`) → `tdd` per slice.

- **One branch and one PR per slice.** Squash-merge with `--delete-branch`, and close the issue referencing the PR. The repo's gitignored `.claude/settings.local.json` allows the git/gh commands, so the flow runs without prompts.
- **TDD strictly red → green,** one behaviour per cycle. Surface design ambiguity the moment it appears rather than guessing.
- **Checkpoint at natural boundaries** (for example halfway through an epic) with a concise status and a default to continue, not a stop-and-ask. The owner delegates heavily and wants momentum with visibility, not permission-seeking at every step.
- **Keep new code `cargo fmt` clean** (fmt is not enforced in CI). Clippy is not enforced either; don't churn working code over style nits.
- **Don't auto-merge when a headline acceptance is empirically unverified.** Leave the PR open and name the gap. Test-covered logic is fine to merge.

## Durable knowledge lives in the repo

Anything worth remembering across sessions — workflow, runtimes, known traps, project vision — is written into committed docs (`CLAUDE.md`, `docs/agents/`, `CONTEXT.md`, `docs/system-design/`), never only in an agent's private memory. Private memory is not under source control and drifts out of date unseen.

# Domain Docs

How the engineering skills should consume this repo's documentation when exploring the codebase.

The project's knowledge lives in three layers: the **domain** (the true facts and dynamics of ecology), the **system design** (our opinion about how to model it, which any implementation must encode), and the **implementation** (the code). There is no decision-record layer — the reasons the design has its shape are written into the system-design docs themselves, in the present tense.

## Before exploring, read these

- **`CONTEXT.md`** at the repo root — the ubiquitous language (or **`CONTEXT-MAP.md`** if it exists, pointing at one `CONTEXT.md` per context; read each one relevant to the topic).
- **`docs/ecology/`** — the domain ground truth for the area you're working in.
- **`docs/system-design/`** — the design that governs the area you're about to work in. This is where mechanisms, functional forms, and the reasons for them live; read the documents that touch your area before changing code.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The producer skill (`/grill-with-docs`) creates them lazily when terms or design actually get resolved.

## File structure

Single-context repo (most repos):

```
/
├── CONTEXT.md                ← ubiquitous language
├── docs/
│   ├── ecology/              ← domain ground truth
│   └── system-design/        ← the design (self-justifying)
└── src/                       ← implementation (the code)
```

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/grill-with-docs`).

## Flag design conflicts

If your output contradicts the documented system design, surface it explicitly rather than silently overriding:

> _Contradicts the embodiment rule in world-rules.md (nutrient is bound into structure) — but worth reopening because…_

If the design doc is silent on *why* it has its current shape and you need that to proceed, that's a gap in the system-design layer, not a missing decision record — note it for `/grill-with-docs` so the rationale gets written into the design doc itself.

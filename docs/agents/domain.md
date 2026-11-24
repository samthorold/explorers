# Domain Docs

The project's knowledge follows the Project knowledge Guide's Stance *three layers, written in one language* (`~/.claude/commons/guides/project-knowledge.md`, and its detail file `~/.claude/commons/guides/project-knowledge/three-layers.md`), with its Stances that the system design justifies itself and that there is no decision-record layer. Here the layers live at:

```
/
├── CONTEXT.md                ← ubiquitous language (single context)
├── docs/
│   ├── ecology/              ← domain: the facts and dynamics of ecology
│   ├── system-design/        ← system design (self-justifying), with the end state and its register
│   └── research/             ← measurements and world-specific analysis, one note per issue
└── crates/                   ← implementation: the explorers-* workspace crates
```

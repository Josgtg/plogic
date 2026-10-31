# AGENTS.md

Instructions for AI agents working in this repository. Human-facing documentation lives
in [README.md](README.md); the design lives in [design/](design/).

## What this project is

plogic evaluates propositions of propositional logic and builds truth tables. It follows
the algorithms of [Crafting Interpreters](https://craftinginterpreters.com/) by Robert
Nystrom, in a functional style rather than the book's OOP.

Current state: design phase. The design is complete; the Rust implementation is a stub
(`src/main.rs`). Implement against `design/`, not from memory of the book.

## How to help me design

**When helping me design, any ambiguous or complicated thing — ASK ME.** Do not guess
design decisions. Ask one focused question with concrete options (the question tool is
fine), then proceed. It is expected to stop mid-task for this. Always ask, don't be
afraid to do it. You asking 100 times is better than you taking a single significant
design decision by yourself.

## Sources of truth

- [design/product_description.md](design/product_description.md) — the product and the
  language. Everything else must agree with it.
- [design/decisions.md](design/decisions.md) — why each settled decision is what it is.
  Append a new record instead of rewriting an accepted one.
- [design/testing.md](design/testing.md) — what has to be verified per stage.

AGENTS.md does not restate design decisions. If a document and an accepted decision
disagree, fix the document and say so.

When editing the design, keep the documents congruent: a change in the language, the
error model, a signature or a flag usually touches several of them.

## Design constraints

These hold for every change; they are decided, not open for reinterpretation:

- **Functional core**: static entry points (`Scanner.scan`, `Parser.parse`,
  `Simplifier.simplify`, `Interpreter.interpret`, `Orchestrator.evaluate`), no
  constructors, no mutable state. State is threaded as immutable values
  (`ScanState`, `ParseState`) and returned anew.
- **Errors are values**: every fallible operation returns `Result<T>`; `Err` carries
  `List<Error>`. No exceptions, no panics, no error sink, no flags. Stages recover
  maximally inside themselves and gate hard between stages.
- **Purity**: only the UI/CLI performs I/O. The Orchestrator and every stage are pure.
- **User-facing text is Spanish**; messages are lower-case and say what was expected.
  Positions are 1-based.
- **Rust**: edition 2024. Do not add dependencies without asking. Keep `cargo clippy`
  and `cargo fmt` clean.

## Commands

```sh
cargo build                      # compile
cargo test                       # unit + integration tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
plantuml -checkonly design/diagrams/*.puml   # after editing diagrams
bash .opencode/update_ai_artifacts.sh        # after code, docs or config changes
```

## Where things live

| Path | Contents |
| --- | --- |
| `src/main.rs` | CLI entry point (the only impure code) |
| `design/` | design documents; `design/diagrams/` holds the PlantUML sources |
| `.opencode/` | generated AI artifacts; never edit by hand, refresh instead |

## Git workflow

- One coherent change per commit. Design documents, code and tooling stay in
  separate commits unless a change genuinely spans them.
- Stage explicit paths. Never `git add -A` or `git add .`: the working tree holds
  work in progress (the diagrams) and generated artifacts.
- Do not commit `design/diagrams/` until the diagrams match the documents.
  Generated artifacts under `.opencode/` (`tags`, `repomix-output.md`) stay
  untracked.
- Refresh artifacts before committing a change to code, documents or
  configuration: `bash .opencode/update_ai_artifacts.sh`.
- Messages are English and imperative, one short subject line, e.g.
  `design: add product and pipeline documents`. Add a body only when the why is
  not obvious from the diff.
- Never commit, amend or force-push unless asked, and never rewrite published
  history.

Implementation order follows the pipeline and the test plan: tokens → scanner → parser →
simplifier → interpreter → truth table → orchestrator → UI. Land each stage with its
tests from `design/testing.md`.

## Diagrams

`design/diagrams/class_diagram.puml`, `relationships_diagram.puml`,
`components_diagram.puml` and the per-class files describe the design. Update them when a
signature or relationship changes, and verify with `plantuml -checkonly`. Do not
restructure them unless asked: they are known to be behind the documents
(`Xor`/`Nand`/`Nor`, `Variable`, `evaluate` without `simplify`, no `InfixPrinter`).

# AGENTS.md

Instructions for AI agents working in this repository. Human-facing documentation lives
in [README.md](README.md); the design lives in [design/](design/).

## What this project is

plogic evaluates propositions of propositional logic and builds truth tables. It follows
the algorithms of [Crafting Interpreters](https://craftinginterpreters.com/) by Robert
Nystrom, in a functional style rather than the book's OOP.

Current state: the full pipeline is implemented. The CLI argument parser (`src/args.rs`),
the token vocabulary (`src/tokens.rs`), the shared error type (`src/error.rs`), the
scanner (`src/scanner.rs`), the abstract syntax tree with its two printers
(`src/expression.rs`), the parser (`src/parser.rs`), the simplifier
(`src/simplifier.rs`), the interpreter with the truth table (`src/interpreter.rs`,
`src/truth_table.rs`), the orchestrator (`src/orchestrator.rs`) and the wired CLI
(`src/cli.rs`) are all in place. Implement against `design/`: the book supplies the
algorithms, but the design documents win wherever the two differ.

## How to help me design

**When helping me design, any ambiguous or complicated thing — ASK ME.** Do not guess
design decisions. Ask one focused question with concrete options (the question tool is
fine), then proceed. It is expected to stop mid-task for this. Always ask, don't be
afraid to do it. You asking 100 times is better than you taking a single significant
design decision by yourself.

## Tests

Do not write tests yourself. Once the code compiles, hand the test plan from
`design/testing.md` to the `test-writer` subagent: tests written from the design, by
someone who did not write the code, catch assumptions that the implementation and its own
tests would share. If a test disagrees with the code, settle the design question first —
in `design/decisions.md` — and change whichever side the decision says is wrong.

## Sources of truth

- [design/product_description.md](design/product_description.md) — the product and the
  language. Everything else must agree with it.
- [design/decisions.md](design/decisions.md) — why each settled decision is what it is.
  Append a new record instead of rewriting an accepted one.
- [design/testing.md](design/testing.md) — what has to be verified per stage.

AGENTS.md does not record design decisions; it points to the log. The invariants below
are binding for every change, and their rationale lives in `decisions.md`. If a document
and an accepted decision disagree, fix the document and say so.

When editing the design, keep the documents congruent: a change in the language, the
error model, a signature or a flag usually touches several of them.

## Design constraints

These hold for every change; they are decided, not open for reinterpretation:

- **Functional core**: static entry points (`Scanner.scan`, `Parser.parse`,
  `Simplifier.simplify`, `Interpreter.interpret`, `Orchestrator.evaluate`), no stage
  constructors, no mutable state. State is threaded as immutable values
  (`ScanState`, `ParseState`) and returned anew.
- **Errors are values**: fallible stage entry points (`Scanner.scan`, `Parser.parse`,
  `Interpreter.interpret`, `Orchestrator.evaluate`) return `Result<T, List<Error>>`;
  `Simplifier.simplify` is total and returns `Expression`. Internal helpers return
  `Result<T, Error>`; parser rules and their `consume` helper are the exceptions,
  returning `Result<T, (Error, ParseState)>` so recovery resumes from the failing
  token, and `TokenType.map` returns `Result<TokenType, str>`. The stage collects
  helper errors.
  No exceptions, no panics, no error sink, no flags. Stages recover maximally inside
  themselves and gate hard between stages.
- **Purity**: only the UI/CLI performs I/O. The Orchestrator and every stage are pure.
- **User-facing text is Spanish**; error messages and prompts are lower-case and say
  what was expected; result labels keep normal capitalization. Positions are 1-based.
- **Rust**: edition 2024. Do not add dependencies without asking. Keep `cargo clippy`
  and `cargo fmt` clean.

## Tech stack

The runtime stack is deliberately small:

- **clap 4** parses the command line, renders the help screen and handles
  `-h`/`--help` and `-V`/`--version`, through the `derive` feature in
  `src/args.rs`: each argument's doc comment is its help line,
  `help_heading` picks the section, and `about`/`override_usage`/`help_template`
  supply the Spanish words (ADR-032); the two informational flags print when
  clap reaches them (ADR-033). plogic still owns the exact Spanish usage-error
  messages and the fixed precedence in ADR-022/ADR-027/ADR-031; a usage error
  prints its message and then the same help screen (ADR-034), and clap's error
  text is never user-facing. The version stays at 4.6.4 because `clap_derive`
  has no 4.6.7 release.
- **thiserror 2** derives `Display` and `std::error::Error` for the typed `Error`
  enum. There is no anyhow: the core keeps `Result<T, List<Error>>`, and file I/O
  failures stay `std::io::Error` inside the CLI, which renders the Spanish message
  and exits 1.
- **proptest 1** is a dev-dependency for the generated-expression tests in
  `design/testing.md`.
- **colored 3** colors the truth-table cells (`V` green, `F` red) and the
  message of an error (bright red, on a terminal only; ADR-035). Formulas,
  labels, classification, the help screen and the output written to a file stay
  plain.

Any other dependency needs a decision first.

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
| `src/tokens.rs` | the token vocabulary: `TokenType`, `Token` and `TokenType::map` |
| `src/error.rs` | the shared `Error` enum and its `Display` |
| `src/scanner.rs` | the scanner: source text to tokens |
| `src/expression.rs` | the AST: `Expression`, `ASTPrinter` and `InfixPrinter` |
| `src/parser.rs` | the parser: tokens to one `Expression` per proposition |
| `src/simplifier.rs` | the simplifier: canonical rewrites to an equivalent expression |
| `src/interpreter.rs` | the interpreter: evaluation and truth-table construction |
| `src/truth_table.rs` | the `TruthTable`, `Row` and `Classification` result types |
| `src/orchestrator.rs` | the pipeline driver: `evaluate`, `simplify` and `report` |
| `src/cli.rs` | the UI/CLI: I/O, presentation, error colors |
| `src/args.rs` | argument parsing: `Args`, `parse`, `Config` and `UsageError` |
| `design/` | design documents; `design/diagrams/` holds the PlantUML sources |
| `.opencode/` | generated AI artifacts; never edit by hand, refresh instead |

## Git workflow

- One coherent change per commit. Design documents, code and tooling stay in
  separate commits unless a change genuinely spans them.
- Stage explicit paths. Never `git add -A` or `git add .`: the working tree may hold
  work in progress and generated artifacts.
- Commit `design/diagrams/` only when the diagrams match the documents. Generated
  artifacts under `.opencode/` (`tags`, `repomix-output.md`) stay untracked.
- Refresh artifacts before committing a change to code, documents or
  configuration: `bash .opencode/update_ai_artifacts.sh`.
- Messages are English and imperative, one short subject line, e.g.
  `design: add product and pipeline documents`. Add a body only when the why is
  not obvious from the diff.
- Never commit, amend or force-push unless asked, and never rewrite published
  history.

Implementation order follows the pipeline and the test plan: tokens → scanner → parser →
simplifier → interpreter → truth-table construction → orchestrator → UI. Land each stage
with its tests from `design/testing.md`.

## Diagrams

`design/diagrams/` holds the PlantUML sources: `class_diagram.puml`,
`relationships_diagram.puml`, `components_diagram.puml`, the per-stage files
`scanner_class.puml`, `parser_class.puml`, `simplifier_class.puml`,
`interpreter_class.puml`, `orchestrator_class.puml` and `ui_class.puml`, plus
`expression_class.puml` and `truth_table_class.puml` (a result type, not a stage). They
mirror the design documents; update them when a signature or relationship changes, and
verify with `plantuml -checkonly`.

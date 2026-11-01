# Plogic - GUI design

Role: the graphical window. It is a second impure UI component, next to the
CLI in [ui.md](ui.md): it drives the Orchestrator, builds result components
from the assessments it returns, and performs the window I/O. The pipeline
stays pure and is not duplicated. The GUI evaluates one proposition at a time.

## Entry point

`main` collects the arguments and asks `gui::should_launch`, which is true only
when the list is empty. Empty means exactly no arguments: `plogic -p p`,
`plogic --`, `plogic -h` and `plogic ""` all keep the CLI. A bare invocation no
longer prints the `no se proporcionó ninguna proposición` usage error;
`args::parse` still returns it if called with an empty list, but the binary
never calls it there.

`gui::run` returns an `eframe` result. If the window cannot open, the binary
prints `no se pudo iniciar la interfaz gráfica` on stderr and exits 1. The
underlying backend error is not user-facing.

## Window

A single native window, 860x880 by default and at least 620x620, titled
`plogic`. It is dark and monospace, fixed to the mockup's theme. One centered
column, at most 760 points wide, holds the content. The header and the input
section are fixed; only the result section scrolls, so the controls never move
when a result appears.

From top to bottom:

1. The header: the title `plogic` and its one-sentence description, then a
   collapsible `Ayuda` section, closed by default. Opening it shows a
   one column set with the columns `Símbolo`, `Nombre` and `También` at
   12.5 points (the body is 15 points), followed by `Usa paréntesis para
   agrupar.`. The list has one row per connective or literal: `¬`, `∧`, `∨`,
   `→`, `↔`, `V`, `F`.
2. `Proposición`: a heading with the `Enter para evaluar` hint, the single-line
   formula field (hint `p | q & s -> t`), the insertion toolbar, the three
   controls and the full-width `Evaluar` button, in that order.
3. `Resultado`: the section title and, in the only scroll area, the centered
   result components. With no result the section shows the muted line
   `El resultado aparecerá aquí.`.

The toolbar inserts `¬`, `∧`, `∨`, `→`, `↔`, `(`, `)` at the cursor, replacing
a selection; `variable` inserts the next single-letter variable from `p` to `z`
that the current input does not use, skipping `y` (the conjunction word, which
can never be an identifier; `p` when every usable letter is taken or the
formula does not scan); `Limpiar` clears the input and its cursor, never the
stored result. Every button has a hover description in Spanish.

## Controls

- `Simplificar` (default on): the next evaluation uses the simplified
  expression; unchecked, it uses the original.
- `Mostrar tabla` (default on): the next evaluation builds a table and shows
  its classification. Off, the table is not processed at all.
- `Mostrar proceso` (default off): the next evaluation keeps and shows its
  process card when it has at least one step.

The controls are independent. There is no separate `no simplificar` control:
unchecking `Simplificar` is how the GUI reaches that mode. All three are read
once, when `Evaluar` (or Enter) is pressed; changing a control afterwards only
affects the next evaluation, never the stored result. Editing the formula,
inserting a symbol or clearing it also leaves the stored result in place. Only
`Evaluar` replaces it.

## Result components

The `Resultado` section is centered and contains, per assessment:

- **Formula**: a small `Simplificada` or `Original` label and the formula in
  normal infix form, selectable and centered.
- **Process**: a `Proceso` label and one numbered row per applied law, present
  only when the evaluation captured `Mostrar proceso` and recorded steps.
- **Truth table**: a `Tabla de verdad` label and a real table, not aligned
  text. The columns are one per variable plus the result column, headed
  `Resultado`. Cells are centered; `V` is green, `F` is red; rows alternate a
  subtle background. The table grows in the result scroll area; it scrolls
  horizontally when the columns do not fit.
- **Classification**: a `Clasificación` label and the usual Spanish sentence
  (`Esta proposición es una tautología.`, `… una contradicción.`, `… una
  contingencia.`), colored green, red or muted. It is derived from the table,
  so it appears only when a table was built.
- **Error**: a red `Error` label and one selectable report line per error,
  replacing the result. An empty or whitespace-only input evaluated with
  `Evaluar` reports `no se proporcionó ninguna proposición` here; it is not a
  usage error.

## Table policy

The GUI builds a table only when `Mostrar tabla` was on for that evaluation and
the chosen expression has fewer than ten distinct variables. With the control
off, `Orchestrator.assess` is called with no table request, so no table is
processed. With ten or more variables, the table is also skipped; the result
shows the formula and, when requested, the process, plus the muted note
`La tabla no se procesa: {n} variables (el límite es 10).`. The classification
is absent with the table. The CLI keeps its own 12-variable / 4096-row behavior
through `Orchestrator.evaluate`.

## Non-goals

The GUI does not read files, run the interactive loop, write `-o` output, or
keep a history. Those stay CLI concerns. It does not change the language or
the Orchestrator's CLI entry point; the GUI uses `assess` for its own table
policy.

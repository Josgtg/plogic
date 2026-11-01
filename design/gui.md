# Plogic - GUI design

Role: the graphical window. It is a second impure UI component, next to the
CLI in [ui.md](ui.md): it drives the Orchestrator, builds result components
from `TruthTable` and `Simplification`, and performs the window I/O. The
pipeline stays pure and is not duplicated. The GUI evaluates one proposition at
a time.

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

A single native window, 1000x960 by default and at least 760x720, titled
`plogic`. It is dark and monospace, fixed to the mockup's theme. The whole
client area is one vertical scroll area: the content column is centered and at
most 860 points wide, and there is no fixed output panel. From top to bottom:

1. The title `plogic`.
2. `Ayuda`: a one-sentence description and `Conectivas y literales`, a
   compact two-group list with the columns `Símbolo`, `Nombre` and `También`.
   It is set at 12.5 points, slightly smaller than the 15-point body, and ends
   with `Usa paréntesis para agrupar.`.
3. `Opciones`: the three independent controls below.
4. `Proposición`: a single-line field with the hint `p | q & s -> t`, the
   hint line `Escribe una proposición y pulsa Evaluar.`, and Enter as a second
   way to evaluate.
5. The insertion toolbar: `¬`, `∧`, `∨`, `→`, `↔`, `(` and `)` insert their
   symbol at the cursor, replacing a selection; `variable` inserts the next
   single-letter variable from `p` to `z` that the current input does not use,
   skipping `y` (the conjunction word, which can never be an identifier; `p`
   when every usable letter is taken or the formula does not scan); `Limpiar`
   clears the input and its cursor, never the stored result. Every button has a
   hover description in Spanish.
6. `Evaluar`, centered.
7. `Resultado`: only when there is a stored result, a heading followed by the
   separate cards. With no result the window shows the muted line
   `El resultado aparecerá aquí.`.

## Controls

- `Simplificar` (default on): the next evaluation uses the simplified
  expression; unchecked, it uses the original.
- `Mostrar tabla` (default on): shows or hides the table card of the stored
  result.
- `Mostrar proceso` (default off): shows or hides the process card of the
  stored result when it has at least one step.

The controls are independent. There is no separate `no simplificar` control:
unchecking `Simplificar` is how the GUI reaches that mode. The formula card is
available whether or not the table is shown, so the table never has to be
forced on to see the original formula.

`Simplificar` affects only the next evaluation. `Mostrar tabla` and
`Mostrar proceso` are display choices over the stored result and apply
immediately. Editing the formula, inserting a symbol, clearing it or changing
`Simplificar` does not replace the stored result; only `Evaluar` (or Enter in
the formula field) does.

## Result components

The `Resultado` heading is followed by one group of cards per evaluation. Each
card is a separate component; there is no text dump and no enclosing output
box.

- **Formula card**: titled `Simplificada` when the stored evaluation has a
  simplified formula and `Original` otherwise. The value is the formula in
  normal infix form, selectable.
- **Process card**: present only when `Mostrar proceso` is on and the stored
  evaluation recorded steps. One numbered row per step, with the legal name
  (`13a. Condicional`, `10a. De Morgan`, …) and the formula it produced.
- **Truth table card**: a real table, not aligned text. The columns are one per
  variable plus the result column, headed `Resultado`. Cells are centered;
  `V` is green, `F` is red; rows alternate a subtle background; the header
  stays above the rows. The card scrolls horizontally when the columns do not
  fit, and all rows grow in the window flow so the window's own scroll handles
  vertical movement. There is no vertical sub-scroll.
- **Classification card**: the usual Spanish sentence (`Esta proposición es
  una tautología.`, `… una contradicción.`, `… una contingencia.`), colored
  green, red or muted to match the table.
- **Error card**: replaces the result when a stage fails. One red, selectable
  report line per error. An empty or whitespace-only input evaluated with
  `Evaluar` reports `no se proporcionó ninguna proposición` in the same card;
  it is not a usage error.

A new `Evaluar` replaces the entire stored result; there is no history. Errors
and evaluations use the same replace point.

## Non-goals

The GUI does not read files, run the interactive loop, write `-o` output, or
keep a history. Those stay CLI concerns. It does not change the language or
the Orchestrator; the GUI is one more caller of `evaluate` and `simplify`.

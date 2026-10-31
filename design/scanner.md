# Plogic - Scanner design

Role: first pipeline stage. The scanner is the only component that reads
raw source text; it turns the source into the token list (tokenization).
Everything downstream works on tokens.

## Entry point and state

- `Scanner.scan(source: str): Result<List<Token>>` — a static entry point;
  the scanner is never constructed and holds no state.
- The immutable `ScanState { source, token_start, current, line, tokens,
  errors }` is threaded through the private helpers (`scan_token`, `match`,
  `peek`, `peek_next`, `is_at_end`, `advance`, `add_token`) and returned
  anew. Nothing is mutated.

## Character handling

For each character or lexeme at the current position:

- Spaces and tabs separate lexemes and produce no token.
- A line break (LF, or CRLF counted as one) emits `Newline` and advances
  `line`.
- `(` and `)` emit `LeftParen` and `RightParen`.
- `!`, `¬`, `&`, `∧`, `|`, `∨`, `→`, `↔` are single-character lexemes;
  the scanner maps them through `TokenType.map` to their kind. The formal
  symbols take several bytes in UTF-8 but are one character each: the
  scanner advances character by character, so a symbol never splits and
  `col` counts it as one position.
- `-` must be followed by `>` (the lexeme `->`); otherwise `ScanError`:
  `se esperaba '>' después de '-'`.
- `<` must be followed by `->` (the lexeme `<->`); otherwise `ScanError`:
  `se esperaba '->' después de '<'`.
- A digit starts a maximal run of digits, which is mapped through
  `TokenType.map`: `0` and `1` become `Literal(false)` and
  `Literal(true)`; any other run is a `ScanError`: `literal inválido '12'`.
- A letter (ASCII `a`–`z`, `A`–`Z`) starts a maximal run of letters and
  digits (a word). `map` maps the reserved words (`no`, `y`, `o`, `entonces`,
  `sisolosi`, `V`, `F`) to their kind; an unknown word becomes
  `Identifier(lexeme)`. Any other character — `ñ`, `ä`, `_` — never starts a
  word, so it is a `ScanError`.
- Any other character is a `ScanError`: `carácter inválido '%'`.

Since every spelling goes through `TokenType.map`, spellings can be mixed
freely inside a proposition: `p | q & s entonces t sisolosi q | 0` puts
symbols and words together with no difference downstream.

## Positions

- `line` is 1-based and increments at each line break.
- `col` is 1-based and computed when a token is added: the number of
  characters between the last line break and `token_start`. `ScanState`
  needs no column field; it is derived from `source` and `token_start`.

## Errors

The scanner reports as many errors as possible in one pass: on a lexical
error it records a `ScanError(line, col, message)`, skips the offending
character or run, and keeps scanning.

At the end of the pass, `scan` decides the stage result:

- any error recorded → `Err(errors)`; the partial token list is discarded;
- otherwise → `Ok(tokens)`.

This is a stage gate (see error_handling.md): when scanning fails, the
pipeline stops and the parser does not run. `ScanError` and `ParseError`
therefore never appear in the same error list.

## Example

`p | q & s -> t <-> q | 0` tokenizes to:

```
Identifier(p) Or Identifier(q) And Identifier(s) Implication Identifier(t)
Biconditional Identifier(q) Or Literal(false) Newline Eof
```

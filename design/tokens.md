# Plogic - Token vocabulary design

Tokens are the scanner's output and the parser's input: a flat list of
lexical units, each carrying its kind and its position in the source. The
vocabulary mirrors the language defined in the product description (the
source of truth): five connectives, Boolean literals, identifiers and
structural tokens.

## Token

A token is `Token { kind: TokenType, line: int, col: int, reserved_word: bool }`
with a `to_string()` method (`Display` in Rust):

- `kind` is the kind, including its payload (see below). The field is called
  `kind` because `type` is a Rust keyword.
- `kind`, `line` and `col` are public: a token is plain data, read directly by
  every later stage.
- `line` and `col` are 1-based and point at the first character of the
  token (for `Eof`, the position just past the end of the source).
- `reserved_word` is true only when the source spelled the token with a
  reserved word (`no`, `y`, `o`, `entonces`, `sisolosi`, `V`, `F`) rather than
  a symbol. The scanner sets it and the parser uses it for its reserved-word
  diagnostics (ADR-052). It carries no language meaning: the token's kind and
  position are what every later stage reads.
- `to_string()` renders the token's display spelling — the normalized
  symbols `¬`, `∧`, `∨`, `→`, `↔`, the literals `V` / `F`, the parentheses
  `(` / `)`, or the identifier's name. It is used by `ASTPrinter`,
  `InfixPrinter` and by diagnostics. `TokenType` renders the same spelling and
  a token delegates to its kind. `Newline` and `Eof` are structural: they have
  no spelling and render as the empty string (ADR-036).

The token does not keep the source spelling of a connective or a literal;
identifiers keep their name. `reserved_word` is only a diagnostic flag, not
the spelling. A proposition is displayed in one normalized form (see
truth_table.md), so how the user spelled a kind (`y`, `entonces`, `0`, `->`)
never reaches the output.

## TokenType

`TokenType` is an enum with payload variants:

| Variant | Payload | Meaning |
| --- | --- | --- |
| `Not` | — | negation |
| `And` | — | conjunction |
| `Or` | — | disjunction |
| `Implication` | — | material implication |
| `Biconditional` | — | material equivalence |
| `Literal` | `bool` | truth-value constant |
| `Identifier` | `str` | propositional variable |
| `LeftParen`, `RightParen` | — | grouping |
| `Newline` | — | proposition terminator |
| `Eof` | — | end of input (synthetic) |

The vocabulary is closed: XOR, NAND and NOR are out of scope and are *not*
variants.

## Lexeme table: `TokenType.map(lexeme: str): Result<TokenType, str>`

`map` is the single place where spellings are defined. Known spellings map
to their kind:

| Kind | Spellings |
| --- | --- |
| `Not` | `!` `¬` `no` |
| `And` | `&` `∧` `y` |
| `Or` | `\|` `∨` `o` |
| `Implication` | `->` `→` `entonces` |
| `Biconditional` | `<->` `↔` `sisolosi` |
| `Literal(true)` | `1` `V` |
| `Literal(false)` | `0` `F` |
| `LeftParen`, `RightParen` | `(` `)` |

`map` returns `Err(message)` for any other lexeme, where the message says what
was expected and is decided by the lexeme's shape: a run of digits gives
`literal inválido '<lexeme>'`, anything else gives
`carácter inválido '<lexeme>'`. The text is composed in `map`, so the scanner
never builds one of its own (ADR-036). The scanner then decides what to do with
the result: a word-shaped lexeme that is not in the table becomes
`Identifier(lexeme)`, anything else is a `ScanError` carrying `map`'s message at
the token's position (see scanner.md).

Two consequences worth spelling out:

- The words used as spellings for every token kind are
  reserved: they can never be identifiers. This is what makes `p y q` a
  two-operand proposition rather than a syntax error.
- Multi-character spellings (`->`, `<->`) must be written without spaces.

## Literals and identifiers

- A literal is one of the four spellings `0`, `1`, `V`, `F`. The truth
  value is the payload, so `0` and `F` produce the identical kind
  `Literal(false)`. Nothing keeps the spelling: every display shows
  literals as `V` and `F`.
- An identifier is an ASCII letter (`a`–`z`, `A`–`Z`) followed by ASCII
  letters, digits or underscores: `p`, `q`, `s`, `t`, `p1`, `p_1`. An
  underscore never starts an identifier, so `_p` is an invalid character.
  Identifiers are case-sensitive: `P` and `p` are different variables.

## Newline and Eof

- `Newline` is a real token: it terminates a proposition. Every line break
  (LF, or CRLF counted as one) emits one, and an empty line is just a
  `Newline` with nothing before it. The parser skips empty propositions.
- `Eof` is a synthetic final token appended after the last line, following
  the Crafting Interpreters convention. It is never produced by `map`.

# Plogic - Simplifier design

Role: rewrites a proposition into an equivalent proposition of lower weight
(defined below), using the laws of propositional algebra (components diagram,
steps 6-7). The Simplifier is pure and total: `Expression` in, `Expression`
out, no errors. Its output contains only negation, conjunction and
disjunction (written `!`, `&`, `|` here and displayed as `¬`, `∧`, `∨`):
conditionals and biconditionals are eliminated.

## Strategy

Three parts:

1. **Canonicalization** (laws 2 and 3). `&` and `|` chains are flattened and
   their operands sorted, so every other law matches regardless of operand
   order or nesting. This never changes the weight.

   Two expressions like `q & (p & r)` and `r & (q & p)` are the same
   proposition by laws 2 and 3, but as trees they look different.
   Canonicalization picks one representative for every such group.
   Flattening merges nested chains of the same operator into one operand
   list: `q & (p & r)` becomes one `&` chain with the operands `q, p, r`.
   The lists are held by the Simplifier internally — the AST keeps binary
   nodes, and a chain is rebuilt as left-nested `Binary` nodes when the
   result is returned. Sorting then puts the operands in the fixed order
   defined below. After both steps, two expressions that differ only by
   laws 2 and 3 are structurally identical, so "do these operands match?"
   becomes a plain equality check — idempotence, absorption and factoring
   all rely on that — and the output is deterministic.
2. **Reduction.** The remaining laws are applied as oriented local rewrite
   rules, bottom-up. After a node is rewritten, the result is canonicalized
   and simplified again, because a rewrite can expose new matches. Passes
   repeat until no rule changes the tree (a fixed point).
3. **Expansion (optional).** Rules that temporarily make the formula bigger
   are tried one at a time. A result is kept only if, after reduction, its
   weight is strictly lower than before; otherwise it is discarded. See
   "Inverse rules".

The result is equivalent (same truth table). No particular normal form is
guaranteed.

## Canonical order

The order used when operands are sorted has to be:

- **Total**: for any two different expressions, one comes before the other.
- **Structural**: it depends only on the expression's content, never on
  memory addresses or insertion order.

Two expressions are compared by kind first, then within the kind:

| Rank | Kind | Tie-break within the same kind |
| --- | --- | --- |
| 1 | constant | `F` before `V` (false before true) |
| 2 | variable | alphabetical by name |
| 3 | `!x` | compare the operands `x` |
| 4 | `&` chain | compare the operand lists element by element, then by length |
| 5 | `\|` chain | same as `&` |
| 6, 7 | `->`, `<->` | compare the left operand, then the right (they are eliminated early, so they only need to sort consistently) |

For example, `q & p & !p & (r | s) & V` sorts to
`V & p & q & !p & (r | s)`: the constant first, then the variables `p` and
`q`, then `!p`, then the `|` chain.

Canonical order is internal to the Simplifier. It shows in the simplified
formula; the proposition itself is labelled with the operand order the user
wrote (see truth_table.md).

This document writes the laws with `!`, `&`, `|`, `->` and `<->` because
they are easier to type; the user-facing display uses `¬`, `∧`, `∨`, `→` and
`↔` (see tokens.md).

## The laws

Important to note `p`, `q` and `r` stand for any
subexpression, not only variables.

| No. | Group | Law |
| --- | --- | --- |
| 1a | Idempotent laws | `p \| p = p` |
| 1b | | `p & p = p` |
| 2a | Associative laws | `(p \| q) \| r = p \| (q \| r)` |
| 2b | | `(p & q) & r = p & (q & r)` |
| 3a | Commutative laws | `p \| q = q \| p` |
| 3b | | `p & q = q & p` |
| 4a | Distributive laws | `p \| (q & r) = (p \| q) & (p \| r)` |
| 4b | | `p & (q \| r) = (p & q) \| (p & r)` |
| 5a | Identity laws | `p \| F = p` |
| 5b | | `p & V = p` |
| 6a | Domination laws | `p \| V = V` |
| 6b | | `p & F = F` |
| 7a | Double negation law | `!!p = p` |
| 8a | Complement laws | `p \| !p = V` |
| 8b | | `p & !p = F` |
| 9a | | `!V = F` |
| 9b | | `!F = V` |
| 10a | De Morgan laws | `!(p \| q) = !p & !q` |
| 10b | | `!(p & q) = !p \| !q` |
| 11a | Absorption laws | `p \| (p & q) = p` |
| 11b | | `p & (p \| q) = p` |
| 12a | Redundancy laws | `p \| (!p & q) = p \| q` |
| 12b | | `p & (!p \| q) = p & q` |
| 13a | Conditional law | `p -> q = !p \| q` |
| 14a | Biconditional law | `p <-> q = (!p \| q) & (!q \| p)` |

## How each law is used

A law is an equivalence, so it can be read in both directions. The Simplifier
fixes one direction per law:

| Laws | Use |
| --- | --- |
| 2a, 2b, 3a, 3b | Canonicalization: flatten, then sort operands. Not rewrite rules. |
| 1a, 1b | Reduction, left to right. |
| 5a, 5b, 6a, 6b | Reduction, left to right. |
| 7a | Reduction, left to right. |
| 8a, 8b, 9a, 9b | Reduction, left to right. |
| 10a, 10b | Reduction, left to right (pushes negations towards the leaves). |
| 11a, 11b, 12a, 12b | Reduction, left to right. |
| 13a, 14a | Reduction, left to right (eliminates `->` and `<->`). |
| 4a, 4b | Reduction right-to-left (factoring); left-to-right (expansion) only in the expansion phase, see below. |

Because `&` and `|` are flattened and sorted, a two-operand pattern matches any
two operands of a longer chain. For example, 11a matches `a | b | (a & c)`
by taking `a` and `(a & c)`. Rule results are re-canonicalized.

## Weight and termination

Every expression has a weight `W`, a natural number:

| Node | Weight |
| --- | --- |
| variable or constant (`F`, `V`) | `1` |
| `!p` | `2 * W(p)` |
| `p & q`, `p \| q` | `W(p) + W(q) + 1` |
| `p -> q` | `2 * W(p) + W(q) + 2` |
| `p <-> q` | `3 * W(p) + 3 * W(q) + 4` |

With `W(p) = a`, `W(q) = b` and `W(r) = c`, each reduction rule strictly
lowers `W`:

| Law | Before | After |
| --- | --- | --- |
| 1a, 1b | `2a + 1` | `a` |
| 5a, 5b | `a + 2` | `a` |
| 6a, 6b | `a + 2` | `1` |
| 7a | `4a` | `a` |
| 8a, 8b | `3a + 1` | `1` |
| 9a, 9b | `2` | `1` |
| 10a, 10b | `2a + 2b + 2` | `2a + 2b + 1` |
| 11a, 11b | `2a + b + 2` | `a` |
| 12a, 12b | `3a + b + 2` | `a + b + 1` |
| 13a | `2a + b + 2` | `2a + b + 1` |
| 14a | `3a + 3b + 4` | `3a + 3b + 3` |
| 4a, 4b right to left (factoring) | `2a + b + c + 3` | `a + b + c + 2` |

The weight is strictly increasing in every operand, so lowering `W` at a node
lowers it for the whole formula, however deeply that node is nested.
Canonicalization does not change `W` (a flattened chain weighs the sum of its
operands plus one per connective, however it is nested or ordered) and always
terminates, since it only flattens and sorts. `W` is a natural number that
cannot decrease forever, so reduction reaches a fixed point after finitely
many rewrites. No iteration cap is needed for correctness; one may be kept as
a debug assertion.

`W` is not the connective count. De Morgan (10a, 10b) and the biconditional
law (14a) add connectives but lower `W`: `W` prefers negations on small
subformulas and no `->` or `<->`.

## Inverse rules (expansion)

Some simplifications need a temporary increase in size. Expansion rules are
laws 4a and 4b read left to right:

- `p | (q & r)` -> `(p | q) & (p | r)`  (4a)
- `p & (q | r)` -> `(p & q) | (p & r)`  (4b)

They are never part of reduction, because they would undo factoring and loop.
Instead they are tried like this:

```
best = reduce(tree)
loop:
    for each expansion rule E and each node where E matches in best:
        candidate = reduce(E(best at that node))
        if W(candidate) < W(best):     // strictly lower, not merely different
            best = candidate
            continue loop              // restart from the new best
    return best
```

The comparison is on `W`:

- Expanding and then re-factoring returns the same tree. Its `W` is not
  strictly lower, so it is discarded. This is the case where the inverse of
  a rule undoes the rule.
- Longer cycles (`A -> B -> C -> A`) are discarded for the same reason, so no
  visited-set bookkeeping is needed.
- Every accepted candidate lowers `W`, a natural number, so the loop
  terminates. There are at most `W(input)` accepted candidates, each found by
  a finite scan of rule/node pairs.
- The expansion phase is deterministic: rules and nodes are tried in a fixed
  order, and the first improvement wins.

Other inverse rules (for example `!p | !q` -> `!(p & q)`, 10b read right to
left) can be added to the expansion list without changing this argument, as
long as each one is equivalence-preserving.

## Examples

Operand order in the output follows the canonical order.

| Input | Law(s) | Output |
| --- | --- | --- |
| `!!p` | 7a | `p` |
| `p & F` | 6b | `F` |
| `p \| V` | 6a | `V` |
| `!(p \| q)` | 10a | `!p & !q` |
| `(p & q) \| (p & r)` | 4b (factoring) | `p & (q \| r)` |
| `!(!p \| !q)` | 10a, 7a | `p & q` |
| `p \| (!p & q)` | 12a | `p \| q` |
| `p -> q` | 13a | `!p \| q` |
| `p <-> q` | 14a | `(!p \| q) & (!q \| p)` |
| `(p & q) \| (p & !q)` | 4b (factoring), 8a, 5b | `p` |
| `p & (q \| r)` | none | `p & (q \| r)` |

- `!(!p | !q)`: 10a gives `!!p & !!q` (W 10 -> 9); 7a on each side gives
  `p & q` (W 3).
- `(p & q) | (p & !q)`: factoring gives `p & (q | !q)`; 8a gives `p & V`; 5b
  gives `p`.
- `p & (q | r)` is unchanged. Expansion (4b) gives `(p & q) | (p & r)`
  (W 5 -> 7), reduction factors it back to W 5, which is not strictly lower,
  so the candidate is discarded.

A proposition that reduces to `V` is a tautology and one that reduces to `F`
is a contradiction. The classification shown to the user is computed from the
truth table (see truth_table.md), so it stays exact even when the
simplified form is not a constant.

## Scope

XOR, NAND and NOR are out of scope (see tokens.md): the law table has no
rules for them.

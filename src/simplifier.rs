//! The simplifier: rewrites an expression into an equivalent one of no greater
//! weight (`design/simplifier.md`).
//!
//! Pure and total: `Expression` in, a [`Simplification`] out, no errors. The
//! result carries the final expression and the ordered [`Step`]s that produced
//! it. Grouping is erased first; `&` and `|` chains are flattened and sorted in
//! the canonical order; the oriented rewrite rules run bottom-up to a fixed
//! point; and the optional expansion phase keeps a candidate only when it
//! strictly lowers the weight (ADR-007, ADR-008). The rules are tried in the
//! order of the law table and the expansion visits the tree pre-order
//! (ADR-042). Only applied laws become steps: canonicalization is not a step,
//! and a discarded expansion candidate leaves no trace (ADR-047).

use std::cmp::Ordering;

use crate::expression::Expression;
use crate::tokens::{Token, TokenType};

/// The simplifier stage. Never constructed.
pub struct Simplifier;

/// A law that a single rewrite applied, named after the law table
/// (`design/simplifier.md`) with its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rule {
    IdempotenceA,
    IdempotenceB,
    IdentityA,
    IdentityB,
    DominationA,
    DominationB,
    DoubleNegation,
    ComplementA,
    ComplementB,
    NegatedTrue,
    NegatedFalse,
    DeMorganA,
    DeMorganB,
    AbsorptionA,
    AbsorptionB,
    RedundancyA,
    RedundancyB,
    Conditional,
    Biconditional,
    FactoringA,
    FactoringB,
    DistributionA,
    DistributionB,
}

impl Rule {
    /// The user-facing label: law number and Spanish name.
    pub fn label(self) -> &'static str {
        match self {
            Rule::IdempotenceA => "1a. Idempotencia",
            Rule::IdempotenceB => "1b. Idempotencia",
            Rule::IdentityA => "5a. Identidad",
            Rule::IdentityB => "5b. Identidad",
            Rule::DominationA => "6a. Dominación",
            Rule::DominationB => "6b. Dominación",
            Rule::DoubleNegation => "7a. Doble negación",
            Rule::ComplementA => "8a. Complemento",
            Rule::ComplementB => "8b. Complemento",
            Rule::NegatedTrue => "9a. Complemento",
            Rule::NegatedFalse => "9b. Complemento",
            Rule::DeMorganA => "10a. De Morgan",
            Rule::DeMorganB => "10b. De Morgan",
            Rule::AbsorptionA => "11a. Absorción",
            Rule::AbsorptionB => "11b. Absorción",
            Rule::RedundancyA => "12a. Redundancia",
            Rule::RedundancyB => "12b. Redundancia",
            Rule::Conditional => "13a. Condicional",
            Rule::Biconditional => "14a. Bicondicional",
            Rule::FactoringA => "4a. Factorización",
            Rule::FactoringB => "4b. Factorización",
            Rule::DistributionA => "4a. Distribución",
            Rule::DistributionB => "4b. Distribución",
        }
    }
}

/// One applied rewrite: the law and the canonical formula it produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub rule: Rule,
    pub expression: Expression,
}

/// The simplifier's result: the final expression and the ordered steps that
/// produced it. The steps are empty when no law applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Simplification {
    pub expression: Expression,
    pub steps: Vec<Step>,
}

impl Simplifier {
    /// Simplifies `expression` into an equivalent expression, together with the
    /// ordered steps that produced it. The expression contains no `Grouping`,
    /// no `->` and no `<->`; its `&` and `|` chains are canonical (flattened
    /// and sorted).
    pub fn simplify(expression: &Expression) -> Simplification {
        // `reduce` canonicalizes its input itself, so this is one pass and not
        // two; `expand` receives the fixed point, which is already canonical.
        let ungrouped = Self::erase_grouping(expression);
        let (reduced, steps) = Self::reduce(&ungrouped);
        Self::expand(&reduced, steps)
    }

    /// Step 0: drops every `Grouping`. The precedence already encodes what the
    /// parentheses said, and the rest of the simplifier never sees one.
    fn erase_grouping(expression: &Expression) -> Expression {
        match expression {
            Expression::Grouping { expression } => Self::erase_grouping(expression),
            Expression::Binary {
                left,
                right,
                operator,
            } => Expression::Binary {
                left: Box::new(Self::erase_grouping(left)),
                right: Box::new(Self::erase_grouping(right)),
                operator: operator.clone(),
            },
            Expression::Unary {
                operator,
                expression,
            } => Expression::Unary {
                operator: operator.clone(),
                expression: Box::new(Self::erase_grouping(expression)),
            },
            leaf => leaf.clone(),
        }
    }

    /// Step 1: flattens `&` and `|` chains and sorts their operands. The AST
    /// keeps binary nodes, so a chain is rebuilt as left-nested `Binary` nodes.
    fn canonicalize(expression: &Expression) -> Expression {
        match expression {
            Expression::Binary {
                left,
                right,
                operator,
            } => {
                let left = Self::canonicalize(left);
                let right = Self::canonicalize(right);
                let node = Expression::Binary {
                    left: Box::new(left),
                    right: Box::new(right),
                    operator: operator.clone(),
                };
                if !matches!(operator.kind, TokenType::And | TokenType::Or) {
                    return node;
                }
                let operands: Vec<Expression> = flatten(&node, &operator.kind)
                    .into_iter()
                    .cloned()
                    .collect();
                let mut operands = operands;
                operands.sort_by(Self::compare);
                // A chain always has at least two operands, so `join_all`
                // cannot fail; `node` is a total fallback all the same.
                join_all(operands, operator).unwrap_or(node)
            }
            Expression::Unary {
                operator,
                expression,
            } => Expression::Unary {
                operator: operator.clone(),
                expression: Box::new(Self::canonicalize(expression)),
            },
            // Erased by step 0; treated transparently if one still arrives.
            Expression::Grouping { expression } => Self::canonicalize(expression),
            leaf => leaf.clone(),
        }
    }

    /// Step 2: applies the reduction rules bottom-up until none matches, and
    /// records every applied law with its canonical result. Every rewrite
    /// strictly lowers the weight, so the loop terminates.
    fn reduce(expression: &Expression) -> (Expression, Vec<Step>) {
        let mut current = Self::canonicalize(expression);
        let mut steps = Vec::new();
        loop {
            let Some(rewrite) = rewrite_once(&current) else {
                return (current, steps);
            };
            let next = Self::canonicalize(&rewrite.expression);
            steps.push(Step {
                rule: rewrite.rule,
                expression: next.clone(),
            });
            current = next;
        }
    }

    /// Step 3: tries the expansion laws one at a time and keeps a candidate
    /// only when, after reduction, it weighs strictly less than the best so
    /// far. The weight is a natural number that only drops, so the loop ends.
    /// An accepted candidate contributes its distribution step followed by the
    /// reduction steps it triggered; a discarded candidate contributes nothing.
    fn expand(expression: &Expression, mut steps: Vec<Step>) -> Simplification {
        let mut best = expression.clone();
        loop {
            let mut improved = false;
            for (rule, candidate) in expansion_candidates(&best) {
                let canonical = Self::canonicalize(&candidate);
                let (reduced, mut reduction_steps) = Self::reduce(&canonical);
                if Self::weight(&reduced) < Self::weight(&best) {
                    steps.push(Step {
                        rule,
                        expression: canonical,
                    });
                    steps.append(&mut reduction_steps);
                    best = reduced;
                    improved = true;
                    break;
                }
            }
            if !improved {
                return Simplification {
                    expression: best,
                    steps,
                };
            }
        }
    }

    /// `W`, the measure every reduction rule strictly lowers. It is not the
    /// connective count: De Morgan and the biconditional law add connectives
    /// while lowering it (ADR-023). Saturating arithmetic keeps a pathological
    /// input (a hundred nested negations) from overflowing; a saturated weight
    /// still terminates the loops, since it can no longer decrease.
    fn weight(expression: &Expression) -> u64 {
        match ungrouped(expression) {
            Expression::Literal { .. } | Expression::Identifier { .. } => 1,
            Expression::Unary { expression, .. } => Self::weight(expression).saturating_mul(2),
            Expression::Binary {
                left,
                right,
                operator,
            } => {
                let left = Self::weight(left);
                let right = Self::weight(right);
                match operator.kind {
                    TokenType::And | TokenType::Or => left.saturating_add(right).saturating_add(1),
                    TokenType::Implication => left
                        .saturating_mul(2)
                        .saturating_add(right)
                        .saturating_add(2),
                    TokenType::Biconditional => left
                        .saturating_mul(3)
                        .saturating_add(right.saturating_mul(3))
                        .saturating_add(4),
                    _ => left.saturating_add(right).saturating_add(1),
                }
            }
            // Only reachable before step 0; a grouping weighs what it wraps.
            Expression::Grouping { .. } => 1,
        }
    }

    /// The canonical order: total and structural. Two expressions are compared
    /// by kind first — constant, variable, negation, `&` chain, `|` chain,
    /// `->`, `<->` — and then within the kind (`design/simplifier.md`).
    fn compare(left: &Expression, right: &Expression) -> Ordering {
        let left = ungrouped(left);
        let right = ungrouped(right);
        if left == right {
            return Ordering::Equal;
        }
        match rank(left).cmp(&rank(right)) {
            Ordering::Equal => {}
            other => return other,
        }
        match (left, right) {
            (Expression::Literal { value: left }, Expression::Literal { value: right }) => {
                left.cmp(right)
            }
            (Expression::Identifier { name: left }, Expression::Identifier { name: right }) => {
                left.cmp(right)
            }
            (
                Expression::Unary {
                    expression: left, ..
                },
                Expression::Unary {
                    expression: right, ..
                },
            ) => Self::compare(left, right),
            (Expression::Binary { operator, .. }, Expression::Binary { .. })
                if matches!(operator.kind, TokenType::And | TokenType::Or) =>
            {
                compare_chains(left, right, &operator.kind)
            }
            (
                Expression::Binary {
                    left: left_left,
                    right: left_right,
                    ..
                },
                Expression::Binary {
                    left: right_left,
                    right: right_right,
                    ..
                },
            ) => Self::compare(left_left, right_left)
                .then_with(|| Self::compare(left_right, right_right)),
            _ => Ordering::Equal,
        }
    }
}

/// Strips a grouping, if any. The simplifier never works on a grouped tree,
/// but the order and the weight stay correct if one slips in.
fn ungrouped(expression: &Expression) -> &Expression {
    match expression {
        Expression::Grouping { expression } => ungrouped(expression),
        other => other,
    }
}

/// The rank of an expression in the canonical order.
fn rank(expression: &Expression) -> u8 {
    match ungrouped(expression) {
        Expression::Literal { .. } => 1,
        Expression::Identifier { .. } => 2,
        Expression::Unary { .. } => 3,
        Expression::Binary { operator, .. } => match operator.kind {
            TokenType::And => 4,
            TokenType::Or => 5,
            TokenType::Implication => 6,
            TokenType::Biconditional => 7,
            _ => 8,
        },
        Expression::Grouping { .. } => 8,
    }
}

/// Compares two chains of the same kind: operand by operand, then by length.
fn compare_chains(left: &Expression, right: &Expression, kind: &TokenType) -> Ordering {
    let left = flatten(left, kind);
    let right = flatten(right, kind);
    for (left, right) in left.iter().zip(right.iter()) {
        let ordering = Simplifier::compare(left, right);
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

/// The operands of a `&` or `|` chain, left to right, as references.
fn flatten<'a>(expression: &'a Expression, kind: &TokenType) -> Vec<&'a Expression> {
    fn collect<'a>(
        expression: &'a Expression,
        kind: &TokenType,
        operands: &mut Vec<&'a Expression>,
    ) {
        match expression {
            Expression::Binary {
                left,
                right,
                operator,
            } if operator.kind == *kind => {
                collect(left, kind, operands);
                collect(right, kind, operands);
            }
            other => operands.push(other),
        }
    }

    let mut operands = Vec::new();
    collect(expression, kind, &mut operands);
    operands
}

/// Rebuilds a chain as left-nested `Binary` nodes. `None` for an empty list.
fn join_all(operands: Vec<Expression>, operator: &Token) -> Option<Expression> {
    let mut operands = operands.into_iter();
    let first = operands.next()?;
    Some(operands.fold(first, |left, right| Expression::Binary {
        left: Box::new(left),
        right: Box::new(right),
        operator: operator.clone(),
    }))
}

/// The operands of `expression` when it is a chain of `kind`, if it is one.
fn chain_operands<'a>(expression: &'a Expression, kind: &TokenType) -> Option<Vec<&'a Expression>> {
    match expression {
        Expression::Binary { operator, .. } if operator.kind == *kind => {
            Some(flatten(expression, kind))
        }
        _ => None,
    }
}

/// A fresh operator token. The rules create nodes the source never spelled, so
/// they need an operator; equality ignores positions (ADR-040).
fn operator_token(kind: &TokenType) -> Token {
    Token {
        kind: kind.clone(),
        line: 0,
        col: 0,
    }
}

/// `!expression`, the negation the rules build.
fn negation(expression: &Expression) -> Expression {
    Expression::Unary {
        operator: operator_token(&TokenType::Not),
        expression: Box::new(expression.clone()),
    }
}

/// Whether `candidate` is the negation of `text`.
fn is_negation_of(text: &Expression, candidate: &Expression) -> bool {
    matches!(
        candidate,
        Expression::Unary { operator, expression }
            if matches!(operator.kind, TokenType::Not) && **expression == *text
    )
}

/// One rewrite at a node: the law applied and the tree it produced. The tree
/// may still need canonicalization; `reduce` does that before recording it.
struct Rewrite {
    rule: Rule,
    expression: Expression,
}

/// Runs the first applicable rewrite at the root of `expression`, with the
/// children already reduced. `None` when the node is already in normal form.
///
/// The order is the law table's (ADR-042): idempotence, identity, domination,
/// complement, absorption, redundancy, conditional, biconditional, and finally
/// factoring, which is law 4 read right to left.
fn rewrite(expression: &Expression) -> Option<Rewrite> {
    match expression {
        Expression::Binary {
            left,
            right,
            operator,
        } => match operator.kind {
            TokenType::Implication => Some(Rewrite {
                rule: Rule::Conditional,
                expression: conditional(left, right),
            }),
            TokenType::Biconditional => Some(Rewrite {
                rule: Rule::Biconditional,
                expression: biconditional(left, right),
            }),
            TokenType::And => chain_rules(expression, operator, &TokenType::And),
            TokenType::Or => chain_rules(expression, operator, &TokenType::Or),
            _ => None,
        },
        Expression::Unary {
            operator,
            expression,
        } if matches!(operator.kind, TokenType::Not) => negation_rules(expression),
        Expression::Grouping { expression } => rewrite(expression),
        _ => None,
    }
}

/// The first rewrite in a bottom-up, left-to-right scan. The caller
/// canonicalizes the result and scans again, so a rewrite is free to leave the
/// tree unsorted.
fn rewrite_once(expression: &Expression) -> Option<Rewrite> {
    match expression {
        Expression::Binary {
            left,
            right,
            operator,
        } => {
            if let Some(rewrite) = rewrite_once(left) {
                return Some(Rewrite {
                    rule: rewrite.rule,
                    expression: Expression::Binary {
                        left: Box::new(rewrite.expression),
                        right: right.clone(),
                        operator: operator.clone(),
                    },
                });
            }
            if let Some(rewrite) = rewrite_once(right) {
                return Some(Rewrite {
                    rule: rewrite.rule,
                    expression: Expression::Binary {
                        left: left.clone(),
                        right: Box::new(rewrite.expression),
                        operator: operator.clone(),
                    },
                });
            }
            rewrite(expression)
        }
        Expression::Unary {
            operator,
            expression: inner,
        } => {
            if let Some(rewrite) = rewrite_once(inner) {
                return Some(Rewrite {
                    rule: rewrite.rule,
                    expression: Expression::Unary {
                        operator: operator.clone(),
                        expression: Box::new(rewrite.expression),
                    },
                });
            }
            rewrite(expression)
        }
        Expression::Grouping { expression } => rewrite_once(expression),
        _ => rewrite(expression),
    }
}

/// Law 13: `p -> q = !p | q`.
fn conditional(left: &Expression, right: &Expression) -> Expression {
    Expression::Binary {
        left: Box::new(negation(left)),
        right: Box::new(right.clone()),
        operator: operator_token(&TokenType::Or),
    }
}

/// Law 14: `p <-> q = (!p | q) & (!q | p)`.
fn biconditional(left: &Expression, right: &Expression) -> Expression {
    let forward = Expression::Binary {
        left: Box::new(negation(left)),
        right: Box::new(right.clone()),
        operator: operator_token(&TokenType::Or),
    };
    let backward = Expression::Binary {
        left: Box::new(negation(right)),
        right: Box::new(left.clone()),
        operator: operator_token(&TokenType::Or),
    };
    Expression::Binary {
        left: Box::new(forward),
        right: Box::new(backward),
        operator: operator_token(&TokenType::And),
    }
}

/// Laws 7, 9 and 10, tried in that order on `!inner`.
fn negation_rules(inner: &Expression) -> Option<Rewrite> {
    match inner {
        // 7a: `!!p = p`.
        Expression::Unary {
            operator,
            expression,
        } if matches!(operator.kind, TokenType::Not) => Some(Rewrite {
            rule: Rule::DoubleNegation,
            expression: (**expression).clone(),
        }),
        // 9a/9b: `!V = F`, `!F = V`.
        Expression::Literal { value } => Some(Rewrite {
            rule: if *value {
                Rule::NegatedTrue
            } else {
                Rule::NegatedFalse
            },
            expression: Expression::Literal { value: !value },
        }),
        // 10a: `!(p | q) = !p & !q`.
        Expression::Binary { operator, .. } if matches!(operator.kind, TokenType::Or) => {
            let negated = flatten(inner, &TokenType::Or)
                .into_iter()
                .map(negation)
                .collect();
            Some(Rewrite {
                rule: Rule::DeMorganA,
                expression: join_all(negated, &operator_token(&TokenType::And))?,
            })
        }
        // 10b: `!(p & q) = !p | !q`.
        Expression::Binary { operator, .. } if matches!(operator.kind, TokenType::And) => {
            let negated = flatten(inner, &TokenType::And)
                .into_iter()
                .map(negation)
                .collect();
            Some(Rewrite {
                rule: Rule::DeMorganB,
                expression: join_all(negated, &operator_token(&TokenType::Or))?,
            })
        }
        _ => None,
    }
}

/// The chain rules of `operands`, a flattened `&` or `|` chain, in the law
/// table's order (ADR-042).
fn chain_rules(expression: &Expression, operator: &Token, kind: &TokenType) -> Option<Rewrite> {
    let operands = flatten(expression, kind);
    idempotence(&operands, operator, kind)
        .or_else(|| identity(&operands, kind, operator))
        .or_else(|| domination(&operands, kind))
        .or_else(|| complement(&operands, kind))
        .or_else(|| absorption(&operands, kind, operator))
        .or_else(|| redundancy(&operands, kind, operator))
        .or_else(|| factoring(&operands, kind, operator))
}

/// Picks the `a` side of a law for `|` and the `b` side for `&`, the
/// convention every chain law follows in the table.
fn directed(a: Rule, b: Rule, kind: &TokenType) -> Rule {
    if matches!(kind, TokenType::And) { b } else { a }
}

/// Laws 1a/1b: `p | p = p`, `p & p = p`. Duplicates are dropped pairwise;
/// the loop removes them all, one rewrite at a time.
fn idempotence(operands: &[&Expression], operator: &Token, kind: &TokenType) -> Option<Rewrite> {
    for (index, operand) in operands.iter().enumerate() {
        if operands[..index].iter().any(|earlier| earlier == operand) {
            let kept = operands
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, operand)| (*operand).clone())
                .collect();
            return Some(Rewrite {
                rule: directed(Rule::IdempotenceA, Rule::IdempotenceB, kind),
                expression: join_all(kept, operator)?,
            });
        }
    }
    None
}

/// Laws 5a/5b: `p | F = p`, `p & V = p`.
fn identity(operands: &[&Expression], kind: &TokenType, operator: &Token) -> Option<Rewrite> {
    let identity = matches!(kind, TokenType::And);
    let index = operands.iter().position(
        |operand| matches!(**operand, Expression::Literal { value } if value == identity),
    )?;
    let kept = operands
        .iter()
        .enumerate()
        .filter(|(other, _)| *other != index)
        .map(|(_, operand)| (*operand).clone())
        .collect();
    Some(Rewrite {
        rule: directed(Rule::IdentityA, Rule::IdentityB, kind),
        expression: join_all(kept, operator)?,
    })
}

/// Laws 6a/6b: `p | V = V`, `p & F = F`.
fn domination(operands: &[&Expression], kind: &TokenType) -> Option<Rewrite> {
    let dominating = matches!(kind, TokenType::Or);
    let found = operands
        .iter()
        .any(|operand| matches!(**operand, Expression::Literal { value } if value == dominating));
    found.then(|| Rewrite {
        rule: directed(Rule::DominationA, Rule::DominationB, kind),
        expression: Expression::Literal { value: dominating },
    })
}

/// Laws 8a/8b: `p | !p = V`, `p & !p = F`.
fn complement(operands: &[&Expression], kind: &TokenType) -> Option<Rewrite> {
    for (index, left) in operands.iter().enumerate() {
        for right in &operands[index + 1..] {
            if is_negation_of(left, right) || is_negation_of(right, left) {
                return Some(Rewrite {
                    rule: directed(Rule::ComplementA, Rule::ComplementB, kind),
                    expression: Expression::Literal {
                        value: matches!(kind, TokenType::Or),
                    },
                });
            }
        }
    }
    None
}

/// Laws 11a/11b: `p | (p & q) = p`, `p & (p | q) = p`. The inner chain is
/// dropped, whatever the other operands are.
fn absorption(operands: &[&Expression], kind: &TokenType, operator: &Token) -> Option<Rewrite> {
    let inner_kind = dual(kind);
    for (index, inner) in operands.iter().enumerate() {
        let Some(inner_operands) = chain_operands(inner, inner_kind) else {
            continue;
        };
        let repeated = operands.iter().enumerate().any(|(other, operand)| {
            other != index && inner_operands.iter().any(|inner| inner == operand)
        });
        if repeated {
            let kept = operands
                .iter()
                .enumerate()
                .filter(|(other, _)| *other != index)
                .map(|(_, operand)| (*operand).clone())
                .collect();
            return Some(Rewrite {
                rule: directed(Rule::AbsorptionA, Rule::AbsorptionB, kind),
                expression: join_all(kept, operator)?,
            });
        }
    }
    None
}

/// Laws 12a/12b: `p | (!p & q) = p | q`, `p & (!p | q) = p & q`. The inner
/// chain loses its `!p`; the outer `p` stays.
fn redundancy(operands: &[&Expression], kind: &TokenType, operator: &Token) -> Option<Rewrite> {
    let inner_kind = dual(kind);
    for (index, inner) in operands.iter().enumerate() {
        let Some(inner_operands) = chain_operands(inner, inner_kind) else {
            continue;
        };
        for (other, outer) in operands.iter().enumerate() {
            if other == index {
                continue;
            }
            let Some(negated) = inner_operands
                .iter()
                .position(|candidate| is_negation_of(outer, candidate))
            else {
                continue;
            };
            let rest = inner_operands
                .iter()
                .enumerate()
                .filter(|(position, _)| *position != negated)
                .map(|(_, operand)| (*operand).clone())
                .collect();
            let replacement = join_all(rest, &operator_token(inner_kind))?;
            let mut kept: Vec<Expression> =
                operands.iter().map(|operand| (*operand).clone()).collect();
            kept[index] = replacement;
            return Some(Rewrite {
                rule: directed(Rule::RedundancyA, Rule::RedundancyB, kind),
                expression: join_all(kept, operator)?,
            });
        }
    }
    None
}

/// Law 4 read right to left (factoring): `(p | q) & (p | r) = p | (q & r)`,
/// and its dual `(p & q) | (p & r) = p & (q | r)`.
fn factoring(operands: &[&Expression], kind: &TokenType, operator: &Token) -> Option<Rewrite> {
    let inner_kind = dual(kind);
    for first in 0..operands.len() {
        let Some(left) = chain_operands(operands[first], inner_kind) else {
            continue;
        };
        for second in first + 1..operands.len() {
            let Some(right) = chain_operands(operands[second], inner_kind) else {
                continue;
            };
            for (left_index, shared) in left.iter().enumerate() {
                let Some(right_index) = right.iter().position(|candidate| candidate == shared)
                else {
                    continue;
                };
                let left_rest = join_all(without(&left, left_index), &operator_token(inner_kind))?;
                let right_rest =
                    join_all(without(&right, right_index), &operator_token(inner_kind))?;
                let combined = Expression::Binary {
                    left: Box::new(left_rest),
                    right: Box::new(right_rest),
                    operator: operator_token(kind),
                };
                let factored = Expression::Binary {
                    left: Box::new((*shared).clone()),
                    right: Box::new(combined),
                    operator: operator_token(inner_kind),
                };
                let mut kept: Vec<Expression> = operands
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != first && *index != second)
                    .map(|(_, operand)| (*operand).clone())
                    .collect();
                kept.push(factored);
                return Some(Rewrite {
                    rule: directed(Rule::FactoringB, Rule::FactoringA, kind),
                    expression: join_all(kept, operator)?,
                });
            }
        }
    }
    None
}

/// The other `&`/`|` connective.
fn dual(kind: &TokenType) -> &TokenType {
    if matches!(kind, TokenType::And) {
        &TokenType::Or
    } else {
        &TokenType::And
    }
}

/// The operands of a chain without the one at `skip`.
fn without(operands: &[&Expression], skip: usize) -> Vec<Expression> {
    operands
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != skip)
        .map(|(_, operand)| (*operand).clone())
        .collect()
}

/// The single-step expansions of the root of `expression`, in operand order:
/// law 4a for an `|` chain holding an `&` chain, law 4b for the dual (ADR-042).
fn expand_at(expression: &Expression) -> Vec<(Rule, Expression)> {
    let Expression::Binary { operator, .. } = expression else {
        return Vec::new();
    };
    let (outer_kind, inner_kind, rule) = match operator.kind {
        TokenType::Or => (TokenType::Or, TokenType::And, Rule::DistributionA),
        TokenType::And => (TokenType::And, TokenType::Or, Rule::DistributionB),
        _ => return Vec::new(),
    };
    let operands = flatten(expression, &outer_kind);
    let mut candidates = Vec::new();
    for (index, operand) in operands.iter().enumerate() {
        let Some(inner) = chain_operands(operand, &inner_kind) else {
            continue;
        };
        let mut distributed = Vec::new();
        for operand in inner {
            let mut parts = without(&operands, index);
            parts.push(operand.clone());
            if let Some(part) = join_all(parts, &operator_token(&outer_kind)) {
                distributed.push(part);
            }
        }
        if let Some(candidate) = join_all(distributed, &operator_token(&inner_kind)) {
            candidates.push((rule, candidate));
        }
    }
    candidates
}

/// Every single-step candidate, visiting the tree pre-order: a node before its
/// children, the left operand before the right one (ADR-042).
fn expansion_candidates(expression: &Expression) -> Vec<(Rule, Expression)> {
    let mut candidates = expand_at(expression);
    match expression {
        Expression::Binary {
            left,
            right,
            operator,
        } => {
            for (rule, left) in expansion_candidates(left) {
                candidates.push((
                    rule,
                    Expression::Binary {
                        left: Box::new(left),
                        right: right.clone(),
                        operator: operator.clone(),
                    },
                ));
            }
            for (rule, right) in expansion_candidates(right) {
                candidates.push((
                    rule,
                    Expression::Binary {
                        left: left.clone(),
                        right: Box::new(right),
                        operator: operator.clone(),
                    },
                ));
            }
        }
        Expression::Unary {
            operator,
            expression,
        } => {
            for (rule, inner) in expansion_candidates(expression) {
                candidates.push((
                    rule,
                    Expression::Unary {
                        operator: operator.clone(),
                        expression: Box::new(inner),
                    },
                ));
            }
        }
        _ => {}
    }
    candidates
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{Rule, Simplification, Simplifier};
    use crate::expression::{Expression, InfixPrinter};
    use crate::tokens::TokenType;

    /// Scans and parses a single proposition; the scanner and the parser have
    /// their own tests.
    fn parse_one(source: &str) -> Expression {
        let tokens = crate::scanner::scan(source.to_string()).expect("the source should scan");
        let mut expressions =
            crate::parser::Parser::parse(tokens).expect("the source should parse");
        assert_eq!(expressions.len(), 1, "source {source:?}");
        expressions.remove(0)
    }

    fn simplification(source: &str) -> Simplification {
        Simplifier::simplify(&parse_one(source))
    }

    fn simplify(source: &str) -> Expression {
        simplification(source).expression
    }

    fn infix(source: &str) -> String {
        InfixPrinter::print(&simplify(source))
    }

    fn contains_grouping(expression: &Expression) -> bool {
        match expression {
            Expression::Grouping { .. } => true,
            Expression::Binary { left, right, .. } => {
                contains_grouping(left) || contains_grouping(right)
            }
            Expression::Unary { expression, .. } => contains_grouping(expression),
            _ => false,
        }
    }

    fn contains_kind(expression: &Expression, kind: &TokenType) -> bool {
        match expression {
            Expression::Binary {
                left,
                right,
                operator,
            } => operator.kind == *kind || contains_kind(left, kind) || contains_kind(right, kind),
            Expression::Unary { expression, .. } => contains_kind(expression, kind),
            Expression::Grouping { expression } => contains_kind(expression, kind),
            _ => false,
        }
    }

    fn contains_implication_or_biconditional(expression: &Expression) -> bool {
        contains_kind(expression, &TokenType::Implication)
            || contains_kind(expression, &TokenType::Biconditional)
    }

    /// `W` is defined after grouping has been erased, so the input side of a
    /// comparison strips groupings first.
    fn input_weight(source: &str) -> u64 {
        let erased = Simplifier::erase_grouping(&parse_one(source));
        Simplifier::weight(&erased)
    }

    // --- Grouping is erased first -------------------------------------------

    #[test]
    fn redundant_groupings_are_erased() {
        assert_eq!(infix("((p))"), "p");
    }

    #[test]
    fn nested_conjunctions_are_flattened_after_erasing_grouping() {
        assert_eq!(infix("q & (p & r)"), "p ∧ q ∧ r");
    }

    #[test]
    fn the_output_contains_no_grouping_node() {
        for source in [
            "((p))",
            "q & (p & r)",
            "(p | q) & r",
            "!(p | q)",
            "p -> (q & r)",
            "p <-> (q | r)",
            "(p & q) | (p & r)",
        ] {
            let simplified = simplify(source);
            assert!(
                !contains_grouping(&simplified),
                "grouping left in {source:?}: {}",
                InfixPrinter::print(&simplified)
            );
        }
    }

    // --- One case per law ----------------------------------------------------

    #[test]
    fn law_1a_absorbs_repeated_disjuncts() {
        assert_eq!(infix("p | p"), "p");
    }

    #[test]
    fn law_1b_absorbs_repeated_conjuncts() {
        assert_eq!(infix("p & p"), "p");
    }

    #[test]
    fn law_5a_drops_false_from_a_disjunction() {
        assert_eq!(infix("p | F"), "p");
    }

    #[test]
    fn law_5b_drops_true_from_a_conjunction() {
        assert_eq!(infix("p & V"), "p");
    }

    #[test]
    fn law_6a_true_dominates_a_disjunction() {
        assert_eq!(infix("p | V"), "V");
    }

    #[test]
    fn law_6b_false_dominates_a_conjunction() {
        assert_eq!(infix("p & F"), "F");
    }

    #[test]
    fn law_7a_double_negation_is_erased() {
        assert_eq!(infix("!!p"), "p");
    }

    #[test]
    fn law_8a_a_disjunction_with_its_negation_is_true() {
        assert_eq!(infix("p | !p"), "V");
    }

    #[test]
    fn law_8b_a_conjunction_with_its_negation_is_false() {
        assert_eq!(infix("p & !p"), "F");
    }

    #[test]
    fn law_9a_negated_true_is_false() {
        assert_eq!(infix("!V"), "F");
    }

    #[test]
    fn law_9b_negated_false_is_true() {
        assert_eq!(infix("!F"), "V");
    }

    #[test]
    fn law_10a_de_morgan_pushes_negation_into_a_disjunction() {
        assert_eq!(infix("!(p | q)"), "¬p ∧ ¬q");
    }

    #[test]
    fn law_10b_de_morgan_pushes_negation_into_a_conjunction() {
        assert_eq!(infix("!(p & q)"), "¬p ∨ ¬q");
    }

    #[test]
    fn law_11a_absorbs_a_conjunction_into_a_disjunction() {
        assert_eq!(infix("p | (p & q)"), "p");
    }

    #[test]
    fn law_11b_absorbs_a_disjunction_into_a_conjunction() {
        assert_eq!(infix("p & (p | q)"), "p");
    }

    #[test]
    fn law_12a_redundancy_drops_the_negated_operand() {
        assert_eq!(infix("p | (!p & q)"), "p ∨ q");
    }

    #[test]
    fn law_12b_redundancy_drops_the_negated_operand_under_conjunction() {
        assert_eq!(infix("p & (!p | q)"), "p ∧ q");
    }

    #[test]
    fn law_13a_eliminates_implication() {
        assert_eq!(infix("p -> q"), "q ∨ ¬p");
    }

    #[test]
    fn law_14a_eliminates_biconditional() {
        assert_eq!(infix("p <-> q"), "(p ∨ ¬q) ∧ (q ∨ ¬p)");
    }

    #[test]
    fn law_4a_factors_a_shared_conjunct() {
        assert_eq!(infix("(p & q) | (p & r)"), "p ∧ (q ∨ r)");
    }

    #[test]
    fn law_4b_factors_a_shared_disjunct() {
        // The printer drops the parentheses around `q ∧ r`: conjunction binds
        // tighter, so they are redundant.
        assert_eq!(infix("(p | q) & (p | r)"), "p ∨ q ∧ r");
    }

    // --- Examples table ------------------------------------------------------

    #[test]
    fn the_design_examples_simplify_to_the_documented_forms() {
        assert_eq!(infix("!!p"), "p");
        assert_eq!(infix("p & F"), "F");
        assert_eq!(infix("p | V"), "V");
        assert_eq!(infix("!(p | q)"), "¬p ∧ ¬q");
        assert_eq!(infix("(p & q) | (p & r)"), "p ∧ (q ∨ r)");
        assert_eq!(infix("!(!p | !q)"), "p ∧ q");
        assert_eq!(infix("p | (!p & q)"), "p ∨ q");
        assert_eq!(infix("p -> q"), "q ∨ ¬p");
        assert_eq!(infix("p <-> q"), "(p ∨ ¬q) ∧ (q ∨ ¬p)");
        assert_eq!(infix("(p & q) | (p & !q)"), "p");
        assert_eq!(infix("q & (p & r)"), "p ∧ q ∧ r");
        assert_eq!(infix("p & (q | r)"), "p ∧ (q ∨ r)");
    }

    // --- Conditional and biconditional elimination --------------------------

    #[test]
    fn the_output_never_contains_an_implication_or_a_biconditional() {
        for source in [
            "p -> q",
            "p <-> q",
            "p -> (q -> r)",
            "(p -> q) <-> (q -> p)",
            "!((p <-> q) | r)",
            "p & (q <-> r) | s -> t",
        ] {
            let simplified = simplify(source);
            assert!(
                !contains_implication_or_biconditional(&simplified),
                "conditional left in {source:?}: {}",
                InfixPrinter::print(&simplified)
            );
        }
    }

    // --- Canonical order -----------------------------------------------------

    #[test]
    fn operand_order_is_structural() {
        assert_eq!(simplify("q & (p & r)"), simplify("r & (q & p)"));
    }

    #[test]
    fn constants_sort_false_before_true() {
        // A chain holding both constants collapses by the identity and
        // domination laws, so the order is asserted on the comparator itself.
        assert_eq!(
            Simplifier::compare(&parse_one("F"), &parse_one("V")),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn a_chain_with_both_constants_collapses() {
        assert_eq!(infix("q & p & !p & (r | s) & V"), "F");
        assert_eq!(infix("V & F & p"), "F");
    }

    #[test]
    fn variables_sort_by_ascii_code_point() {
        assert_eq!(infix("q & p"), "p ∧ q");
        assert_eq!(infix("p & P"), "P ∧ p");
        assert_eq!(infix("p1 & p"), "p ∧ p1");
    }

    // --- Idempotence ---------------------------------------------------------

    #[test]
    fn simplifying_a_simplified_expression_changes_nothing() {
        for source in [
            "p",
            "!!p",
            "p | (!p & q)",
            "p -> q",
            "p <-> q",
            "!(p | q)",
            "(p & q) | (p & r)",
            "!(!p | !q)",
            "p & (q | r)",
            "q & (p & r)",
        ] {
            let once = simplify(source);
            assert_eq!(
                Simplifier::simplify(&once).expression,
                once,
                "not idempotent on {source:?}: {}",
                InfixPrinter::print(&once)
            );
        }
    }

    // --- Termination and the expansion phase --------------------------------

    #[test]
    fn shapes_that_could_oscillate_terminate() {
        let cases = [
            "p & (q | r)",
            "!(!(p & q) | !(r & s))",
            "!!p",
            "(p & q) | (p & r)",
            "((p | q) & (p | r)) & (p | s)",
            "(p & q) | (p & !q) | (r & s)",
        ];
        for source in cases {
            let simplified = simplify(source);
            assert_eq!(
                Simplifier::simplify(&simplified).expression,
                simplified,
                "unstable fixed point on {source:?}"
            );
        }
    }

    #[test]
    fn expansion_keeps_a_formula_that_does_not_get_lighter() {
        assert_eq!(infix("p & (q | r)"), "p ∧ (q ∨ r)");
    }

    #[test]
    fn nested_de_morgan_pushes_every_negation_to_the_leaves() {
        assert_eq!(infix("!(!(p & q) | !(r & s))"), "p ∧ q ∧ r ∧ s");
    }

    // --- Process list --------------------------------------------------------

    #[test]
    fn de_morgan_and_two_double_negations_are_recorded_in_order() {
        let result = simplification("!(!p | !q)");

        assert_eq!(
            result
                .steps
                .iter()
                .map(|step| step.rule)
                .collect::<Vec<_>>(),
            [Rule::DeMorganA, Rule::DoubleNegation, Rule::DoubleNegation]
        );
        assert_eq!(
            result
                .steps
                .iter()
                .map(|step| InfixPrinter::print(&step.expression))
                .collect::<Vec<_>>(),
            ["¬¬p ∧ ¬¬q", "p ∧ ¬¬q", "p ∧ q"]
        );
        assert_eq!(InfixPrinter::print(&result.expression), "p ∧ q");
    }

    #[test]
    fn implication_elimination_is_a_single_conditional_step() {
        let result = simplification("p -> q");

        assert_eq!(result.steps.len(), 1);
        assert_eq!(result.steps[0].rule, Rule::Conditional);
        assert_eq!(InfixPrinter::print(&result.steps[0].expression), "q ∨ ¬p");
        assert_eq!(result.steps[0].expression, result.expression);
    }

    #[test]
    fn a_formula_that_needs_no_law_has_no_steps() {
        assert!(simplification("p").steps.is_empty());

        // Canonicalization reorders the operands but is not a step.
        let reordered = simplification("q & p");
        assert_eq!(InfixPrinter::print(&reordered.expression), "p ∧ q");
        assert!(reordered.steps.is_empty());
    }

    #[test]
    fn every_recorded_step_is_canonical() {
        for source in [
            "!(!p | !q)",
            "p -> q",
            "p <-> q",
            "p | (!p & q)",
            "(p & q) | (p & r)",
            "!(!(p & q) | !(r & s))",
        ] {
            let result = simplification(source);
            assert!(!result.steps.is_empty(), "no steps recorded for {source:?}");
            for step in result.steps {
                assert_eq!(
                    Simplifier::canonicalize(&step.expression),
                    step.expression,
                    "step {} is not canonical for {source:?}",
                    step.rule.label()
                );
                assert!(
                    !contains_grouping(&step.expression),
                    "grouping left in step {} for {source:?}",
                    step.rule.label()
                );
            }
        }
    }

    #[test]
    fn the_last_step_is_the_returned_expression() {
        for source in [
            "p",
            "!(!p | !q)",
            "p -> q",
            "p <-> q",
            "p | (!p & q)",
            "(p & q) | (p & r)",
            "((p | q) & (p | r)) & (p | s)",
            "p & (q | r)",
        ] {
            let result = simplification(source);
            if let Some(last) = result.steps.last() {
                assert_eq!(
                    last.expression, result.expression,
                    "last step differs from the result for {source:?}"
                );
            }
        }
    }

    #[test]
    fn a_rejected_expansion_adds_no_distribution_step() {
        let result = simplification("p & (q | r)");

        assert_eq!(InfixPrinter::print(&result.expression), "p ∧ (q ∨ r)");
        assert!(
            result.steps.is_empty(),
            "discarded expansion left steps: {:?}",
            result
                .steps
                .iter()
                .map(|step| step.rule.label())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rule_labels_are_the_law_number_and_the_spanish_name() {
        let labels = [
            (Rule::IdempotenceA, "1a. Idempotencia"),
            (Rule::IdempotenceB, "1b. Idempotencia"),
            (Rule::IdentityA, "5a. Identidad"),
            (Rule::IdentityB, "5b. Identidad"),
            (Rule::DominationA, "6a. Dominación"),
            (Rule::DominationB, "6b. Dominación"),
            (Rule::DoubleNegation, "7a. Doble negación"),
            (Rule::ComplementA, "8a. Complemento"),
            (Rule::ComplementB, "8b. Complemento"),
            (Rule::NegatedTrue, "9a. Complemento"),
            (Rule::NegatedFalse, "9b. Complemento"),
            (Rule::DeMorganA, "10a. De Morgan"),
            (Rule::DeMorganB, "10b. De Morgan"),
            (Rule::AbsorptionA, "11a. Absorción"),
            (Rule::AbsorptionB, "11b. Absorción"),
            (Rule::RedundancyA, "12a. Redundancia"),
            (Rule::RedundancyB, "12b. Redundancia"),
            (Rule::Conditional, "13a. Condicional"),
            (Rule::Biconditional, "14a. Bicondicional"),
            (Rule::FactoringA, "4a. Factorización"),
            (Rule::FactoringB, "4b. Factorización"),
            (Rule::DistributionA, "4a. Distribución"),
            (Rule::DistributionB, "4b. Distribución"),
        ];

        for (rule, label) in labels {
            assert_eq!(rule.label(), label, "wrong label for {rule:?}");
        }
    }

    #[test]
    fn factoring_a_disjunction_is_law_4b() {
        // `design/simplifier.md` reads law 4b right to left for
        // `(p & q) | (p & r) = p & (q | r)`.
        let result = simplification("(p & q) | (p & r)");

        assert_eq!(
            result
                .steps
                .iter()
                .map(|step| step.rule)
                .collect::<Vec<_>>(),
            [Rule::FactoringB]
        );
    }

    #[test]
    fn factoring_a_conjunction_is_law_4a() {
        // `design/simplifier.md` reads law 4a right to left for
        // `(p | q) & (p | r) = p | (q & r)`.
        let result = simplification("(p | q) & (p | r)");

        assert_eq!(
            result
                .steps
                .iter()
                .map(|step| step.rule)
                .collect::<Vec<_>>(),
            [Rule::FactoringA]
        );
    }

    // --- Chain rules over longer chains --------------------------------------

    #[test]
    fn absorption_matches_inside_a_longer_disjunction() {
        assert_eq!(infix("p | q | (p & s)"), "p ∨ q");
    }

    #[test]
    fn absorption_matches_inside_a_longer_conjunction() {
        assert_eq!(infix("p & r & (p | s)"), "p ∧ r");
    }

    // --- Weight --------------------------------------------------------------

    #[test]
    fn weight_matches_the_design_definition() {
        assert_eq!(Simplifier::weight(&parse_one("p")), 1);
        assert_eq!(Simplifier::weight(&parse_one("!p")), 2);
        assert_eq!(Simplifier::weight(&parse_one("p & q")), 3);
        assert_eq!(Simplifier::weight(&parse_one("p -> q")), 5);
        assert_eq!(Simplifier::weight(&parse_one("p <-> q")), 10);
    }

    #[test]
    fn simplification_never_increases_the_weight() {
        for source in [
            "p",
            "!p",
            "p & q",
            "p | q",
            "p -> q",
            "p <-> q",
            "p | (!p & q)",
            "!(p | q)",
            "p & (q | r)",
            "(p & q) | (p & r)",
            "q & (p & r)",
            "!(!p | !q)",
            "p -> (q & r)",
            "(p -> q) & (q -> p)",
        ] {
            let simplified = simplify(source);
            assert!(
                Simplifier::weight(&simplified) <= input_weight(source),
                "weight grew on {source:?}: {} ({} > {})",
                InfixPrinter::print(&simplified),
                Simplifier::weight(&simplified),
                input_weight(source)
            );
        }
    }

    // --- Total order ---------------------------------------------------------

    /// One shape per canonical-order kind: constants, variables, a negation,
    /// `&` chains of two and three operands, `|` chains (plain and with a
    /// negated operand), an implication and a biconditional. None wrap a
    /// `Grouping`, so equality and the comparator agree on what "different"
    /// means. The negated chain operands matter: with the old left-child
    /// against right-child pairing, `p | !q` and `q | !p` compared `Less` in
    /// both directions.
    fn canonical_order_shapes() -> Vec<Expression> {
        [
            "F",
            "V",
            "P",
            "p",
            "p1",
            "!p",
            "p & q",
            "p & q & r",
            "p | q",
            "p | !q",
            "q | !p",
            "p -> q",
            "p <-> q",
        ]
        .into_iter()
        .map(parse_one)
        .collect()
    }

    #[test]
    fn canonical_order_is_antisymmetric_on_representative_shapes() {
        let shapes = canonical_order_shapes();
        for left in &shapes {
            for right in &shapes {
                let forward = Simplifier::compare(left, right);
                let backward = Simplifier::compare(right, left);
                assert_eq!(
                    forward,
                    backward.reverse(),
                    "compare is not antisymmetric on {} and {}",
                    InfixPrinter::print(left),
                    InfixPrinter::print(right)
                );
                if left == right {
                    assert_eq!(
                        forward,
                        Ordering::Equal,
                        "equal expressions do not compare equal: {}",
                        InfixPrinter::print(left)
                    );
                } else {
                    assert_ne!(
                        forward,
                        Ordering::Equal,
                        "different expressions compare equal: {} and {}",
                        InfixPrinter::print(left),
                        InfixPrinter::print(right)
                    );
                }
            }
        }
    }

    #[test]
    fn canonical_order_is_transitive_on_representative_shapes() {
        let shapes = canonical_order_shapes();
        let at_most = |left: &Expression, right: &Expression| {
            Simplifier::compare(left, right) != Ordering::Greater
        };
        for a in &shapes {
            for b in &shapes {
                if !at_most(a, b) {
                    continue;
                }
                for c in &shapes {
                    if !at_most(b, c) {
                        continue;
                    }
                    assert!(
                        at_most(a, c),
                        "order is not transitive: {} <= {} <= {} but not {} <= {}",
                        InfixPrinter::print(a),
                        InfixPrinter::print(b),
                        InfixPrinter::print(c),
                        InfixPrinter::print(a),
                        InfixPrinter::print(c)
                    );
                }
            }
        }
    }

    #[test]
    fn chain_order_comes_from_the_flattened_operands() {
        // The old comparator paired the left child of one chain with the
        // right child of the other, so both directions could be `Less`.
        // Here the flattened lists are `p, !q` and `q, !p`: the first
        // operands decide, and the answer flips with the argument order.
        let disjunction = parse_one("p | !q");
        let swapped = parse_one("q | !p");
        assert_eq!(Simplifier::compare(&disjunction, &swapped), Ordering::Less);
        assert_eq!(
            Simplifier::compare(&swapped, &disjunction),
            Ordering::Greater
        );

        // Same check with three operands: `p, q, r` versus `q, p, r`, even
        // though the two trees are stored left-nested in different shapes.
        let conjunction = parse_one("p & q & r");
        let swapped = parse_one("q & p & r");
        assert_eq!(Simplifier::compare(&conjunction, &swapped), Ordering::Less);
        assert_eq!(
            Simplifier::compare(&swapped, &conjunction),
            Ordering::Greater
        );
    }

    #[test]
    fn operand_construction_order_does_not_change_the_simplified_tree() {
        let canonical = simplify("p & q");
        assert_eq!(simplify("q & p"), canonical);
        assert_eq!(
            InfixPrinter::print(&canonical),
            "p ∧ q",
            "the shared tree should be the canonical order"
        );
        assert_eq!(Simplifier::simplify(&canonical).expression, canonical);
    }
}

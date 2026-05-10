# grammar-fuzz — DCG companion to the M lexer/parser

Independent second reading of the M language grammar, written as a Prolog DCG
in [scryer-prolog](https://github.com/mthom/scryer-prolog). Used for:

1. **Spec sanity check** — writing the grammar in a different paradigm forces
   every ambiguity in the spec to be answered.
2. **Fuzz input generation** — DCGs are reversible; the same predicates that
   parse can also generate random valid source.
3. **Differential testing** — token streams (and later AST shapes) from the
   Rust implementation are diffed against the DCG output. Any divergence is a
   bug somewhere; investigate.

This is a development tool. It is **not** a runtime dependency of `mrsflow-core`,
the CLI, or the WASM build.

## Files

- `lexical.pl` — DCG for the lexical grammar. Mirror of `mrsflow-core/src/lexer/`.
- `lex_cli.pl` — Tiny CLI driver: reads M source from stdin, prints one token per line.

## Running

```sh
echo 'let x = 1 + 2 in x' | scryer-prolog -f --no-add-history lex_cli.pl
```

## Scope

Mirrors the Rust lexer's slice 1: comments, decimal numbers, text literals with
`""` escape, the core keywords (`let in if then else true false null`), dotted
identifiers, the slice-1 operators (`= + - * / & ( ) [ ] { } , ;`).

Slice 2 (hex, exponents, `#(...)` escapes, quoted identifiers, `#`-keywords,
remaining operators) lands here in lockstep with the Rust side so the
differential harness has equal coverage on both ends.

# explain — plain-English diagnoses for broken M

A Prolog tool that takes a piece of Power Query M source and, when it
recognises a common mistake, prints a one-paragraph diagnosis plus a
fix-it diff. The audience is Excel / Power BI users hitting cryptic PQ
errors with no idea what to do.

This is a development prototype. The intended final shape is a web
service: paste your broken M (plus optionally the PQ error message you
saw), get the diagnosis back. The CGI server isn't built yet — what
lives here is the engine and the rule catalogue.

## Files

- `error_rules.pl` — the catalogue. Each `rule/3` clause is one
  recognised mistake plus its diagnosis and fix-it. **This is the
  product.** Adding a rule = appending a clause.
- `explain.pl` — the driver. Lexes the input (via the existing
  `tools/grammar-fuzz/lexical.pl`), walks the catalogue, prints the
  first matching diagnosis or an honest "I don't recognise this one."
- `explain.sh` — wrapper script. Loads everything in the right order
  and invokes scryer-prolog.

## Running

```sh
echo 'let x = 1; in x' > /tmp/broken.m
tools/explain/explain.sh /tmp/broken.m
```

Output:

```
[semicolon_in_let] Semicolon used as binding separator in `let`

  M separates `let` bindings with commas, not semicolons. ...

  Before: let x = 1; in x
  After:  let x = 1, in x
```

## Adding a rule

Open `error_rules.pl`, append a `rule/3` clause:

```prolog
rule(unique_id,
     [ token_pattern_with_holes_via_underscores ],
     diag("Short title",
          "One or two sentences of plain English. No PQ jargon. Show the fix.",
          fix("wrong source",
              "correct source"))).
```

Test:

```sh
echo 'your test input' > /tmp/case.m && tools/explain/explain.sh /tmp/case.m
```

If no rule fires for a real broken case you care about, that's a
missing rule, not a bug — add one.

## Why Prolog

Definite-clause grammars are the natural shape for "match a wrong-shape
token pattern and emit a structured diagnosis." Each rule is one
declarative clause. The catalogue doubles as the spec — anyone reading
`error_rules.pl` can see exactly which mistakes the tool recognises,
and the same file is what you'd ship to a developer asking
"what does it catch?"

The grammar (`tools/grammar-fuzz/lexical.pl`) already existed as a
differential test for the Rust lexer. Reusing it here means the
explainer and the runtime agree on what the source means down to the
token level — divergence between them would be a bug in one of them.

## Status

- 13 rules. 7 source-keyed (fire from M shape alone), 6 error-keyed
  (require the user to also paste the PQ error string). Seeded from
  cheat-sheet pages that already collate real user reports.
- Regression suite under `test_cases/` (one `.m` per rule, plus
  matching `.err` for error-keyed rules). Run `run_tests.sh` before
  deploying — 13/13 pass today.
- Real-corpus false-positive check via `pump_corpus.sh`: 0/19
  legitimate Power Query files in `examples/powerqueries/` trigger
  any rule, so the patterns are tight enough to leave real M alone.
- Live at <https://dw.ramsden-international.com/m-explain.html>.
- API: `POST /m-explain` with either plain M source or JSON
  `{source, error}`. Plain text returns plain text; JSON returns the
  same plain text but the optional `error` field can sharpen which rule
  fires (see "match priority" in `error_rules.pl`).
- Server side: bash CGI wrapper (`deploy/explain-m.cgi`) → scryer-prolog
  running `cgi.pl`. The wrapper extracts source + error via `jq` since
  scryer 0.10 has no JSON library; source is piped to scryer on stdin,
  error file path passed via `EXPLAIN_M_ERROR_FILE` env var.
- Requires scryer-prolog >= 0.10.0-162-g8dffd72d. Earlier 0.10 had
  broken stdin EOF on pipes which forced a temp-file workaround.

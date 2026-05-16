% error_rules.pl — catalogue of common M syntax/semantic mistakes.
%
% Each rule has the shape:
%
%   rule(Id, Tokens, ErrorConstraints, Diagnosis)
%
% where:
%   Tokens             - partial token-stream pattern, matched as a
%                        contiguous sub-sequence anywhere in the stream.
%                        Use ident(_), text(_), number(_) etc. as holes;
%                        use the empty list [] to match always (rely on
%                        the error string).
%   ErrorConstraints   - list of constraints on the PQ error string AND
%                        the token stream:
%                          contains("substring") - error contains it
%                          equals("text")        - error equals it exactly
%                          not_followed_by(Pat)  - the matched pattern
%                                                  must NOT be followed
%                                                  later in the stream by
%                                                  the given pattern.
%                                                  Used for "saw X but
%                                                  never saw Y" rules.
%                        empty list = no constraint, just match the
%                        pattern.
%   Diagnosis          - diag(Title, Explanation, FixIt) record.
%                        FixIt is fix(Before, After) or `none`.
%
% Match priority: a rule with any non-empty ErrorConstraints (whose
% constraints all hold) beats an unconstrained rule that also matches.
% This lets generic shape-rules co-exist with sharpened variants keyed
% on the exact PQ error string.
%
% Two rule populations:
%   - Source-keyed: pattern is distinctive, error constraint is empty
%     (or just a not_followed_by). Fire purely from the M shape. Most
%     syntax mistakes are here.
%   - Error-keyed: pattern is [] (matches always), constraints depend
%     on the PQ error string. Fire only when the user pasted an error
%     too. Catch evaluator-level mistakes (field not found, wrong arg
%     count) that don't show in the token stream.

:- use_module(library(lists)).

% --- Public API ---

% match_rule(+Tokens, +ErrorChars, -Id, -Diag).
match_rule(Tokens, ErrorChars, Id, Diag) :-
    rule(Id, Pattern, Constraints, Diag),
    Constraints \= [],
    pattern_in_tokens(Pattern, Tokens),
    constraints_hold(Constraints, Tokens, ErrorChars),
    !.
match_rule(Tokens, _ErrorChars, Id, Diag) :-
    rule(Id, Pattern, [], Diag),
    Pattern \= [],
    pattern_in_tokens(Pattern, Tokens),
    !.

% pattern_in_tokens(+Pattern, +Tokens). True if Pattern appears as a
% contiguous sub-sequence of Tokens. [] matches trivially.
pattern_in_tokens([], _) :- !.
pattern_in_tokens(Pattern, Tokens) :-
    append(_Prefix, Tail, Tokens),
    append(Pattern, _Suffix, Tail),
    !.

% constraints_hold(+Constraints, +Tokens, +ErrorChars).
constraints_hold([], _, _).
constraints_hold([C|Cs], Tokens, ErrorChars) :-
    constraint_holds(C, Tokens, ErrorChars),
    constraints_hold(Cs, Tokens, ErrorChars).

constraint_holds(contains(Sub), _Tokens, ErrorChars) :-
    append(_, Tail, ErrorChars),
    append(Sub, _, Tail),
    !.
constraint_holds(equals(Exact), _Tokens, ErrorChars) :-
    ErrorChars == Exact.
constraint_holds(not_followed_by(Pat), Tokens, _ErrorChars) :-
    % "Saw the rule's anchor pattern (matched by pattern_in_tokens above)
    % AND no occurrence of Pat anywhere in the stream." For the rules we
    % currently express (`if` without `else`/`then`) checking the whole
    % stream is fine — the anchor already established we saw `if`.
    \+ pattern_in_tokens(Pat, Tokens).

% all_rules(-Ids). Inventory for CLI listing.
all_rules(Ids) :-
    findall(Id, rule(Id, _, _, _), Ids).

% =========================================================================
% Rules: source-keyed (token-pattern only)
% =========================================================================

% R001: `let x = 1; in x` — semicolon used as binding separator.
rule(semicolon_in_let,
     [keyword(let), ident(_), op(equals), _, op(semicolon)],
     [],
     diag("Semicolon used as binding separator in `let`",
          "M separates `let` bindings with commas, not semicolons. Semicolons end statements at the top level of a query, but inside a `let` you write `let a = 1, b = 2 in ...`.",
          fix("let x = 1; in x", "let x = 1, in x"))).

% R002: `{ 1, 3, }` — trailing comma in list literal.
rule(trailing_comma_in_list,
     [op(comma), op(rbrace)],
     [],
     diag("Trailing comma before `}` in list",
          "List literals can't end with a comma. M is stricter than JavaScript here: remove the comma immediately before the closing `}`.",
          fix("{ 1, 2, 3, }", "{ 1, 2, 3 }"))).

% R003: `[ a = 1, b = 2, ]` — trailing comma in record literal.
rule(trailing_comma_in_record,
     [op(comma), op(rbracket)],
     [],
     diag("Trailing comma before `]` in record",
          "Record literals can't end with a comma. Remove the comma immediately before the closing `]`.",
          fix("[ a = 1, b = 2, ]", "[ a = 1, b = 2 ]"))).

% R004: `let Var1 = 1, Var2 = 2, in Var2` — trailing comma before `in`.
rule(trailing_comma_in_let,
     [op(comma), keyword(in)],
     [],
     diag("Trailing comma before `in`",
          "The last binding in a `let` block must not have a trailing comma. Bindings are comma-*separated*, not comma-*terminated*.",
          fix("let a = 1, b = 2, in b", "let a = 1, b = 2 in b"))).

% R005: `if 5 > 4 then 6` — `if` without `else`.
% Anchor on `then` (not bare `if`) so we don't fire on `Date.IsInNextNDays`
% style identifiers that contain the substring "if" — wait, identifiers
% can't be `if` because `if` is a keyword. Anchor on `if` is fine.
rule(if_without_else,
     [keyword(if)],
     [not_followed_by([keyword(else)])],
     diag("`if` expression has no `else` branch",
          "M's `if` is an expression, not a statement: it must always produce a value, so every `if` requires both `then` and `else`. Add `else <default-value>` to the end.",
          fix("if x > 0 then x", "if x > 0 then x else 0"))).

% R006: `if 5 > 4 else 6` — `if` without `then`.
rule(if_without_then,
     [keyword(if)],
     [not_followed_by([keyword(then)])],
     diag("`if` expression has no `then` branch",
          "M's `if` syntax is `if <condition> then <value> else <value>`. The `then` keyword is required between the condition and the true branch.",
          fix("if x > 0 else 0", "if x > 0 then x else 0"))).

% R007: semicolon used between record fields.
rule(semicolon_in_record,
     [op(semicolon), ident(_), op(equals)],
     [],
     diag("Semicolon used as field separator in record",
          "Record fields are separated by commas, not semicolons. The semicolon is reserved for ending top-level statements.",
          fix("[a = 1; b = 2]", "[a = 1, b = 2]"))).

% =========================================================================
% Rules: error-keyed (Tier-2 — rely on the pasted PQ error string)
% =========================================================================

% R100: "The field 'X' of the record wasn't found."
rule(field_not_found,
     [],
     [contains("field"), contains("wasn't found")],
     diag("Record field doesn't exist (typo?)",
          "PQ tried to look up a field by name and couldn't find it. Check the field-access expression `[Name]` — is it spelled exactly as the record / column defines it? Power Query field names are case-sensitive.",
          none)).

% R101: "N arguments were passed to a function which expects between X and Y."
rule(wrong_arg_count,
     [],
     [contains("arguments were passed to a function")],
     diag("Wrong number of arguments to a stdlib function",
          "You're calling a function with the wrong number of arguments. The error message lists how many it expected — check the function's documentation for which args are required vs optional.",
          none)).

% R102: "We cannot apply operator X to types Y and Z."
rule(operator_type_mismatch,
     [],
     [contains("cannot apply operator")],
     diag("Operator can't combine these types",
          "The two operands have types the operator isn't defined for. For arithmetic that usually means a Number on one side and a Date / Text / null on the other. Wrap or convert one side: `Number.From(...)`, `Text.From(...)`, `Date.From(...)`.",
          none)).

% R103: "The name 'X' wasn't recognized..."
rule(name_not_recognised,
     [],
     [contains("wasn't recognized")],
     diag("Identifier not recognised",
          "A name in the expression doesn't refer to anything in scope. Possible causes: (1) typo — check spelling and case; (2) you wrote `[ColName]` outside an `each` lambda — wrap the predicate in `each`; (3) you referenced a step name that hasn't been defined yet.",
          none)).

% R104: "There weren't enough elements in the enumeration..."
rule(list_index_out_of_range,
     [],
     [contains("enough elements in the enumeration")],
     diag("List index out of range",
          "You indexed past the end of a list or column. M lists are zero-indexed: `{ \"a\", \"b\" }{0}` is the first element, `{2}` is past the end of a two-element list. Check the length with `List.Count(...)` first if the size is dynamic.",
          none)).

% R105: "We couldn't parse the input provided as a <type> value." (DataFormat.Error)
rule(data_format_error,
     [],
     [contains("couldn't parse the input")],
     diag("Value couldn't be parsed as the requested type",
          "A conversion function (`Date.From`, `Number.From`, etc.) was given input it doesn't recognise as that type. For dates, check the text format matches what PQ accepts (or use `Date.FromText(_, [Format=...])` with an explicit pattern). For numbers, check culture-specific separators (`,` vs `.` for decimal).",
          none)).

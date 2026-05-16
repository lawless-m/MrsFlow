% error_rules.pl — catalogue of common M syntax/semantic mistakes.
%
% Each rule has the shape:
%
%   rule(Id, Tokens, ErrorConstraints, Diagnosis)
%
% where:
%   Tokens             - partial token-stream pattern, matched anywhere
%   ErrorConstraints   - list of constraints on the PQ error string:
%                          contains("substring") - error must contain it
%                          equals("text")        - exact match
%                        empty list means "no constraint"
%   Diagnosis          - diag(Title, Explanation, FixIt) record
%
% Match priority: a rule whose ErrorConstraints all hold beats a rule with
% an empty constraint list, even if both match the token pattern. This lets
% generic shape-rules co-exist with sharpened variants keyed on the exact
% PQ error string.
%
% Adding a rule = append a rule/4 clause. Order matters only for ties.

:- use_module(library(lists)).

% --- Public API ---

% match_rule(+Tokens, +ErrorChars, -Id, -Diag).
% Find the best-matching rule. Constrained rules (whose error-constraints
% all satisfy ErrorChars) win over unconstrained rules.
%
% ErrorChars is a chars-list (possibly empty if no PQ error supplied).
match_rule(Tokens, ErrorChars, Id, Diag) :-
    rule(Id, Pattern, Constraints, Diag),
    pattern_in_tokens(Pattern, Tokens),
    Constraints \= [],
    constraints_hold(Constraints, ErrorChars),
    !.
match_rule(Tokens, _ErrorChars, Id, Diag) :-
    rule(Id, Pattern, [], Diag),
    pattern_in_tokens(Pattern, Tokens),
    !.

% pattern_in_tokens(+Pattern, +Tokens). True if Pattern appears as a
% sub-sequence of Tokens (any starting position).
pattern_in_tokens(Pattern, Tokens) :-
    append(_Prefix, Tail, Tokens),
    append(Pattern, _Suffix, Tail),
    !.

% constraints_hold(+Constraints, +ErrorChars). True if every constraint
% in the list is satisfied by ErrorChars.
constraints_hold([], _).
constraints_hold([C|Cs], ErrorChars) :-
    constraint_holds(C, ErrorChars),
    constraints_hold(Cs, ErrorChars).

constraint_holds(contains(Sub), ErrorChars) :-
    append(_, Tail, ErrorChars),
    append(Sub, _, Tail),
    !.
constraint_holds(equals(Exact), ErrorChars) :-
    ErrorChars == Exact.

% all_rules(-Ids). Inventory for CLI listing.
all_rules(Ids) :-
    findall(Id, rule(Id, _, _, _), Ids).

% --- Rules ---

% R001: `let x = ... ; in x`  — semicolon used as separator inside a let.
% PQ uses comma between bindings; semicolon ends a let-in expression at the
% top level only and isn't a binding separator.
rule(semicolon_in_let,
     [keyword(let), ident(_), op(equals), _, op(semicolon)],
     [],
     diag("Semicolon used as binding separator in `let`",
          "M separates `let` bindings with commas, not semicolons. Semicolons end statements at the top level of a query, but inside a `let` you write `let a = 1, b = 2 in ...`.",
          fix("let x = 1; in x", "let x = 1, in x"))).

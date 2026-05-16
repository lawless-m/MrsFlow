% explain.pl — read M source (and optionally a PQ error string),
% write a diagnosis to stdout.
%
% Pipeline:
%   1. Lex the source via lexical.pl.
%   2. If lex fails, print a generic lex-error message and exit.
%   3. Walk the catalogue in error_rules.pl. Rules with error-constraints
%      get first pick if those constraints match the supplied error.
%   4. If no rule fires, say so honestly.

:- use_module(library(format)).
:- use_module(library(lists)).
:- use_module(library(pio)).

% Single-arg form: keep working for callers that don't have an error
% string. Equivalent to passing an empty error.
explain_file(SrcPath) :-
    explain_file_with_error(SrcPath, none).

% Two-arg form: error_arg is either none, or file(ErrPath).
% Reading the error from a temp file (rather than as a Prolog atom on the
% command line) keeps quoting sane.
explain_file_with_error(SrcPath, ErrArg) :-
    phrase_from_file(tokens(Tokens), SrcPath),
    error_chars(ErrArg, ErrorChars),
    explain_tokens(Tokens, ErrorChars).

error_chars(none, []).
error_chars(file(Path), Chars) :-
    phrase_from_file(all_chars(Chars), Path).

% Read a file's contents as a flat chars-list. Local DCG, named to avoid
% the seq//1 already defined in lexical.pl.
all_chars([])     --> [].
all_chars([C|Cs]) --> [C], all_chars(Cs).

explain_tokens(Tokens, ErrorChars) :-
    ( match_rule(Tokens, ErrorChars, Id, Diag) ->
        print_diagnosis(Id, Diag)
    ; format("No known-mistake pattern matched. Source lexed cleanly.~n", [])
    ).

print_diagnosis(Id, diag(Title, Explanation, FixIt)) :-
    format("[~w] ~s~n", [Id, Title]),
    format("~n", []),
    format("  ~s~n", [Explanation]),
    format("~n", []),
    print_fixit(FixIt).

print_fixit(none) :- !.
print_fixit(fix(Before, After)) :-
    format("  Before: ~s~n", [Before]),
    format("  After:  ~s~n", [After]).

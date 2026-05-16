% cgi.pl — CGI entry point for the explainer.
%
% Reads M source from stdin (Apache per-request invocation). The optional
% PQ error string, when provided, lives in a temp file whose path is
% passed via the EXPLAIN_M_ERROR_FILE environment variable. Keeping the
% error out-of-band (env var, not command line) means we don't have to
% worry about shell-escaping a user-supplied string into the -g goal.
%
% Response: plain text, single block of body — diagnosis or "no match"
% or "lex error". CGI header is emitted by the calling bash wrapper.
%
% scryer >=0.10.0-162-g8dffd72d required: earlier 0.10.0 had broken
% stdin-EOF on pipes, which forced an earlier temp-file workaround.

:- use_module(library(charsio)).
:- use_module(library(format)).
:- use_module(library(iso_ext)).
:- use_module(library(lists)).
:- use_module(library(os)).
:- use_module(library(pio)).

cgi_main :-
    catch(
        ( phrase_from_stream(tokens(Tokens), user_input),
          error_chars_from_env(ErrorChars),
          dispatch(Tokens, ErrorChars)
        ),
        Err,
        format("Lex error: ~q~n", [Err])
    ).

% If EXPLAIN_M_ERROR_FILE is set and points to a non-empty file, read
% its contents as the PQ error string. Otherwise empty.
error_chars_from_env(Chars) :-
    ( catch(getenv("EXPLAIN_M_ERROR_FILE", Path), _, fail),
      Path \= []
    -> phrase_from_file(all_chars(Chars), Path)
    ;  Chars = []
    ).

all_chars([])     --> [].
all_chars([C|Cs]) --> [C], all_chars(Cs).

dispatch(Tokens, ErrorChars) :-
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

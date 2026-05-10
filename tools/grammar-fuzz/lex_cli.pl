% lex_cli.pl — read M source from stdin, print one token per line.
%
% Format: matches mrsflow-core's tokenize output for differential comparison.
%
% Run: scryer-prolog -f --no-add-history -g main -t halt lex_cli.pl

:- use_module(library(charsio)).
:- use_module(library(format)).
:- use_module(library(iso_ext)).
:- use_module(library(lists)).
:- use_module('lexical.pl').

main :-
    read_all_chars(Cs),
    ( tokenize(Cs, Ts)
    -> print_tokens(Ts)
    ;  format("LEX ERROR~n", [])
    ).

read_all_chars(Cs) :-
    get_char(C),
    ( C == end_of_file -> Cs = []
    ; Cs = [C|Rest], read_all_chars(Rest)
    ).

print_tokens([]).
print_tokens([T|Ts]) :- print_token(T), print_tokens(Ts).

print_token(keyword(K))  :- format("Keyword ~w~n", [K]).
print_token(ident(Cs))   :- format("Identifier ~s~n", [Cs]).
print_token(number(Cs))  :- format("Number ~s~n", [Cs]).
print_token(text(Cs))    :- format("Text ~s~n", [Cs]).
print_token(op(O))       :- format("Op ~w~n", [O]).

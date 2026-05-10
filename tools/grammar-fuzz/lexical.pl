% lexical.pl — DCG for the M language lexical grammar (slice 1).
%
% Reference: Microsoft Power Query M language specification, lexical structure.
% Mirror of mrsflow-core/src/lexer/. Scope is intentionally identical.
%
% Token term shape:
%   keyword(Name)   - let | in | if | then | else | true | false | null
%   ident(Chars)    - regular identifier, possibly dotted
%   number(Chars)   - decimal literal, raw lexeme
%   text(Chars)     - text literal, unescaped value
%   op(Sym)         - equals | plus | minus | star | slash | ampersand
%                   | lparen | rparen | lbracket | rbracket | lbrace | rbrace
%                   | comma | semicolon
%
% Source is a list of character atoms (Scryer default with double_quotes=chars).
%
% Convention: every choice in the DCG uses (->) to commit; we never want
% backtracking through token alternatives because it explodes on long input.

:- use_module(library(dcgs)).
:- use_module(library(lists)).
:- use_module(library(pio)).
:- use_module(library(format)).

% --- Public entry points ---

tokenize(Source, Tokens) :-
    phrase(tokens(Tokens), Source).

main :-
    phrase_from_stream(tokens(Ts), user_input),
    print_tokens(Ts).

print_tokens([]).
print_tokens([T|Ts]) :- print_token(T), print_tokens(Ts).

print_token(keyword(K))      :- atom_chars(K, Chars), format("Keyword ~s~n", [Chars]).
print_token(ident(Cs))       :- format("Identifier ~s~n", [Cs]).
print_token(quoted_ident(Cs)) :- format("QuotedIdentifier ~s~n", [Cs]).
print_token(verbatim(Cs))    :- format("VerbatimLiteral ~s~n", [Cs]).
print_token(number(Cs))      :- format("Number ~s~n", [Cs]).
print_token(text(Cs))        :- format("Text ~s~n", [Cs]).
print_token(op(O))           :- format("Op ~w~n", [O]).

% --- Top-level token stream ---

tokens(Ts) --> trivia, tokens_rest(Ts).

tokens_rest([])     --> [].
tokens_rest([T|Ts]) --> token(T), trivia, tokens_rest(Ts).

% --- Trivia: whitespace + comments, repeated ---

trivia -->
    ( ws_one
    -> trivia
    ;   ( "//"
        -> line_comment_body, trivia
        ;   ( "/*"
            -> delim_comment_body, trivia
            ; []
            )
        )
    ).

ws_one --> [' '].
ws_one --> ['\t'].
ws_one --> ['\n'].
ws_one --> ['\r'].

line_comment_body -->
    ( newline_char
    -> []
    ;   ( [_]
        -> line_comment_body
        ; []
        )
    ).

newline_char --> ['\n'].
newline_char --> ['\r'].

delim_comment_body -->
    ( "*/"
    -> []
    ; [_], delim_comment_body
    ).

% --- Tokens: each branch commits, no backtracking ---

token(T) -->
    ( ['"']
    -> { T = text(Cs) }, text_chars(Cs)
    ;   ( ['#']
        -> hash_token(T)
        ;   ( peek_digit
            -> number_token(T)
            ;   ( peek_ident_start
                -> [C], ident_rest(Rest),
                   dotted_continuation([C|Rest], Full),
                   { classify_ident(Full, T) }
                ; operator(T)
                )
            )
        )
    ).

% --- Hash dispatch: #"..." | #!"..." | #keyword ---

hash_token(T) -->
    ( ['"']
    -> { T = quoted_ident(Cs) }, text_chars(Cs)
    ;   ( ['!'], ['"']
        -> { T = verbatim(Cs) }, text_chars(Cs)
        ;   ( peek_ident_start
            -> [C], ident_rest(Rest),
               { classify_hash_keyword([C|Rest], T) }
            ; { fail }
            )
        )
    ).

classify_hash_keyword(Cs, keyword(HK)) :-
    hash_keyword_chars(Cs, HK).

hash_keyword_chars([b,i,n,a,r,y],             '#binary').
hash_keyword_chars([d,a,t,e],                 '#date').
hash_keyword_chars([d,a,t,e,t,i,m,e],         '#datetime').
hash_keyword_chars([d,a,t,e,t,i,m,e,z,o,n,e], '#datetimezone').
hash_keyword_chars([d,u,r,a,t,i,o,n],         '#duration').
hash_keyword_chars([i,n,f,i,n,i,t,y],         '#infinity').
hash_keyword_chars([n,a,n],                   '#nan').
hash_keyword_chars([s,e,c,t,i,o,n,s],         '#sections').
hash_keyword_chars([s,h,a,r,e,d],             '#shared').
hash_keyword_chars([t,a,b,l,e],               '#table').
hash_keyword_chars([t,i,m,e],                 '#time').

peek_digit, [C] --> [C], { digit_char(C) }.
peek_ident_start, [C] --> [C], { ident_start(C) }.
peek_hex_prefix, ['0', X] --> ['0'], [X], { hex_marker(X) }.

hex_marker('x').
hex_marker('X').

number_token(number(Cs)) -->
    ( peek_hex_prefix
    -> ['0'], [X],
       hex_digits1(HexDs),
       { Cs = ['0', X | HexDs] }
    ; [D],
      digits(Ds),
      fractional(F),
      exponent(E),
      { append([D|Ds], F, NF), append(NF, E, Cs) }
    ).

% --- Text literal body (after opening quote) ---
%
% Shared between text literals, quoted identifiers, and verbatim literals.
% Recognises: doubled-quote escape (""), end-of-literal ("), character-escape
% sequence (#(...)), and any other char as literal.

text_chars(Cs) -->
    ( "\"\""
    -> { Cs = ['"' | More] }, text_chars(More)
    ;   ( "\""
        -> { Cs = [] }
        ;   ( ['#'], ['(']
            -> escape_body(Decoded), text_chars(More),
               { append(Decoded, More, Cs) }
            ; [C], { Cs = [C | More] }, text_chars(More)
            )
        )
    ).

% --- Character escape sequences ---
%
% escape-sequence-list := single-escape ( ',' single-escape )* ')'
% Each single-escape decodes to exactly one char.

escape_body([C | Rest]) -->
    single_escape(C),
    ( [',']
    -> escape_body(Rest)
    ; [')'],
      { Rest = [] }
    ).

% Order matters: literal `#` first; control names before hex; hex last.
% Cuts commit the choice — wrong escape contents fail rather than backtrack.

single_escape('#') --> ['#'], !.
single_escape('\r') --> ['c'], ['r'], !.
single_escape('\n') --> ['l'], ['f'], !.
single_escape('\t') --> ['t'], ['a'], ['b'], !.
single_escape(C) -->
    hex_chars_max(8, Hs),
    { length(Hs, N),
      ( N = 4 ; N = 8 ),
      hex_to_code(Hs, CP),
      char_code(C, CP) }.

% Read up to N hex digits, greedily.
hex_chars_max(0, []) --> [], !.
hex_chars_max(N, [H | Hs]) -->
    [H], { hex_digit(H) },
    !,
    { N1 is N - 1 },
    hex_chars_max(N1, Hs).
hex_chars_max(_, []) --> [].

hex_to_code(Hs, CP) :- hex_to_code_acc(Hs, 0, CP).

hex_to_code_acc([], Acc, Acc).
hex_to_code_acc([H | Hs], Acc, CP) :-
    hex_value(H, V),
    Acc1 is Acc * 16 + V,
    hex_to_code_acc(Hs, Acc1, CP).

hex_value(C, V) :-
    ( C @>= '0', C @=< '9'
    -> char_code(C, X), V is X - 0'0
    ;   ( C @>= 'a', C @=< 'f'
        -> char_code(C, X), V is X - 0'a + 10
        ; char_code(C, X), V is X - 0'A + 10
        )
    ).

% --- Number components ---

digits([D | Ds]) -->
    [D], { digit_char(D) },
    !,
    digits(Ds).
digits([]) --> [].

fractional(['.', D | Ds]) -->
    ['.'], [D], { digit_char(D) },
    !,
    digits(Ds).
fractional([]) --> [].

% Exponent: e|E, optional sign, mandatory at least one decimal digit.
exponent([E, S, D | Ds]) -->
    [E], { exp_marker(E) },
    [S], { sign_char(S) },
    !,
    [D], { digit_char(D) },
    digits(Ds).
exponent([E, D | Ds]) -->
    [E], { exp_marker(E) },
    !,
    [D], { digit_char(D) },
    digits(Ds).
exponent([]) --> [].

exp_marker('e').
exp_marker('E').
sign_char('+').
sign_char('-').

hex_digit(C) :- digit_char(C).
hex_digit(C) :- C @>= 'a', C @=< 'f'.
hex_digit(C) :- C @>= 'A', C @=< 'F'.

hex_digits1([D | Ds]) -->
    [D], { hex_digit(D) },
    !,
    hex_digits(Ds).

hex_digits([D | Ds]) -->
    [D], { hex_digit(D) },
    !,
    hex_digits(Ds).
hex_digits([]) --> [].

% --- Identifier shape ---

ident_rest([C | Cs]) -->
    [C], { ident_part(C) },
    !,
    ident_rest(Cs).
ident_rest([]) --> [].

% Dotted continuation: `.X` where X is an identifier-start char extends the token.
dotted_continuation(Acc, Full) -->
    ['.'], [C], { ident_start(C) },
    !,
    ident_rest(Rest),
    { append(Acc, ['.', C | Rest], Acc1) },
    dotted_continuation(Acc1, Full).
dotted_continuation(Acc, Acc) --> [].

% --- Operators / punctuators ---
%
% Longest-match: multi-char alternatives must come before their prefixes so
% top-down clause selection picks the longer form first.

operator(op(ellipsis))      --> ['.', '.', '.'].
operator(op(dot_dot))       --> ['.', '.'].
operator(op(le))            --> ['<', '='].
operator(op(ne))            --> ['<', '>'].
operator(op(ge))            --> ['>', '='].
operator(op(fat_arrow))     --> ['=', '>'].
operator(op(null_coalesce)) --> ['?', '?'].
operator(op(lt))            --> ['<'].
operator(op(gt))            --> ['>'].
operator(op(question))      --> ['?'].
operator(op(equals))        --> ['='].
operator(op(plus))          --> ['+'].
operator(op(minus))         --> ['-'].
operator(op(star))          --> ['*'].
operator(op(slash))         --> ['/'].
operator(op(ampersand))     --> ['&'].
operator(op(lparen))        --> ['('].
operator(op(rparen))        --> [')'].
operator(op(lbracket))      --> ['['].
operator(op(rbracket))      --> [']'].
operator(op(lbrace))        --> ['{'].
operator(op(rbrace))        --> ['}'].
operator(op(comma))         --> [','].
operator(op(semicolon))     --> [';'].
operator(op(at))            --> ['@'].
operator(op(bang))          --> ['!'].

% --- Char classification ---
%
% digit_char/1 is ASCII 0-9 (used for number literals and hex-escape digits;
% the spec defines those as ASCII).
%
% Identifier characters use full Unicode general categories per spec §12.
% Range tables are auto-generated into unicode_tables.pl from the same UCD
% the Rust lexer consults via the unicode-general-category crate.

digit_char(C) :- C @>= '0', C @=< '9'.

ident_start(C) :-
    char_code(C, Code),
    ident_start_code(Code).

ident_start_code(0'_).
ident_start_code(Code) :- letter_range(Lo, Hi), Code >= Lo, Code =< Hi.

ident_part(C) :-
    char_code(C, Code),
    ident_part_code(Code).

ident_part_code(Code) :- ident_start_code(Code).
ident_part_code(Code) :- decimal_digit_range(Lo, Hi), Code >= Lo, Code =< Hi.
ident_part_code(Code) :- connecting_range(Lo, Hi), Code >= Lo, Code =< Hi.
ident_part_code(Code) :- combining_range(Lo, Hi), Code >= Lo, Code =< Hi.
ident_part_code(Code) :- formatting_range(Lo, Hi), Code >= Lo, Code =< Hi.

% --- Keyword classification ---

classify_ident(Cs, T) :-
    ( keyword_chars(Cs, K)
    -> T = keyword(K)
    ;  T = ident(Cs)
    ).

keyword_chars([l,e,t],         let).
keyword_chars([i,n],           in).
keyword_chars([i,f],           if).
keyword_chars([t,h,e,n],       then).
keyword_chars([e,l,s,e],       else).
keyword_chars([t,r,u,e],       true).
keyword_chars([f,a,l,s,e],     false).
keyword_chars([n,u,l,l],       null).
keyword_chars([a,n,d],         and).
keyword_chars([o,r],           or).
keyword_chars([n,o,t],         not).
keyword_chars([e,a,c,h],       each).
keyword_chars([t,r,y],         try).
keyword_chars([o,t,h,e,r,w,i,s,e], otherwise).
keyword_chars([e,r,r,o,r],     error).
keyword_chars([a,s],           as).
keyword_chars([i,s],           is).
keyword_chars([t,y,p,e],       type).
keyword_chars([m,e,t,a],       meta).
keyword_chars([s,e,c,t,i,o,n], section).
keyword_chars([s,h,a,r,e,d],   shared).

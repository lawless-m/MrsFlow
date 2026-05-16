% syntactic.pl — DCG parser for the M language (slice 1).
%
% Reference: Microsoft Power Query M language specification.
% Mirror of mrsflow-core/src/parser/. Scope is intentionally identical:
% literals, identifier refs, parens, unary +/-/not, full binary precedence
% chain, if/then/else, let/in. AST term shape:
%
%   num(Chars) | text(Chars) | bool(true|false) | null
%   ref(Chars)
%   unop(Name, Expr)   where Name = pos|neg|not
%   binop(Name, L, R)  where Name = mul|div|add|sub|cat|lt|le|gt|ge|eq|ne|and|or
%   if(Cond, Then, Else)
%   let(Bindings, Body)  where Bindings = [binding(NameChars, Value), ...]
%
% Token stream comes from lexical.pl. The parser DCG operates over that list.

:- use_module(library(dcgs)).
:- use_module(library(lists)).
:- use_module(library(format)).

% print_sexpr clauses are interleaved with their helpers for readability;
% scryer's loader silently breaks goal initialization without this directive.
:- discontiguous(print_sexpr/1).

parse(Tokens, Ast) :-
    phrase(document(Ast), Tokens).

% Top-level: a section declaration (corpus has `section Section1;` files)
% or a bare expression (everything else).
document(Ast) -->
    [keyword(section)],
    !,
    section_body(Ast).
document(Ast) --> expression(Ast).

section_body(section(Name, Members)) -->
    section_name(Name),
    [op(semicolon)],
    section_members(Members).

section_name(N) --> [ident(N)], !.
section_name(N) --> [quoted_ident(N)].

section_members([M | Ms]) -->
    section_member(M),
    !,
    section_members(Ms).
section_members([]) --> [].

section_member(member(Shared, Name, Value)) -->
    shared_marker(Shared),
    section_name(Name),
    [op(equals)],
    expression(Value),
    [op(semicolon)].

shared_marker(shared) --> [keyword(shared)], !.
shared_marker(private) --> [].

% --- Top-level expression dispatch ---

expression(Ast) -->
    ( [keyword(if)]
    -> if_expr(Ast)
    ;   ( [keyword(let)]
        -> let_expr(Ast)
        ;   ( [keyword(each)]
            -> each_expr(Ast)
            ;   ( [keyword(try)]
                -> try_expr(Ast)
                ;   ( [keyword(error)]
                    -> error_expr(Ast)
                    ; logical_or(Ast)
                    )
                )
            )
        )
    ).

each_expr(each(E)) --> expression(E).

try_expr(try(Body, Otherwise)) -->
    expression(Body),
    ( [keyword(otherwise)]
    -> expression(Fb), { Otherwise = some(Fb) }
    ; { Otherwise = none }
    ).

error_expr(error(Msg)) --> expression(Msg).

if_expr(if(Cond, Then, Else)) -->
    expression(Cond),
    [keyword(then)],
    expression(Then),
    [keyword(else)],
    expression(Else).

let_expr(let(Bindings, Body)) -->
    bindings(Bindings),
    [keyword(in)],
    expression(Body).

bindings([B | Rest]) -->
    binding(B),
    ( [op(comma)]
    -> bindings(Rest)
    ; { Rest = [] }
    ).

binding(binding(NameChars, Value)) -->
    binding_name(NameChars),
    [op(equals)],
    expression(Value).

binding_name(Cs) --> [ident(Cs)], !.
binding_name(Cs) --> [quoted_ident(Cs)].

% --- Binary precedence chain (low → high precedence, left-associative) ---

logical_or(A) --> logical_and(L), logical_or_rest(L, A).

logical_or_rest(L, A) -->
    [keyword(or)], !,
    logical_and(R),
    logical_or_rest(binop(or, L, R), A).
logical_or_rest(L, L) --> [].

logical_and(A) --> is_expr(L), logical_and_rest(L, A).
logical_and_rest(L, A) -->
    [keyword(and)], !,
    is_expr(R),
    logical_and_rest(binop(and, L, R), A).
logical_and_rest(L, L) --> [].

is_expr(A) --> as_expr(L), is_rest(L, A).
is_rest(L, A) -->
    [keyword(is)], !,
    primary_type(R),
    is_rest(binop(is, L, R), A).
is_rest(L, L) --> [].

as_expr(A) --> equality(L), as_rest(L, A).
as_rest(L, A) -->
    [keyword(as)], !,
    primary_type(R),
    as_rest(binop(as, L, R), A).
as_rest(L, L) --> [].

% Primary type — full primary-type per spec, lenient (parser doesn't enforce
% the spec's restriction of as/is RHS to nullable-primitive). Handles:
% nullable T, table T, function (...) as T, list-type {T}, record-type [...],
% paren-escape (...), or a primitive-type identifier (any primary).

primary_type(unop(nullable, E)) -->
    [ident([n,u,l,l,a,b,l,e])],
    !,
    primary_type(E).
% `table T` (composite table-type with row-type) vs bare `table` (primitive).
% The cut moves AFTER primary_type so Prolog can backtrack to the fallback
% `primary_type(E) --> primary(E).` clause when no inner type follows
% (e.g. `Value.Is(x, type table)`). Mirrors how `function`/`nullable` are
% handled — both require a body, but `table` alone is a valid primitive.
primary_type(table_type(E)) -->
    [ident([t,a,b,l,e])],
    primary_type(E),
    !.
primary_type(E) -->
    [ident([f,u,n,c,t,i,o,n])],
    !,
    function_type_body(E).
primary_type(list_type(T)) -->
    [op(lbrace)],
    !,
    primary_type(T),
    [op(rbrace)].
primary_type(E) -->
    [op(lbracket)],
    !,
    record_type_body(E).
primary_type(E) -->
    [op(lparen)],
    !,
    expression(E),
    [op(rparen)].
primary_type(E) --> primary(E).

function_type_body(function_type(Params, Return)) -->
    [op(lparen)],
    params_list(Params),
    [op(rparen)],
    [keyword(as)],
    primary_type(Return).

record_type_body(record_type(Fields, IsOpen)) -->
    ( [op(rbracket)]
    -> { Fields = [], IsOpen = closed }
    ; record_type_fields(Fields, IsOpen),
      [op(rbracket)]
    ).

record_type_fields([], open) -->
    [op(ellipsis)],
    !.
record_type_fields([F | Rest], IsOpen) -->
    record_type_field(F),
    ( [op(comma)]
    -> record_type_fields(Rest, IsOpen)
    ; { Rest = [], IsOpen = closed }
    ).

record_type_field(field(Name, Opt, Type)) -->
    optional_param_marker(Opt),
    field_name(Name),
    record_field_type_opt(Type).

record_field_type_opt(some(T)) --> [op(equals)], !, primary_type(T).
record_field_type_opt(none) --> [].

equality(A) --> relational(L), equality_rest(L, A).
equality_rest(L, A) -->
    eq_op(Op), !,
    relational(R),
    equality_rest(binop(Op, L, R), A).
equality_rest(L, L) --> [].

eq_op(eq) --> [op(equals)].
eq_op(ne) --> [op(ne)].

relational(A) --> additive(L), relational_rest(L, A).
relational_rest(L, A) -->
    rel_op(Op), !,
    additive(R),
    relational_rest(binop(Op, L, R), A).
relational_rest(L, L) --> [].

rel_op(lt) --> [op(lt)].
rel_op(le) --> [op(le)].
rel_op(gt) --> [op(gt)].
rel_op(ge) --> [op(ge)].

additive(A) --> multiplicative(L), additive_rest(L, A).
additive_rest(L, A) -->
    add_op(Op), !,
    multiplicative(R),
    additive_rest(binop(Op, L, R), A).
additive_rest(L, L) --> [].

add_op(add) --> [op(plus)].
add_op(sub) --> [op(minus)].
add_op(cat) --> [op(ampersand)].

multiplicative(A) --> metadata(L), multiplicative_rest(L, A).
multiplicative_rest(L, A) -->
    mul_op(Op), !,
    metadata(R),
    multiplicative_rest(binop(Op, L, R), A).
multiplicative_rest(L, L) --> [].

mul_op(mul) --> [op(star)].
mul_op(div) --> [op(slash)].

metadata(A) --> unary(L), metadata_rest(L, A).
metadata_rest(L, A) -->
    [keyword(meta)], !,
    unary(R),
    metadata_rest(binop(meta, L, R), A).
metadata_rest(L, L) --> [].

unary(unop(pos, E)) --> [op(plus)], !, unary(E).
unary(unop(neg, E)) --> [op(minus)], !, unary(E).
unary(unop(not, E)) --> [keyword(not)], !, unary(E).
% `type X` switches into type context — X is a primary-type, not a regular
% unary (per spec: type-expression: ... | type primary-type).
unary(unop(type, E)) --> [keyword(type)], !, primary_type(E).
unary(E) --> postfix(E).

% --- Postfix chain: invocation, field access, item access ---

postfix(E) --> primary(P), postfix_chain(P, E).

postfix_chain(P, E) -->
    ( [op(lparen)]
    -> args(Args), [op(rparen)],
       postfix_chain(invoke(P, Args), E)
    ;   ( [op(lbracket)]
        -> field_name(F), [op(rbracket)],
           optional_marker(Opt),
           postfix_chain(field_access(P, F, Opt), E)
        ;   ( [op(lbrace)]
            -> expression(I), [op(rbrace)],
               optional_marker(Opt),
               postfix_chain(item_access(P, I, Opt), E)
            ; { E = P }
            )
        )
    ).

args([A | As]) -->
    expression(A),
    ( [op(comma)]
    -> args(As)
    ; { As = [] }
    ).
args([]) --> [].

optional_marker(opt) --> [op(question)], !.
optional_marker(req) --> [].

% Field name: either a quoted identifier (single token) or a generalized
% identifier (one or more adjacent regular identifier tokens, joined with
% single spaces). Used in record fields and field-access selectors.
field_name(N) --> [quoted_ident(N)], !.
field_name(N) -->
    gen_id_token(First),
    field_name_tail(More),
    { join_chars_with_space([First | More], N) }.

field_name_tail([Cs | Rest]) -->
    gen_id_token(Cs),
    !,
    field_name_tail(Rest).
field_name_tail([]) --> [].

% A token usable in a generalized identifier — idents, numbers, and
% most keywords (PQ corpus has [Bastian L H2 2023], [Company or Comment]
% etc.). The block list `let|in|if|then|else` is intentionally NOT
% joinable: they're the entry tokens of major expression forms and
% would create unbounded ambiguity in record-literal field names which
% reuse this rule via `[name = expr]`. Mirrors the Rust
% `generalized_identifier_text` helper.
gen_id_token(Cs) --> [ident(Cs)], !.
gen_id_token(Cs) --> [number(Cs)], !.
gen_id_token(Cs) -->
    [keyword(K)],
    { \+ member(K, [let, in, if, then, else]),
      atom_chars(K, Cs) }.

% Join a list of char-lists with single ' ' separator.
join_chars_with_space([Cs], Cs) :- !.
join_chars_with_space([Cs | Rest], Joined) :-
    join_chars_with_space(Rest, RestJoined),
    append(Cs, [' ' | RestJoined], Joined).

% --- Primary expressions ---

primary(num(Cs))    --> [number(Cs)].
primary(text(Cs))   --> [text(Cs)].
primary(bool(true)) --> [keyword(true)].
primary(bool(false)) --> [keyword(false)].
primary(null)       --> [keyword(null)].
primary(ref(Cs))    --> [ident(Cs)].
primary(ref(Cs))    --> [quoted_ident(Cs)].
% #-keywords (#date, #table, #nan, etc.) act as identifier-like refs at primary
% position. Constructors like #date(2024,1,1) get wrapped by the postfix chain
% into invoke(ref("#date"), [...]).
primary(ref(NameChars)) -->
    [keyword(K)],
    { is_hash_keyword(K) },
    { atom_chars(K, NameChars) }.
primary(E)          --> [op(lparen)], parens_or_function(E).
primary(E)          --> [op(lbracket)], bracketed_body(E).
primary(E)          --> [op(lbrace)], list_body(E).
% `@<identifier>` — scoping operator. AST shape is just ref(Cs); the
% recursive lookup semantics are an evaluator concern, not a parser one.
primary(ref(Cs))    --> [op(at)], [ident(Cs)].
primary(ref(Cs))    --> [op(at)], [quoted_ident(Cs)].

is_hash_keyword('#binary').
is_hash_keyword('#date').
is_hash_keyword('#datetime').
is_hash_keyword('#datetimezone').
is_hash_keyword('#duration').
is_hash_keyword('#infinity').
is_hash_keyword('#nan').
is_hash_keyword('#sections').
is_hash_keyword('#shared').
is_hash_keyword('#table').
is_hash_keyword('#time').

% Parens vs function literal — try function first, fall back to parens.
% (Cond -> Then ; Else) is pure: if Cond fails, no input is consumed.
% Function tail recognises `=>` directly OR `as TYPE =>` for return type.
parens_or_function(E) -->
    ( params_list(Ps), [op(rparen)], return_type_opt(Ret), [op(fat_arrow)]
    -> expression(Body), { E = fn(Ps, Ret, Body) }
    ; expression(E), [op(rparen)]
    ).

params_list([P | Ps]) -->
    param(P),
    ( [op(comma)] -> params_list(Ps) ; { Ps = [] } ).
params_list([]) --> [].

param(param(Name, Opt, Type)) -->
    optional_param_marker(Opt),
    binding_name(Name),
    type_assertion_opt(Type).

optional_param_marker(opt) --> [ident([o,p,t,i,o,n,a,l])], !.
optional_param_marker(req) --> [].

type_assertion_opt(some(T)) --> [keyword(as)], !, primary_type(T).
type_assertion_opt(none) --> [].

return_type_opt(some(T)) --> [keyword(as)], !, primary_type(T).
return_type_opt(none) --> [].

% Bracketed primary (after consuming `[`):
%   `]`      → empty record
%   `name ]` → implicit field access on `_` (with optional `?`)
%   `name =` → record literal
bracketed_body(E) -->
    ( [op(rbracket)]
    -> { E = record([]) }
    ;   ( field_name(F), [op(rbracket)]
        -> optional_marker(Opt),
           { E = field_access(ref(['_']), F, Opt) }
        ; record_fields(Fields), [op(rbracket)],
          { E = record(Fields) }
        )
    ).

record_fields([F | Fs]) -->
    record_field(F),
    ( [op(comma)] -> record_fields(Fs) ; { Fs = [] } ).

record_field(pair(Name, Value)) -->
    field_name(Name),
    [op(equals)],
    expression(Value).

% List literal body (after consuming `{`).
list_body(list(Items)) -->
    ( [op(rbrace)]
    -> { Items = [] }
    ; list_items(Items), [op(rbrace)]
    ).

list_items([I | Is]) -->
    list_item(I),
    ( [op(comma)] -> list_items(Is) ; { Is = [] } ).

list_item(I) -->
    expression(E),
    ( [op(dot_dot)]
    -> expression(End), { I = range(E, End) }
    ; { I = single(E) }
    ).

% --- Canonical S-expression printer ---
%
% Format matches mrsflow-core/src/parser/ast.rs `Expr::to_sexpr` exactly.

print_ast(A) :- print_sexpr(A), nl.

print_sexpr(num(Cs))     :- format("(num ", []), print_quoted(Cs), format(")", []).
print_sexpr(text(Cs))    :- format("(text ", []), print_quoted(Cs), format(")", []).
print_sexpr(bool(true))  :- format("(bool true)", []).
print_sexpr(bool(false)) :- format("(bool false)", []).
print_sexpr(null)        :- format("(null)", []).
print_sexpr(ref(Cs))     :- format("(ref ", []), print_quoted(Cs), format(")", []).
print_sexpr(unop(Op, E)) :-
    format("(~a ", [Op]), print_sexpr(E), format(")", []).
print_sexpr(binop(Op, L, R)) :-
    format("(~a ", [Op]),
    print_sexpr(L), format(" ", []),
    print_sexpr(R),
    format(")", []).
print_sexpr(if(C, T, E)) :-
    format("(if ", []),
    print_sexpr(C), format(" ", []),
    print_sexpr(T), format(" ", []),
    print_sexpr(E),
    format(")", []).
print_sexpr(let(Bindings, Body)) :-
    format("(let (", []),
    print_bindings(Bindings),
    format(") ", []),
    print_sexpr(Body),
    format(")", []).

print_sexpr(record(Fields)) :-
    format("(record (", []),
    print_record_fields(Fields),
    format("))", []).

print_sexpr(list(Items)) :-
    format("(list (", []),
    print_list_items(Items),
    format("))", []).

print_sexpr(fn(Params, Ret, Body)) :-
    format("(fn (", []),
    print_param_specs(Params),
    format(") ", []),
    print_type_or_none(Ret),
    format(" ", []),
    print_sexpr(Body),
    format(")", []).

print_param_specs([]).
print_param_specs([P]) :- print_param_spec(P).
print_param_specs([P1, P2 | Rest]) :-
    print_param_spec(P1),
    format(" ", []),
    print_param_specs([P2 | Rest]).

print_param_spec(param(NameChars, Opt, Type)) :-
    format("(", []),
    print_quoted(NameChars),
    format(" ~a ", [Opt]),
    print_type_or_none(Type),
    format(")", []).

print_type_or_none(none) :- format("none", []).
print_type_or_none(some(T)) :- print_sexpr(T).

print_sexpr(each(Body)) :-
    format("(each ", []),
    print_sexpr(Body),
    format(")", []).

print_sexpr(invoke(Target, Args)) :-
    format("(invoke ", []),
    print_sexpr(Target),
    format(" (", []),
    print_args(Args),
    format("))", []).

print_sexpr(field_access(Target, Name, Opt)) :-
    field_op(Opt, Op),
    format("(~a ", [Op]),
    print_sexpr(Target),
    format(" ", []),
    print_quoted(Name),
    format(")", []).

print_sexpr(item_access(Target, Index, Opt)) :-
    item_op(Opt, Op),
    format("(~a ", [Op]),
    print_sexpr(Target),
    format(" ", []),
    print_sexpr(Index),
    format(")", []).
print_sexpr(try(Body, none)) :-
    format("(try ", []),
    print_sexpr(Body),
    format(")", []).
print_sexpr(try(Body, some(Otherwise))) :-
    format("(try ", []),
    print_sexpr(Body),
    format(" ", []),
    print_sexpr(Otherwise),
    format(")", []).
print_sexpr(error(Msg)) :-
    format("(error ", []),
    print_sexpr(Msg),
    format(")", []).
print_sexpr(section(Name, Members)) :-
    format("(section ", []),
    print_quoted(Name),
    format(" (", []),
    print_section_members(Members),
    format("))", []).

print_section_members([]).
print_section_members([M]) :- print_section_member(M).
print_section_members([M1, M2 | Rest]) :-
    print_section_member(M1),
    format(" ", []),
    print_section_members([M2 | Rest]).

print_section_member(member(Shared, Name, Value)) :-
    format("(member ~a ", [Shared]),
    print_quoted(Name),
    format(" ", []),
    print_sexpr(Value),
    format(")", []).
print_sexpr(list_type(T)) :-
    format("(list-type ", []),
    print_sexpr(T),
    format(")", []).
print_sexpr(record_type(Fields, IsOpen)) :-
    format("(record-type (", []),
    print_record_type_fields(Fields),
    format(") ~a)", [IsOpen]).
print_sexpr(table_type(T)) :-
    format("(table-type ", []),
    print_sexpr(T),
    format(")", []).
print_sexpr(function_type(Params, Return)) :-
    format("(function-type (", []),
    print_param_specs(Params),
    format(") ", []),
    print_sexpr(Return),
    format(")", []).

print_record_type_fields([]).
print_record_type_fields([F]) :- print_record_type_field(F).
print_record_type_fields([F1, F2 | Rest]) :-
    print_record_type_field(F1),
    format(" ", []),
    print_record_type_fields([F2 | Rest]).

print_record_type_field(field(NameChars, Opt, Type)) :-
    format("(", []),
    print_quoted(NameChars),
    format(" ~a ", [Opt]),
    print_type_or_none(Type),
    format(")", []).

field_op(req, field).
field_op(opt, 'field?').
item_op(req, item).
item_op(opt, 'item?').

print_bindings([]).
print_bindings([B]) :- print_binding(B).
print_bindings([B1, B2 | Rest]) :-
    print_binding(B1),
    format(" ", []),
    print_bindings([B2 | Rest]).

print_binding(binding(NameChars, Value)) :-
    format("(", []),
    print_quoted(NameChars),
    format(" ", []),
    print_sexpr(Value),
    format(")", []).

print_record_fields([]).
print_record_fields([F]) :- print_record_field(F).
print_record_fields([F1, F2 | Rest]) :-
    print_record_field(F1),
    format(" ", []),
    print_record_fields([F2 | Rest]).

print_record_field(pair(NameChars, Value)) :-
    format("(", []),
    print_quoted(NameChars),
    format(" ", []),
    print_sexpr(Value),
    format(")", []).

print_list_items([]).
print_list_items([I]) :- print_list_item(I).
print_list_items([I1, I2 | Rest]) :-
    print_list_item(I1),
    format(" ", []),
    print_list_items([I2 | Rest]).

print_list_item(single(E)) :-
    format("(item ", []),
    print_sexpr(E),
    format(")", []).
print_list_item(range(S, E)) :-
    format("(range ", []),
    print_sexpr(S),
    format(" ", []),
    print_sexpr(E),
    format(")", []).

print_params([]).
print_params([P]) :- print_quoted(P).
print_params([P1, P2 | Rest]) :-
    print_quoted(P1),
    format(" ", []),
    print_params([P2 | Rest]).

print_args([]).
print_args([A]) :- print_sexpr(A).
print_args([A1, A2 | Rest]) :-
    print_sexpr(A1),
    format(" ", []),
    print_args([A2 | Rest]).

% Print a chars list quoted, matching Rust's write_quoted: escape " \ \n \r \t.
print_quoted(Cs) :- format("\"", []), print_quoted_chars(Cs), format("\"", []).

print_quoted_chars([]).
print_quoted_chars([C | Cs]) :- print_escaped_char(C), print_quoted_chars(Cs).

print_escaped_char('"')  :- format("\\\"", []).
print_escaped_char('\\') :- format("\\\\", []).
print_escaped_char('\n') :- format("\\n", []).
print_escaped_char('\r') :- format("\\r", []).
print_escaped_char('\t') :- format("\\t", []).
print_escaped_char(C) :-
    \+ memberchk(C, ['"', '\\', '\n', '\r', '\t']),
    put_char(C).

% evaluator.pl — Prolog evaluator companion for the M language.
%
% Mirror of mrsflow-core/src/eval/. Same value shapes (terms instead of Rust
% enums), same evaluation semantics — independent reading of the spec used as
% a differential oracle on every change. See mrsflow/07-evaluator-design.md.
%
% This file is currently a scaffold — the actual eval/3 rules land in slice-1
% (task #35). The structural pieces are here so the diff_eval.sh harness can
% wire up against print_value/1 immediately.
%
% Value term shapes (mirrors Rust's `Value` enum):
%
%   null
%   bool(true) | bool(false)
%   num(F)              — F is always a float for parity with Rust f64
%   text(Cs)            — Cs is chars list
%   date(Cs)            — placeholder until date slice
%   datetime(Cs)        — placeholder
%   duration(Cs)        — placeholder
%   binary(Bytes)       — Bytes list of integers 0..255
%   list(Items)         — Items list of values
%   record(Pairs)       — Pairs = [pair(NameChars, Value), ...] in insertion order
%   table(...)          — placeholder until eval-7
%   closure(Params, Body, Env)
%   type_value(Repr)    — placeholder until eval-5
%   thunk(Expr, Env)    — forced on access via force/2

:- use_module(library(dcgs)).
:- use_module(library(lists)).
:- use_module(library(format)).

% Both eval/3 and print_value/1 will have clauses interleaved with helpers
% as slices land — same scryer trap as syntactic.pl, head it off now.
:- discontiguous(eval/3).
:- discontiguous(print_value/1).

% --- eval/3: Ast × Env → Value ---
%
% Mirrors the Rust eval-1 surface in mrsflow-core/src/eval/mod.rs.
% Env representation: list of frame(Bindings) where Bindings is a list of
% Name-Value pairs. Innermost frame at the head; lookup walks rightward.
%
% Thunks for let bindings reference the *new* env (the one being built) via
% Prolog term sharing — see extend_lazy_bindings/3 below.

% Literals
eval(num(Cs),  _,    num(F))   :- chars_to_number(Cs, F).
eval(text(Cs), _,    text(Cs)).
eval(bool(B),  _,    bool(B)).
eval(null,     _,    null).

% Identifier reference: env lookup + force.
eval(ref(Name), Env, Value) :-
    eval_lookup(Name, Env, Raw),
    force(Raw, Value).

% Unary
eval(unop(pos, E), Env, num(N)) :-
    eval(E, Env, num(N)).
eval(unop(neg, E), Env, num(NN)) :-
    eval(E, Env, num(N)),
    NN is -N.
eval(unop(not, E), Env, bool(NB)) :-
    eval(E, Env, bool(B)),
    not_bool(B, NB).

% Logical and/or — short-circuit via if-then-else so the right operand
% isn't evaluated when the left determines the result.
eval(binop(and, L, R), Env, bool(Result)) :-
    eval(L, Env, bool(LV)),
    ( LV == false
    -> Result = false
    ; eval(R, Env, bool(Result))
    ).
eval(binop(or, L, R), Env, bool(Result)) :-
    eval(L, Env, bool(LV)),
    ( LV == true
    -> Result = true
    ; eval(R, Env, bool(Result))
    ).

% Arithmetic — operands must be numbers; pattern-match on num/1 ensures
% type-mismatches simply fail (matches Rust's TypeMismatch via empty stdout).
eval(binop(mul, L, R), Env, num(V)) :-
    eval(L, Env, num(A)), eval(R, Env, num(B)),
    V is A * B.
eval(binop(div, L, R), Env, num(V)) :-
    eval(L, Env, num(A)), eval(R, Env, num(B)),
    B =\= 0.0,
    V is A / B.
eval(binop(add, L, R), Env, num(V)) :-
    eval(L, Env, num(A)), eval(R, Env, num(B)),
    V is A + B.
eval(binop(sub, L, R), Env, num(V)) :-
    eval(L, Env, num(A)), eval(R, Env, num(B)),
    V is A - B.

% Concat — text only for slice 1; list concat lands when eval-3 brings lists.
eval(binop(cat, L, R), Env, text(Combined)) :-
    eval(L, Env, text(LCs)),
    eval(R, Env, text(RCs)),
    append(LCs, RCs, Combined).

% Comparison — numbers and texts only for slice 1.
eval(binop(lt, L, R), Env, bool(Result)) :-
    eval(L, Env, LV), eval(R, Env, RV),
    compare_values(<, LV, RV, Result).
eval(binop(le, L, R), Env, bool(Result)) :-
    eval(L, Env, LV), eval(R, Env, RV),
    compare_values(=<, LV, RV, Result).
eval(binop(gt, L, R), Env, bool(Result)) :-
    eval(L, Env, LV), eval(R, Env, RV),
    compare_values(>, LV, RV, Result).
eval(binop(ge, L, R), Env, bool(Result)) :-
    eval(L, Env, LV), eval(R, Env, RV),
    compare_values(>=, LV, RV, Result).

% Equality — structural; mismatched kinds compare false.
eval(binop(eq, L, R), Env, bool(Result)) :-
    eval(L, Env, LV), eval(R, Env, RV),
    values_equal(LV, RV, Result).
eval(binop(ne, L, R), Env, bool(Result)) :-
    eval(L, Env, LV), eval(R, Env, RV),
    values_equal(LV, RV, Eq),
    not_bool(Eq, Result).

% if/then/else
eval(if(Cond, Then, Else), Env, Result) :-
    eval(Cond, Env, bool(C)),
    ( C == true
    -> eval(Then, Env, Result)
    ; eval(Else, Env, Result)
    ).

% let/in with lazy mutual-recursive bindings
eval(let(Bindings, Body), Env, Value) :-
    extend_lazy_bindings(Bindings, Env, NewEnv),
    eval(Body, NewEnv, Value).

% Function literal — capture current env in a closure. Return type and
% per-param type annotations are parsed but ignored at runtime (eval-5
% enforces them).
eval(fn(Params, _Ret, Body), Env, closure(Params, Body, Env)).

% `each E` is sugar for `(_) => E` — build the closure directly with a
% single required param named "_".
eval(each(Body), Env, closure([param(['_'], req, none)], Body, Env)).

% Function invocation — eager arg evaluation (force each), arity check,
% bind to params (missing optional → null), eval body in extended env.
eval(invoke(Target, Args), Env, Value) :-
    eval(Target, Env, TargetV0),
    force(TargetV0, closure(Params, Body, CEnv)),
    bind_args(Params, Args, Env, Bindings),
    eval(Body, [frame(Bindings) | CEnv], Value).

% --- eval-3: list / record / field+item access ---

% List literal — items are eager (only records have per-field laziness).
% Range items expand to inclusive integer sequences.
eval(list(Items), Env, list(Values)) :-
    eval_list_items(Items, Env, Values).

% Record literal — each field becomes a thunk in a shared env so sibling
% fields can reference one another (same self-referential term-sharing
% pattern as extend_lazy_bindings for let).
eval(record(Fields), Env, record(Pairs)) :-
    NewEnv = [frame(Frame) | Env],
    build_record_frame(Fields, NewEnv, Pairs, Frame).

% Field access — r[name] (required) or r[name]? (optional).
eval(field_access(Target, Name, Opt), Env, Value) :-
    eval(Target, Env, T0),
    force(T0, record(Pairs)),
    field_lookup(Pairs, Name, Opt, Value).

% Item access — list-only for slice 3. Index must be a non-negative integer.
eval(item_access(Target, Index, Opt), Env, Value) :-
    eval(Target, Env, T0),
    force(T0, list(Items)),
    eval(Index, Env, I0),
    force(I0, num(IF)),
    IInt is truncate(IF),
    IF =:= IInt,
    IInt >= 0,
    list_index_opt(Items, IInt, Opt, Value).

% --- eval-4: try / otherwise / error ---
%
% Errors flow as thrown `mrsflow_error(ErrorRec)` terms. `try` catches
% them (and predicate failures) and surfaces them as records. `error E`
% evaluates E, builds the error record, throws.

eval(error(E), Env, _) :-
    eval(E, Env, V0),
    force(V0, V),
    build_error_record(V, ErrRec),
    throw(mrsflow_error(ErrRec)).

% try without otherwise — always succeeds, returning the success record on
% normal eval and the failure record on either a thrown mrsflow_error or
% a predicate failure inside the body.
eval(try(Body, none), Env, Result) :-
    catch(
        ( eval(Body, Env, V0), force(V0, V), try_success_record(V, Result) ),
        mrsflow_error(ErrRec),
        try_failure_record(ErrRec, Result)
    ),
    !.
eval(try(Body, none), _Env, Result) :-
    default_error_record(DefRec),
    try_failure_record(DefRec, Result),
    % Suppress singleton-warning by binding Body in description; not actually used.
    Body = Body.

% --- eval-5: type system ---

% type X — construct a type-value. RHS is in type context, not value context.
eval(unop(type, Inner), _Env, type_value(T)) :-
    eval_as_type(Inner, T).

% as X — runtime conformance check. Fails if non-conforming, which matches
% the Rust side's MError::Other and produces empty stdout for the differential.
eval(binop(as, L, R), Env, Value) :-
    eval(L, Env, LV0),
    force(LV0, LV),
    eval_as_type(R, T),
    type_conforms(LV, T),
    Value = LV.

% is X — runtime type test. Always succeeds with a boolean.
eval(binop(is, L, R), Env, bool(B)) :-
    eval(L, Env, LV0),
    force(LV0, LV),
    eval_as_type(R, T),
    ( type_conforms(LV, T) -> B = true ; B = false ).

% try with otherwise — succeeds with body's value on success, with
% fallback's value on either thrown error or predicate failure.
eval(try(Body, some(Fallback)), Env, Value) :-
    catch(
        ( eval(Body, Env, V0), force(V0, Value) ),
        mrsflow_error(_),
        ( eval(Fallback, Env, F0), force(F0, Value) )
    ),
    !.
eval(try(Body, some(Fallback)), Env, Value) :-
    % Predicate-failure path — first clause's catch only fires on thrown
    % mrsflow_error, not on bare failure. This handles missing-name etc.
    eval(Fallback, Env, F0),
    force(F0, Value),
    Body = Body.

% --- force/2: thunk → forced value ---
%
% Slice-1 recomputes; memoisation TBD when a real workload shows it matters.

force(thunk(Expr, Env), Value) :-
    !,
    eval(Expr, Env, V0),
    force(V0, Value).
force(V, V).

% --- env operations ---

eval_lookup(Name, [frame(Bindings) | _], Value) :-
    member(Name-Value, Bindings),
    !.
eval_lookup(Name, [_ | Rest], Value) :-
    eval_lookup(Name, Rest, Value).

% Build a new env where each binding's thunk references the new env itself.
% Prolog term sharing: NewEnv is constructed with an unbound Frame slot;
% thunks capture NewEnv (with Frame still unbound); then Frame is unified
% with the bindings. Mutual recursion works because all thunks share the
% same env term — exactly the Rust::new_cyclic pattern, in Prolog form.
extend_lazy_bindings(Bindings, ParentEnv, NewEnv) :-
    NewEnv = [frame(Frame) | ParentEnv],
    build_lazy_frame(Bindings, NewEnv, Frame).

build_lazy_frame([], _, []).
build_lazy_frame([binding(Name, Expr) | Rest], Env,
                 [Name-thunk(Expr, Env) | RestFrame]) :-
    build_lazy_frame(Rest, Env, RestFrame).

% Bind invocation arguments to a closure's params. Args are evaluated in the
% *caller's* env (CallerEnv), then forced eagerly — M is not call-by-name.
% Optional params with no matching arg default to null. If there are fewer
% args than required params (or more args than total params), bind_args fails
% — which matches Rust's "arity mismatch" error: both sides produce empty
% stdout on the differential, agreeing structurally.
bind_args(Params, Args, CallerEnv, Bindings) :-
    bind_args_(Params, Args, CallerEnv, Bindings).

bind_args_([], [], _, []).
% Required param + supplied arg: eval and force, then bind.
bind_args_([param(Name, req, _) | PRest], [Arg | ARest], CallerEnv,
           [Name-Value | BRest]) :-
    eval(Arg, CallerEnv, V0),
    force(V0, Value),
    bind_args_(PRest, ARest, CallerEnv, BRest).
% Optional param + supplied arg: same as required.
bind_args_([param(Name, opt, _) | PRest], [Arg | ARest], CallerEnv,
           [Name-Value | BRest]) :-
    eval(Arg, CallerEnv, V0),
    force(V0, Value),
    bind_args_(PRest, ARest, CallerEnv, BRest).
% Optional param + no more args: bind to null and recurse with [] args.
bind_args_([param(Name, opt, _) | PRest], [], CallerEnv,
           [Name-null | BRest]) :-
    bind_args_(PRest, [], CallerEnv, BRest).
% Required param + no more args fails (arity mismatch).
% Extra args (more args than params) also fails — the base case ([], [], ...)
% doesn't match and no other clause does.

% --- eval-3 helpers ---

% List-item evaluation. Singles are eagerly evaluated and forced; ranges
% expand to integer sequences with strict integer bounds and s <= e.
eval_list_items([], _, []).
eval_list_items([single(E) | Rest], Env, [V | VRest]) :-
    eval(E, Env, V0),
    force(V0, V),
    eval_list_items(Rest, Env, VRest).
eval_list_items([range(SE, EE) | Rest], Env, Values) :-
    eval(SE, Env, SV0), force(SV0, num(SF)),
    eval(EE, Env, EV0), force(EV0, num(EF)),
    SI is truncate(SF), EI is truncate(EF),
    SF =:= SI, EF =:= EI,
    SI =< EI,
    range_values(SI, EI, RangeValues),
    eval_list_items(Rest, Env, RestValues),
    append(RangeValues, RestValues, Values).

range_values(I, End, []) :- I > End, !.
range_values(I, End, [num(F) | Rest]) :-
    F is float(I),
    Next is I + 1,
    range_values(Next, End, Rest).

% Record-frame construction. Builds both the pair list (for the record
% value) and the env frame entries (for sibling resolution). Each value
% slot is a thunk capturing the new env via term sharing.
build_record_frame([], _, [], []).
build_record_frame([pair(Name, Expr) | RestFields], Env,
                   [pair(Name, thunk(Expr, Env)) | RestPairs],
                   [Name-thunk(Expr, Env) | RestFrame]) :-
    build_record_frame(RestFields, Env, RestPairs, RestFrame).

% Record field lookup. Optional+missing → null; required+missing → fail
% (matching Rust's field-not-found error, which produces empty stdout in
% the differential).
field_lookup(Pairs, Name, _Opt, Forced) :-
    member(pair(Name, V), Pairs),
    !,
    force(V, Forced).
field_lookup(_, _, opt, null).

% List item access. nth0 succeeds when index is in range; on miss we fall
% through to opt → null or fail (required missing).
list_index_opt(Items, Idx, _Opt, Forced) :-
    nth0(Idx, Items, V),
    !,
    force(V, Forced).
list_index_opt(_, _, opt, null).

% --- eval-4 helpers ---

% Build the error record from `error E`'s operand. Text → standard
% [Reason, Message, Detail] shape; record → use as-is.
build_error_record(text(Cs), record([
    pair("Reason",  text("Expression.Error")),
    pair("Message", text(Cs)),
    pair("Detail",  null)
])).
build_error_record(record(Pairs), record(Pairs)).

% Default error record when a predicate failure (not a thrown error) is
% caught by `try`. Mirrors the lifted form Rust uses for internal MError
% variants — Reason = "Expression.Error", Message = a synthesised string.
default_error_record(record([
    pair("Reason",  text("Expression.Error")),
    pair("Message", text("evaluation failed")),
    pair("Detail",  null)
])).

% `try` success-record builder.
try_success_record(V, record([
    pair("HasError", bool(false)),
    pair("Value",    V)
])).

% `try` failure-record builder.
try_failure_record(ErrRec, record([
    pair("HasError", bool(true)),
    pair("Error",    ErrRec)
])).

% --- eval-5 helpers ---

% Interpret an AST term as a type representation. Mirror of Rust's
% evaluate_as_type. The `null` keyword reaches the parser as the atom
% `null` (parser produces `primary(null)`), so we handle that explicitly
% in addition to the ref(Chars) path.
eval_as_type(null, type_prim(null)).
eval_as_type(ref(Cs), type_prim(Name)) :-
    primitive_type_name(Cs, Name).
eval_as_type(unop(nullable, Inner), type_nullable(T)) :-
    eval_as_type(Inner, T).
% Compound type expressions (list_type/_, record_type/_, table_type/_,
% function_type/_) are deferred per design doc — they simply fail here,
% which propagates as empty stdout for the differential.

primitive_type_name([a,n,y],                                    any).
primitive_type_name([a,n,y,n,o,n,n,u,l,l],                      anynonnull).
primitive_type_name([n,u,l,l],                                  null).
primitive_type_name([l,o,g,i,c,a,l],                            logical).
primitive_type_name([n,u,m,b,e,r],                              number).
primitive_type_name([t,e,x,t],                                  text).
primitive_type_name([d,a,t,e],                                  date).
primitive_type_name([d,a,t,e,t,i,m,e],                          datetime).
primitive_type_name([d,u,r,a,t,i,o,n],                          duration).
primitive_type_name([b,i,n,a,r,y],                              binary).
primitive_type_name([l,i,s,t],                                  list).
primitive_type_name([r,e,c,o,r,d],                              record).
primitive_type_name([t,a,b,l,e],                                table).
primitive_type_name([f,u,n,c,t,i,o,n],                          function).
primitive_type_name([t,y,p,e],                                  type).

% Conformance test between a value term and a type representation.
type_conforms(_,             type_prim(any))         :- !.
type_conforms(V,             type_prim(anynonnull))  :- !, V \= null.
type_conforms(null,          type_prim(null))        :- !.
type_conforms(bool(_),       type_prim(logical)).
type_conforms(num(_),        type_prim(number)).
type_conforms(text(_),       type_prim(text)).
type_conforms(date(_),       type_prim(date)).
type_conforms(datetime(_),   type_prim(datetime)).
type_conforms(duration(_),   type_prim(duration)).
type_conforms(binary(_),     type_prim(binary)).
type_conforms(list(_),       type_prim(list)).
type_conforms(record(_),     type_prim(record)).
type_conforms(table(_),      type_prim(table)).
type_conforms(closure(_,_,_),type_prim(function)).
type_conforms(type_value(_), type_prim(type)).
type_conforms(null,          type_nullable(_))       :- !.
type_conforms(V,             type_nullable(Inner))   :- type_conforms(V, Inner).

% Deep force — recursively forces thunks inside lists/records. The harness
% calls this on the top-level result before printing, mirroring the Rust
% `deep_force` helper used by `value_dump.rs`.
deep_force(thunk(Expr, Env), Forced) :-
    !,
    eval(Expr, Env, V0),
    deep_force(V0, Forced).
deep_force(list(Items), list(Forced)) :-
    !,
    deep_force_list(Items, Forced).
deep_force(record(Pairs), record(ForcedPairs)) :-
    !,
    deep_force_pairs(Pairs, ForcedPairs).
deep_force(V, V).

deep_force_list([], []).
deep_force_list([V | Vs], [F | Fs]) :-
    deep_force(V, F),
    deep_force_list(Vs, Fs).

deep_force_pairs([], []).
deep_force_pairs([pair(N, V) | Rest], [pair(N, F) | RestF]) :-
    deep_force(V, F),
    deep_force_pairs(Rest, RestF).

% --- helpers ---

not_bool(true, false).
not_bool(false, true).

compare_values(Op, num(A), num(B), Result) :-
    !,
    ( call(Op, A, B) -> Result = true ; Result = false ).
compare_values(Op, text(A), text(B), Result) :-
    !,
    ( text_compare(Op, A, B) -> Result = true ; Result = false ).
% Other type combinations fail — matches Rust's TypeMismatch.

text_compare(<,  A, B) :- A @<  B.
text_compare(=<, A, B) :- A @=< B.
text_compare(>,  A, B) :- A @>  B.
text_compare(>=, A, B) :- A @>= B.

values_equal(null,    null,    true) :- !.
values_equal(bool(B), bool(B), true) :- !.
values_equal(num(A),  num(B),  Result) :-
    !,
    ( A =:= B -> Result = true ; Result = false ).
values_equal(text(Cs), text(Cs), true) :- !.
values_equal(_, _, false).

% Number parsing — hex (0x.../0X...) goes through manual digit accumulation;
% decimal goes through number_chars then float() coercion to ensure parity
% with Rust's f64.
chars_to_number(Cs, F) :-
    ( Cs = ['0', X | Hex], eval_hex_marker(X)
    -> eval_hex_chars_to_int(Hex, Int),
       F is float(Int)
    ; number_chars(N, Cs),
      F is float(N)
    ).

% Renamed to avoid colliding with lexical.pl's hex_marker/1 — scryer's
% discontiguous-across-files warning silently breaks goal initialization.
eval_hex_marker('x').
eval_hex_marker('X').

eval_hex_chars_to_int(Hex, Int) :- eval_hex_acc(Hex, 0, Int).

eval_hex_acc([], Acc, Acc).
eval_hex_acc([C | Cs], Acc, Int) :-
    eval_hex_value(C, V),
    Acc1 is Acc * 16 + V,
    eval_hex_acc(Cs, Acc1, Int).

eval_hex_value(C, V) :-
    ( C @>= '0', C @=< '9'
    -> char_code(C, X), V is X - 0'0
    ;   ( C @>= 'a', C @=< 'f'
        -> char_code(C, X), V is X - 0'a + 10
        ; char_code(C, X), V is X - 0'A + 10
        )
    ).

% --- print_value/1: canonical S-expression emit ---
%
% Format must agree byte-for-byte with mrsflow-core/examples/value_dump.rs.
% See design doc §07 "Canonical value format for the differential".

print_value(null) :- format("(null)", []).
print_value(bool(true))  :- format("(bool true)", []).
print_value(bool(false)) :- format("(bool false)", []).
print_value(num(F)) :-
    % Number rendering — for the scaffold, ~w; canonical form locks in slice-1
    % once we see what Rust's f64::to_string does in practice. Both sides
    % will converge then.
    format("(num ~w)", [F]).
print_value(text(Cs)) :-
    format("(text ", []),
    eval_print_quoted(Cs),
    format(")", []).
print_value(date(Cs)) :-
    format("(date ", []),
    eval_print_quoted(Cs),
    format(")", []).
print_value(datetime(Cs)) :-
    format("(datetime ", []),
    eval_print_quoted(Cs),
    format(")", []).
print_value(duration(Cs)) :-
    format("(duration ", []),
    eval_print_quoted(Cs),
    format(")", []).
print_value(binary(_Bytes)) :-
    % Slice-when-needed will define a canonical hex form.
    format("(binary ...)", []).
print_value(list(Items)) :-
    format("(list (", []),
    print_value_list(Items),
    format("))", []).
print_value(record(Pairs)) :-
    format("(record (", []),
    print_record_pairs(Pairs),
    format("))", []).
print_value(table(_)) :-
    % Placeholder — real format lands when eval-7 brings in the Arrow-backed
    % representation on the Rust side; Prolog table comparison is bounded to
    % small enough cases that a list-of-records form will suffice.
    format("(table ...)", []).
print_value(closure(_, _, _)) :-
    % Per spec, function equality is reference equality; canonical interior
    % printing isn't well-defined. Both sides emit `(function ...)` as a
    % placeholder.
    format("(function ...)", []).
print_value(type_value(type_prim(Name))) :-
    !,
    format("(type-value ~w)", [Name]).
print_value(type_value(type_nullable(T))) :-
    !,
    format("(type-value (nullable ", []),
    print_type_inner(T),
    format("))", []).
print_value(type_value(_)) :-
    % Fallback for any future TypeRep variants not yet handled — keeps the
    % differential noticing divergence rather than silently emitting nothing.
    format("(type-value ...)", []).

print_type_inner(type_prim(Name)) :- format("~w", [Name]).
print_type_inner(type_nullable(T)) :-
    format("(nullable ", []),
    print_type_inner(T),
    format(")", []).
print_value(thunk(_, _)) :-
    % Forcing should happen before printing — if a thunk reaches the printer
    % unforced, that's a bug somewhere. Emit a marker for visibility.
    format("(thunk ...)", []).

print_value_list([]).
print_value_list([V]) :- print_value(V).
print_value_list([V1, V2 | Rest]) :-
    print_value(V1),
    format(" ", []),
    print_value_list([V2 | Rest]).

print_record_pairs([]).
print_record_pairs([P]) :- print_record_pair(P).
print_record_pairs([P1, P2 | Rest]) :-
    print_record_pair(P1),
    format(" ", []),
    print_record_pairs([P2 | Rest]).

print_record_pair(pair(NameChars, Value)) :-
    format("(", []),
    eval_print_quoted(NameChars),
    format(" ", []),
    print_value(Value),
    format(")", []).

% Same quoting logic as syntactic.pl's print_quoted, but renamed because
% both files load into the same scryer session and duplicate predicate
% definitions break -g initialization (silent loader failure).
eval_print_quoted(Cs) :-
    format("\"", []),
    eval_print_quoted_chars(Cs),
    format("\"", []).

eval_print_quoted_chars([]).
eval_print_quoted_chars([C | Cs]) :-
    eval_print_escaped_char(C),
    eval_print_quoted_chars(Cs).

eval_print_escaped_char('"')  :- format("\\\"", []).
eval_print_escaped_char('\\') :- format("\\\\", []).
eval_print_escaped_char('\n') :- format("\\n", []).
eval_print_escaped_char('\r') :- format("\\r", []).
eval_print_escaped_char('\t') :- format("\\t", []).
eval_print_escaped_char(C) :-
    \+ memberchk(C, ['"', '\\', '\n', '\r', '\t']),
    put_char(C).

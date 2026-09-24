# 1. God Objects and Mixed Responsibilities

## 1.2 HirQueueBuilder — scheduler + hoister + resolver + cycle detector
Location: crates/hir/src/builders/mod.rs:69-91 (methods across function.rs, component.rs, styles.rs, structs.rs)
Problem: Combines work scheduling (WorkChannel loops), function hoisting, component hoisting, stylesheet hoisting, struct-method resolution (find_self_type, resolve_method), AST→HIR type lowering (HirNode), and cycle detection — all sharing mutable DashMap/DashSet/RefCell state. HirNode::find_type_named_as (builders/mod.rs:96-242) alone is a 147-line method handling struct creation, enum creation (including discriminant counters and repr validation), component signature resolution, and alias expansion.
Reference: Zig's Sema is a single-purpose pass with explicit - a Compilation object owns scheduling separately; Swift separates Decl construction from semantic analysis.
Suggested fix: Split into: (1) a DeclarationHoister (lookup + enqueue), (2) a TypeLowerer (AST Type → HIR type, already needed for find_type/find_self_type duplication), (3) a BodyResolver (process the work channel), (4) per-kind minimodules for struct/enum/component signature computation.

## 1.5 SlynxIR — 8 storage responsibilities via one struct
Location: crates/ir/src/ir.rs:32-49, impls spread over 4 files (ir.rs, api.rs, queries.rs, values.rs) plus Deref<Target=IRTypes>
Problem: SlynxIR holds globals, functions, components, labels, instructions, impure_instructions, IRTypes, and the string interner — and silently re-exports ~15 type constructors through Deref, so ir.create_struct(...) (real method) and ir.insert_type(...) (Deref) are indistinguishable at call sites. rustc's TyCtxt and lld/Lex contexts keep storage behind explicit fields or query interfaces.
Suggested fix: Drop the Deref to IRTypes; make type construction an explicit ir.types.int_type() API. Group the impl split by concern (Storage, Api, Queries).
# 2. Repeated Code Patterns

## 2.1 Four copies of the monomorphization skeleton
Location: crates/monomorphizer/src/functions.rs:62-116, structs.rs:85-144, components.rs:89-134, enums.rs:62-122
Problem: All four resolve_*_target methods copy the identical sequence: look up template by name (three separate find_*_declaration_by_name linear scans, structs.rs:150-163/components.rs:244-258/enums.rs:127-140) → validate arity → cache-check → cycle-check (in_progress) → build substitution → mangle name → create declaration → cache insert → dead-code mark. This is ~60 lines × 4. The neutralize_*_generics functions are similarly identical apart from the pool name.
Suggested fix: A generic specialize<T>(req: SpecializationRequest<T>, spec_fn) -> Result<AnyDeclarationId> helper parameterized over the declaration kind, plus one generic find_declaration_by_name.

## 2.2 The parser precedence cascade — 6 near-identical functions
Location: crates/parser/src/expr.rs:400-572 (parse_multiplicative, parse_additive, parse_bitoperation, parse_comparison, parse_match, parse_logical)
Problem: Six copies of the "parse LHS, while peek is an operator, parse RHS, fold into Binary" skeleton differing only in the operator→Operator mapping. Each carries a defensive _ => unreachable!() that is genuinely unreachable in five of them — and genuinely buggy in parse_bitoperation (see §5.1).
Reference: Hand-written recursive descent in Swift/Rustc for this case uses either a TokenKind → Operator lookup table consumed by a single loop or an explicit precedence table.
Suggested fix: One parse_infix(precedence, next_prec, is_op, f: fn(&TokenKind) -> Operator) helper; kills the cascade and the unreachable!() sites at once.

## 2.3 The comma-separated-list loop appears ~12 times
Location: expr.rs:42-57, 192-200, 580-589, 602-611, types.rs:100-106, 209-225, functions.rs:14-21, enums.rs:44-50, 62-68, 89-95, declarations.rs:46-55, styles.rs:180-190, import.rs:55-60
Problem: Same "parse item → if peek == separator eat → if peek == terminator break" shape everywhere, with inconsistent trailing-comma policies and terminator sets (some use Comma|SemiColon, some just Comma; styles.rs:71 differs).
Suggested fix: parse_separated<T>(term, sep, allow_trailing, item: impl FnMut(&mut Self) -> Result<T>) -> Result<Vec<T>>.

## 2.4 Duplicated field-access lowering with drift
Location: crates/codegen/src/expressions.rs:161-200 (read) vs crates/codegen/src/instructions.rs:50-97 (write)
Problem: The read and write paths of field access re-implement the same three-way dispatch (external / deref-of-struct / plain) and the deref pointer-type computation — but inconsistently: the read path checks ty_viewer().concrete_type().is_struct() (expressions.rs:183-187) while the write path checks is_mutable_ref().is_struct() (instructions.rs:57-59) and even panics on an immutable ref (instructions.rs:62-65). The external-field intern+lookup is copy-pasted a third time at expressions.rs:326-328.
Suggested fix: One fn field_type_of(&mut self, expr, field_index, hir, ir) -> Result<IRTypeId> that dispatches once; both paths call it.

## 2.5 Dead-attribute-processing and hoisting postamble
Location: crates/hir/src/builders/function.rs:53-64, component.rs:126-136, styles.rs:64-74, mod.rs:468-480
Problem: The "process_attributes → write back to file → declarations pool" sequence after every hoist is identical 4 times over.
Suggested fix: fn attach_attributes(&mut self, file: FileId, id: AnyLocalDeclarationId, attrs: &[ASTAttribute]).

## 2.6 Near-identical build_array / build_vector
Location: crates/hir/src/builders/expression/collections.rs:153-211, 213-261
Problem: Two ~50-line methods differing only in the final Array(ty, len) vs Vector(ty) wrap.
Suggested fix: One build_sequence(expected, elements, kind).

## 2.7 Duplicated error-construction sites
Location: codegen types.rs:18-28 and types.rs:71-91 (the messages "{decl:?} should map to an Object/Enum" are each duplicated with identical strings), plus 3 near-identical "layout is not registered" strings (queries.rs:83-86, expressions.rs:42, expressions.rs:434).
Suggested fix: A single enum_layout(ty) -> Result<&EnumLayout> accessor returning one typed error variant.

# 3. Leaky Abstractions and Wrong Layer Violations

## 3.1 The parser does name resolution and semantic rejection
Location: crates/parser/src/types.rs:83-89 and types.rs:289-296
Problem: generic_param_index resolves an identifier against the in-scope generic list during parsing — and the failure mode is silent: an unknown identifier becomes Type::Plain(...) (types.rs:291-296), so a misspelled generic is reported as a failed name lookup in a completely different phase. parse_typedname even special-cases self/&mut self into Reference(Self) (types.rs:25-58), and Self is synthesized as a plain type name. Additionally, statement.rs:106-118 makes an assignability judgment at parse time: a non-assignable LHS silently falls through to the Expression branch and then surfaces as a confusing UnexpectedToken.
Reference: rustc and Swift parse first, then resolve names in a dedicated semantic pass; the parser produces syntax trees, never scoped types.
Suggested fix: Introduce a Type::Named(SymbolPointer) form that defers resolution; the HIR builder or a name-resolution pre-pass turns it into Generic(index) or an error. if/assign disambiguation that depends on assignment rules should be a semantic check.

## 3.4 Name mangling lives in semantic code
Location: crates/hir/src/builders/structs.rs:109-113 (resolve_method builds "{struct}_{method}")
Problem: Method identity is stored by mangled string in HIR; mangling is a codegen concern (rustc mangles only at the symbol-name stage).
Suggested fix: Store methods by semantic identity in HIR; rename only when emitting IR globals.

## 3.5 Codegen repeats scope management and type resolution that HIR already did
Location: crates/codegen/src/functions.rs:10-53 (FunctionContext re-derives a variable scope; arg Values are re-derived via context.arguments() + add_variable at functions.rs:77-86 instead of using the Values FunctionBuilder::set_function_type already returns), and crates/codegen/src/expressions.rs:184-190, 429-444 (re-reads enum variants/discriminants and field types that FieldAccess already carries).
Suggested fix: Make FunctionBuilder::set_function_type's returned arg values the single path; have the HIR carry pre-resolved variant indexes so codegen stops re-deriving enum shape.

# 4. Missing Abstractions

## 4.1 No traversal/visitor abstraction — ownership analysis is a manual match
Location: crates/hir/src/ownership/analysis.rs:355 (manual match over every HirExpressionKind/HirStatement), also queries.rs:112-124 (flatten_type), styles.rs:166-191 (flatten_struct_value)
Problem: Every consumer that needs to recurse over HIR re-walks the tree by hand. New expression kinds silently break the analysis until a panicking unreachable! or a missed case appears. rustc uses a Visit/walk_* infrastructure precisely to avoid this.
Suggested fix: A Visit/Walk trait (or generated fold_*/walk_* functions) over HirExpression/HirStatement, plus a flatten_type that is the single implementation reused by the ownership pass and codegen (flatten_struct_value at codegen styles.rs:166-191 duplicates it).

## 4.2 The FindType duplication
Location: crates/hir/src/builders/mod.rs:245-322 (HirNode::find_type) vs crates/hir/src/builders/structs.rs:12-49 (find_self_type)
Problem: Two recursive AST-Typ→HIR-type walkers that differ only in the Type::Plain => self substitution.
Suggested fix: One lower_type(ty, self_substitute: Option<SymbolPointer>) traversal.

## 4.3 DedupPool/Pool with concurrent primitives in single-threaded code
Location: crates/common/src/pool/mod.rs:11-124 (boxcar::Vec + DashMap for what is uniformly used single-threaded), crates/monomorphizer/src/lib.rs:89 (cache: DashMap), crates/compilation_context/mod.rs:35 (DashMap file cache)
Problem: Three crates use lock-free/concurrent structures with no threading anywhere in the pipeline. This adds per-op overhead, memory overhead, and — critically — DedupPool::get_mut has a documented "mutating a value invalidates the DashMap hash key" footgun (pool/mod.rs:68-78) that nothing guards against.
Reference: Zig's self-hosted compiler uses plain arenas and std.AutoHashMap; no concurrency until Compilation explicitly parallelizes.
Suggested fix: Use Vec + HashMap unless/until parallel phase compilation is actually implemented. If interior mutation is needed, add an explicit "insert-only" invariant or a downgrade_to_mut barrier.

## 4.4 HirViewer is a manual vtable
Location: crates/hir/src/helpers/views/mod.rs:10-40 and per-kind impls
Problem: HirViewer<'a, T> + a separate impl for every pool id type is hand-rolled dispatch. is_copy_type is a free function (ownership/mod.rs:81-86) that should be a viewer method.
Suggested fix: Traits (HasName, HasFields, HasVariants) implemented per type, or a single HirViewer::of(id) that returns an enum view.

## 4.5 No error-standing helper for the parser's most common error
Location: crates/parser/src/*: Err(ParseError::UnexpectedToken(self.eat()?, ExpectedContent::Raw(format!(...)))) hand-assembled at expr.rs:51,241,389, component.rs:26,93,124,131-134, declarations.rs:129-135, functions.rs:124-130, types.rs:217-222 with inconsistent to_string()/to_owned()/into()/slice conventions.
Suggested fix: fn unexpected<T>(&mut self, msg: impl Into<String>) -> Result<T, ParseError> and make ExpectedContent::Raw(String) accept impl Into<String>.

# 5. Error Handling Inconsistencies

## 5.1 CONFIRMED CRASH: right-shift >> panics with unreachable!()
Location: crates/parser/src/expr.rs:458-470
Problem: The lexer has no combined >> token; is_shiftright() (expr.rs:447-449) checks for two consecutive Gt tokens. In parse_bitoperation, the loop guard enters on is_shiftright()?, then eat() consumes the first Gt, and the inner _ if self.is_shiftright()? re-check (expr.rs:469) now sees only one remaining Gt → returns false → unreachable!() at expr.rs:470. let y = x >> 1 panics the compiler (verified by static trace; the agent confirmed with a run). Left shift works only because it's a single token. This is plausibly the single highest-impact bug in the codebase — a trivial, legal program kills the compiler.
Suggested fix: Capture the two-token-ness when the loop is entered: let is_rs = self.is_shiftright()?; then dispatch on (eat, is_rs) — or lex >> as one token.

## 5.2 Ad-hoc InternalError(String) grab-bag with no spans
Location: crates/codegen/src/error.rs — InternalError(String) absorbs ~11 distinct messages across types.rs:18,25,71,89,107,116, queries.rs:84, expressions.rs:33,36,42,57,434,459. IRError (crates/ir/src/error.rs:23) is two unconstrained parallel enums (Kind, Description) with no std::error::Error impl, no Display, no span data.
Problem: With no spans, codegen/IR errors regress the diagnostic quality that the HIR and parser achieve; users reporting bugs get "InternalError: layout not registered." CodegenError also carries raw pool ids (indices) as Debug output.
Suggested fix: Typed variants (EnumLayoutMissing, NotAnEnum, InvalidVariantIndex, UnknownVariable) with spans threaded from HIR expressions, and a proper Display/Error impl (or workspace-wide thiserror/color_eyre).

## 5.4 Panic-first culture where Result exists
Location: 13× switch_to_block(...).unwrap() in codegen (functions.rs:103, instructions.rs:20,24,30, expressions.rs:217,236,482,508,533,536,539, styles.rs:223,284) — all bypassing IRError; expect("Variable not found for assignment") at instructions.rs:47 while the identical read-position condition returns UnrecognizedVariable at expressions.rs:375; queries.rs:58,65,70 expects in HIR.
Problem: These are exactly the "should have been a Result" patterns the review asked for: the error data structure exists on the other side of the unwrap.
Suggested fix: A ctx.block(label) -> Result<(), CodegenError> wrapper mapping IRError→CodegenError; replace the assignment-page expect with the same error as the read path.

## 5.5 Two disabled unimplemented!() panic paths
Location: crates/monomorphizer/src/lib.rs:248,253 — unimplemented!("monomorphization of generic aliases/styles is not implemented yet")
Problem: A user hitting a generic alias crashes the compiler with a backtrace instead of a diagnostic.
Suggested fix: Return HIRError::not_implemented(span, "generic aliases") invoking the normal diagnostics path.

## 5.6 LangItems error mismatch
Location: crates/hir/src/context/lang_items.rs:19,25-28
Problem: register panics while get returns Result<_, String> — an ad-hoc String error in a crate whose entire error surface is HIRError. Callers must .map_err().
Suggested fix: register returns Result, and both use HIRError.

## 5.7 suggestions_from_parser is a stub that can never return a suggestion
Location: src/compilation_context/errors/helpers.rs:139-144
Problem: The SlynxSuggestion::UnexpectedToken variant exists with a Display impl (helpers.rs:76,107-108) but no parse error ever produces it. Same for handle_ownership_error (always empty suggestions).
Suggested fix: Construct UnexpectedToken { expected, found } from ParseError::UnexpectedToken, which already carries both.

## 5.8 Copy-paste errors in docs and a bugs-in-errors typo
Location: crates/hir/src/error.rs:375-395 (three error constructors all documented as "InvalidStyleDefinition"), error.rs:640,642,648 ("infered", "Atempt", "imutable"), crates/ir/src/model/styles.rs and elsewhere.
Problem: Diagnostic messages users will read contain typos; copy-paste doc comments hide what constructors actually do.
Suggested fix: Proofread the user-facing Display strings; fix the doc comments.

# 6. Phase Ordering and Pipeline Clarity

## 6.1 Driver exists but phase boundaries are leaky
Location: src/compilation_context/mod.rs — SlynxContext::compile/build_stages/build_hir (:342-405)
Good: There is a clear orchestrator (SlynxContext) with one entry point, comparable to Zig's Compilation — the best part of the pipeline design.
Problem:
- build_stages (mod.rs:382-405) re-implements load_modules (mod.rs:321) with a slightly different on_load closure instead of calling it — a duplication that will drift.
- build_tokens and build_parser (:304-319) are public and dead — the real pipeline lexes/parses inside SourceLoader. The public API advertises a phase split that doesn't exist.
- Monomorphization runs inside build_hir (:342-360) rather than as a separate stage, and the monomorphizer mutates the HIR in place (it hollows out generic templates and splices specializations into the same pools). rustc treats specialization/monomorphization as a distinct staged phase that produces new IR.
- The ownership analysis and monomorphizer both run between HIR build and codegen with no container; results (dead-code set, ownership) are passed positionally as tuples (SlynxHir, HashSet, OwnershipAnalysis) (:342).
Suggested fix: Make SourceLoader expose a single parse_modules entry used by both; delete build_tokens/build_parser or route them through SourceLoader; create a MonomorphizedHir boundary type and a pipeline module that names each phase explicitly.

## 6.2 A two-phase stylesheet pass does the same work twice
Location: crates/codegen/src/lib.rs:232-257 (stylesheet_pre_pass) vs crates/codegen/src/helper/styles.rs:75-79
Problem: stylesheet_pre_pass computes inheritance + property codes in phase 0; lower_stylesheet then recomputes and overwrites them. Per-stylesheet double resolution of the same graph.
Suggested fix: Make pre_pass the single place that resolves inheritance; lower_stylesheet consumes the result.

## 6.4 impure_instructions is a global stream without invalidation guards
Location: crates/ir/src/ir.rs:44, labels slice instruction_start..start+count into the global impure_instructions
Problem: Coordinates are global and silently break if functions aren't lowered strictly sequentially; there's no check that a label's range still matches after other functions are appended. The label doc (label.rs:7-9) says "flat instruction array" while views carve impure_instructions (views/function.rs:7-9) — documentation and implementation disagree.

# 7. Naming and Discoverability

## 7.2 Misspellings that encode API and state
- parse_tupleparse_tuple_with_first (expr.rs:154-182) — a dead function with a duplicated typo'd name (parse_tuple_ + parse_tuple_...); never called, shadows the live parse_tuple_with_first (expr.rs:130-152) and diverges from it silently.

## 7.3 SlynxErrorType display strings disagree with variant names
Location: src/compilation_context/errors/mod.rs:33-52 — Hire displays as "Name Resolution Error", Compilation as "Compilation Error" while variants are Lexer|Parser|Hir|Type|Compilation. No Codegen variant exists (see 5.3).

# 8. Testing Gaps
 
## 8.1 Five of eight crates have zero unit tests
Location: crates/common, crates/module_loader, crates/codegen, crates/hir (context/queries), crates/ir builders+cfg have no unit tests. tests/parser.rs is literally #[test] fn test_parser() {}.
Problem: The highest-risk logic — lowering, layout, substitution, CFG — is verified only as end-to-end smoke tests, so failures surface as generic crashes (or worse, silent wrong output) with no isolation.

## 8.2 Error paths: ~3 of 30+ HIRErrorKind variants directly tested
Location: tests/ — functioncall_invalid_args.rs and type_checker.rs cover InvalidFuncallArgLength, InvalidTupleIndex, InvalidTupleAccessTarget, MissingReturn. Never tested: InvalidEnumUsage, InvalidDeref, MatchesOnNonEnum, ConflictingBorrow, CyclicMonomorphization, CouldntInfer, RecursiveType, AmbiguousDeclaration, most write/mutability errors.

## 8.3 IR/SIR output is almost never asserted
Location: tests/if_expression.rs:12-18 is the only content assertion (main + Cbr). All other output tests check only the .sir extension. SFV: no snapshot tests of the SIR formatter.

## 8.4 The currently-broken paths are uncovered
- No test exercises >> (bug 5.1 would be caught immediately by a file with two shifts).
- No tests for CFG construction, get_successors_of, topological order, or the mislabeled Backedge.
- No monomorphizer tests for hash-collision mangling, deep nested generics, or cyclic instantiation.
- No ownership tests for borrow-while-mutably-borrowed, move-while-borrowed, or ownership in loops.
- tests/monomorphizer.rs contains a known-wrong test using expect("expected generic to be properly inferred") (a test that asserts a nicer error string as if it were the success case).

## 8.5 Data-driven tests deliver breadth but hide depth
examples/ dirs with xfail/xpass markers (enums ×25, generics ×31) are the main coverage. They document known gaps (xpass = "should fail, doesn't") but give no signal about which phase failed or whether the SIR output is correct.
Prioritized Refactoring Checklist

#	Change
1	Fix the >> parser crash (expr.rs:458-470) — capture two-token-ness before eat(), or lex >> as a token; add integration tests for shift ops
2	Fix the SIR formatter's {instr} = {instr} dep bug (formatter.rs:433-439); add the missing var_names lookup; add a Cbr-args note
3	Fix the wrong error cross-phase mapping (errors/ir.rs:46 → report codegen failures as Compilation); rename/mislabel fixes for is_negative_int (ir/src/types/mod.rs:80)
4	Collapse the parser's six-operator precedence cascade into one table-driven infix helper; extract parse_separated; delete parse_tuple_parse_tuple_with_first
5	Unify error handling across back half: typed CodegenError variants with spans; a ctx.block() Result wrapper replacing 13 switch_to_block(...).unwrap(); make IRError a real Display+Error
6	De-duplicate the monomorphizer: one generic specialize skeleton + find_declaration_by_name; report generic-alias/styles unimplemented!() as errors; replace DashMap cache with HashMap
7	Split SlynxHir/TypesContext/Codegen god structs: HirStore vs TypeLowerer/TypeRegistry, Codegen → TypeLowerer+NameTable+StyleLowerer
8	Add a Visit/Walk infrastructure for HIR; consolidate flatten_type, find_type/find_self_type, and the safe enum-layout accessor
9	Delete dead machinery: codegen ChildInitWork/component_child_inits/var_id_to_prop_index/StyleApplyData/collect_var_ids_from_expr (~180 LOC), StructBuilder, IRTuple/Slot/Label::get_argument_value
10	Close the testing gap: unit tests for CFG + SIR formatter; snapshot tests for lowering; error-path tests for the 25+ untested variants; a >> regression test; fix the misc-expect in tests/monomorphizer.rs
Items flagged as unclear from reading alone
- The component pipeline (codegen components.rs, ir/src/builder/component.rs): substantial dead scaffolding (component_child_inits never populated — get_component_initcall at components.rs:157-173 always returns empty) means the live component path is hard to distinguish from the vestigial one. Any refactor should first confirm with the maintainers which component features are currently expected to work.
- DedupPool hash-consing semantics: whether the parser's dedup of expression/statement pools (which embed Spans in their Hash) is relied upon anywhere, or whether those pools could be plain Pool without behavioral change — worth confirming before touching.
- The impure_instructions global coordinate scheme: correct today only because codegen lowers functions sequentially; any future parallelization or reordering of lowering must fix this first (flag for the team, not for readers).
- Style numeric code tables (ir/src/model/styles.rs): the STYLES_TABLE lives in code with a panic message that says it belongs in HIR; the actual frontend table was not found in the HIR crate from reading alone.

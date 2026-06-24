; atlas extraction query — C (tree-sitter-c)
;
; Capture-name contract, consumed by src/parse.rs:
;   @definition.<kind>  — whole declaration node; its span becomes the signature
;                         <kind> ∈ function, method, class, interface, enum,
;                                   type, constant, module, field
;   @reference.call     — call site (graph edge: caller → callee)
;   @reference.import   — include (graph edge: file → file)
;   @name               — the identifier inside the enclosing capture
;
; Validate against tests/queries/fixtures/c.c; node names come from
; `cargo run --example dump-ast <file>`, never from guessing. Visibility: a
; `static` (internal-linkage) function/global is file-private (src/parse.rs).
; Kind mapping: struct/union → class.

; Function definitions and prototype declarations (the latter cover headers).
(function_definition
  declarator: (function_declarator
    declarator: (identifier) @name)) @definition.function

(declaration
  declarator: (function_declarator
    declarator: (identifier) @name)) @definition.function

; Aggregate types → class; enums → enum. Require a body so a *reference* to a
; type (e.g. `struct Point` inside a typedef or variable decl) isn't re-captured.
(struct_specifier
  name: (type_identifier) @name
  body: (field_declaration_list)) @definition.class

(union_specifier
  name: (type_identifier) @name
  body: (field_declaration_list)) @definition.class

(enum_specifier
  name: (type_identifier) @name
  body: (enumerator_list)) @definition.enum

; typedefs.
(type_definition
  declarator: (type_identifier) @name) @definition.type

; Struct/union fields (PRD §5.3).
(field_declaration
  declarator: (field_identifier) @name) @definition.field

; Includes: the whole directive is the edge source text.
(preproc_include) @reference.import

; Call sites.
(call_expression
  function: (identifier) @name) @reference.call

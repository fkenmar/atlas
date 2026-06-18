; repomap extraction query — Go (tree-sitter-go)
;
; Capture-name contract, consumed by src/parse.rs:
;   @definition.<kind>  — whole declaration node; its span becomes the signature
;                         <kind> ∈ function, method, class, interface, enum,
;                                   type, constant, module, field
;   @reference.call     — call site (graph edge: caller → callee)
;   @reference.import   — import (graph edge: file → file)
;   @name               — the identifier inside the enclosing capture
;
; Validate against tests/queries/fixtures/go.go; node names come from
; `cargo run --example dump-ast <file>`, never from guessing. Go has no export
; keyword — visibility is decided by the name's first letter in src/parse.rs.
; Kind mapping: struct → class, interface → interface.

; Free functions.
(function_declaration
  name: (identifier) @name) @definition.function

; Methods (declared with a receiver).
(method_declaration
  name: (field_identifier) @name) @definition.method

; Struct types → class.
(type_declaration
  (type_spec
    name: (type_identifier) @name
    type: (struct_type))) @definition.class

; Interface types → interface.
(type_declaration
  (type_spec
    name: (type_identifier) @name
    type: (interface_type))) @definition.interface

; Struct fields (PRD §5.3).
(field_declaration
  name: (field_identifier) @name) @definition.field

; Constants.
(const_declaration
  (const_spec
    name: (identifier) @name)) @definition.constant

; Imports: one edge per spec (handles both grouped and single imports).
(import_spec) @reference.import

; Call sites: bare calls and selector calls (pkg.Func / x.Method).
(call_expression
  function: (identifier) @name) @reference.call

(call_expression
  function: (selector_expression
    field: (field_identifier) @name)) @reference.call

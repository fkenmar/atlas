; repomap extraction query — Rust (tree-sitter-rust)
;
; Capture-name contract, consumed by src/parse.rs:
;   @definition.<kind>  — whole declaration node; its span becomes the signature
;                         <kind> ∈ function, method, class, interface, enum,
;                                   type, constant, module
;   @reference.call     — call site (graph edge: caller → callee)
;   @reference.import   — import (graph edge: file → file)
;   @name               — the identifier inside the enclosing capture
;
; Validate against tests/queries/fixtures/rust.rs; node names come from
; `cargo run --example dump-ast <file>`, never from guessing.
; Kind mapping: struct → class, trait → interface (the cross-language kinds
; src/parse.rs normalizes to).

; Free functions.
(function_item
  name: (identifier) @name) @definition.function

; Methods and associated functions inside impl blocks.
(impl_item
  body: (declaration_list
    (function_item
      name: (identifier) @name) @definition.method))

; Trait method signatures: no body, still definitions.
(trait_item
  body: (declaration_list
    (function_signature_item
      name: (identifier) @name) @definition.method))

; Trait methods with a default body parse as `function_item` (like a free
; function); capture them as methods so they are not mis-tagged. The dedup
; priority in src/parse.rs lets this method match win over the free-function
; pattern that also matches the same node.
(trait_item
  body: (declaration_list
    (function_item
      name: (identifier) @name) @definition.method))

; Types.
(struct_item
  name: (type_identifier) @name) @definition.class

(enum_item
  name: (type_identifier) @name) @definition.enum

(trait_item
  name: (type_identifier) @name) @definition.interface

(type_item
  name: (type_identifier) @name) @definition.type

; Constants and statics.
(const_item
  name: (identifier) @name) @definition.constant

(static_item
  name: (identifier) @name) @definition.constant

; Modules.
(mod_item
  name: (identifier) @name) @definition.module

; macro_rules! definitions.
(macro_definition
  name: (identifier) @name) @definition.function

; Imports: the whole use declaration is the edge source text.
(use_declaration) @reference.import

; Call sites: bare, method, and path-qualified calls.
(call_expression
  function: (identifier) @name) @reference.call

(call_expression
  function: (field_expression
    field: (field_identifier) @name)) @reference.call

(call_expression
  function: (scoped_identifier
    name: (identifier) @name)) @reference.call

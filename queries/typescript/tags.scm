; repomap extraction query — TypeScript/JavaScript (tree-sitter-typescript)
;
; Capture-name contract, consumed by src/parse.rs:
;   @definition.<kind>  — whole declaration node; its span becomes the signature
;                         <kind> ∈ function, method, class, interface, enum,
;                                   type, constant, module
;   @reference.call     — call site (graph edge: caller → callee)
;   @reference.import   — import (graph edge: file → file)
;   @name               — the identifier inside the enclosing capture
;
; Validate against tests/queries/fixtures/typescript.ts; node names come from
; `cargo run --example dump-ast <file>`, never from guessing.
; Overload signatures and ambient (`declare`) declarations parse as
; `function_signature` (no body) — a distinct node kind from
; `function_declaration` — captured by the dedicated rule below so the public
; surface of `.d.ts` files and overload sets is not dropped.

; Function declarations (with a body).
(function_declaration
  name: (identifier) @name) @definition.function

; Overload signatures and ambient `declare function …;` (no body).
(function_signature
  name: (identifier) @name) @definition.function

; Arrow functions / function expressions bound to const/let.
(lexical_declaration
  (variable_declarator
    name: (identifier) @name
    value: [
      (arrow_function)
      (function_expression)
    ]) @definition.function)

; Classes and methods.
(class_declaration
  name: (type_identifier) @name) @definition.class

(method_definition
  name: (property_identifier) @name) @definition.method

; Interfaces, type aliases, enums.
(interface_declaration
  name: (type_identifier) @name) @definition.interface

(type_alias_declaration
  name: (type_identifier) @name) @definition.type

(enum_declaration
  name: (identifier) @name) @definition.enum

; Exported constants (non-function values).
(export_statement
  (lexical_declaration
    (variable_declarator
      name: (identifier) @name) @definition.constant))

; Imports: the module source string is the edge target.
(import_statement
  source: (string) @name) @reference.import

; Call sites: bare calls and member calls (obj.method(...)).
(call_expression
  function: (identifier) @name) @reference.call

(call_expression
  function: (member_expression
    property: (property_identifier) @name)) @reference.call

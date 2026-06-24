; atlas extraction query — Java (tree-sitter-java)
;
; Capture-name contract, consumed by src/parse.rs:
;   @definition.<kind>  — whole declaration node; its span becomes the signature
;                         <kind> ∈ function, method, class, interface, enum,
;                                   type, constant, module, field
;   @reference.call     — call site (graph edge: caller → callee)
;   @reference.import   — import (graph edge: file → file)
;   @name               — the identifier inside the enclosing capture
;
; Validate against tests/queries/fixtures/java.java; node names come from
; `cargo run --example dump-ast <file>`, never from guessing. Visibility: a
; `private`/`protected` modifier marks non-public (src/parse.rs).

; Types.
(class_declaration
  name: (identifier) @name) @definition.class

(interface_declaration
  name: (identifier) @name) @definition.interface

(enum_declaration
  name: (identifier) @name) @definition.enum

; Methods and constructors.
(method_declaration
  name: (identifier) @name) @definition.method

(constructor_declaration
  name: (identifier) @name) @definition.method

; Fields.
(field_declaration
  declarator: (variable_declarator
    name: (identifier) @name)) @definition.field

; Imports: the whole declaration is the edge source text.
(import_declaration) @reference.import

; Call sites.
(method_invocation
  name: (identifier) @name) @reference.call

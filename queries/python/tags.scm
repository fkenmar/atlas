; atlas extraction query — Python (tree-sitter-python)
;
; Capture-name contract, consumed by src/parse.rs:
;   @definition.<kind>  — whole declaration node; its span becomes the signature
;                         <kind> ∈ function, method, class, interface, enum,
;                                   type, constant, module
;   @reference.call     — call site (graph edge: caller → callee)
;   @reference.import   — import (graph edge: file → file)
;   @name               — the identifier inside the enclosing capture
;
; Validate against tests/queries/fixtures/python.py; node names come from
; `cargo run --example dump-ast <file>`, never from guessing.

; Functions (module-level and nested; `async def` shares this node kind).
(function_definition
  name: (identifier) @name) @definition.function

; Methods: a function_definition directly inside a class body.
(class_definition
  body: (block
    (function_definition
      name: (identifier) @name) @definition.method))

; Decorated methods (@property, @staticmethod, …): the decorator wraps the
; def in decorated_definition, so the pattern above misses them.
(class_definition
  body: (block
    (decorated_definition
      definition: (function_definition
        name: (identifier) @name) @definition.method)))

; Decorated definitions: query through the decorator wrapper so
; @property / @staticmethod / @dataclass declarations are not missed.
(decorated_definition
  definition: (function_definition
    name: (identifier) @name) @definition.function)

(decorated_definition
  definition: (class_definition
    name: (identifier) @name) @definition.class)

; Classes.
(class_definition
  name: (identifier) @name) @definition.class

; Module-level assignments; src/parse.rs keeps only UPPER_CASE names as
; constants.
(module
  (expression_statement
    (assignment
      left: (identifier) @name) @definition.constant))

; Class fields: annotated class-body assignments (dataclass/attrs fields,
; typed class attributes) — `name: Type` or `name: Type = default`. PRD §5.3
; shows fields in the map; the `type:` requirement avoids plain method-body
; statements and keeps this to declared attributes.
(class_definition
  body: (block
    (expression_statement
      (assignment
        left: (identifier) @name
        type: (_)) @definition.field)))

; Imports.
(import_statement
  name: (dotted_name) @name) @reference.import

(import_from_statement
  module_name: (dotted_name) @name) @reference.import

; Call sites: bare calls and attribute calls (obj.method(...)).
(call
  function: (identifier) @name) @reference.call

(call
  function: (attribute
    attribute: (identifier) @name)) @reference.call

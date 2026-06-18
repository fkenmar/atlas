; repomap extraction query — C++ (tree-sitter-cpp)
;
; Capture-name contract, consumed by src/parse.rs:
;   @definition.<kind>  — whole declaration node; its span becomes the signature
;                         <kind> ∈ function, method, class, interface, enum,
;                                   type, constant, module, field
;   @reference.call     — call site (graph edge: caller → callee)
;   @reference.import   — include (graph edge: file → file)
;   @name               — the identifier inside the enclosing capture
;
; Validate against tests/queries/fixtures/cpp.cpp; node names come from
; `cargo run --example dump-ast <file>`, never from guessing. Member access
; specifiers (`private:`/`protected:`) live on a sibling `access_specifier`
; node, not the member's own line, so member visibility is resolved by an AST
; pass in src/parse.rs (cpp_private_member_rows), not from the signature text;
; a `static` free function/global stays file-private.
; Kind mapping: struct/class → class, namespace → module.

; Namespaces → module.
(namespace_definition
  name: (namespace_identifier) @name) @definition.module

; Aggregate types (require a body so a bare reference isn't re-captured).
(class_specifier
  name: (type_identifier) @name
  body: (field_declaration_list)) @definition.class

(struct_specifier
  name: (type_identifier) @name
  body: (field_declaration_list)) @definition.class

(enum_specifier
  name: (type_identifier) @name
  body: (enumerator_list)) @definition.enum

; Free function definitions and prototype declarations (headers).
(function_definition
  declarator: (function_declarator
    declarator: (identifier) @name)) @definition.function

(declaration
  declarator: (function_declarator
    declarator: (identifier) @name)) @definition.function

; Class methods: declared (field_declaration) or defined (function_definition)
; inside the class body, identified by a field_identifier name.
(field_declaration
  declarator: (function_declarator
    declarator: (field_identifier) @name)) @definition.method

(function_definition
  declarator: (function_declarator
    declarator: (field_identifier) @name)) @definition.method

; Constructors/destructors declared in the class body: a `declaration` whose
; declarator is a plain identifier. Tagged method; the higher method priority in
; src/parse.rs wins over the free-function match on the same node.
(field_declaration_list
  (declaration
    declarator: (function_declarator
      declarator: (identifier) @name)) @definition.method)

; Member and struct fields.
(field_declaration
  declarator: (field_identifier) @name) @definition.field

; Type aliases: `using X = Y;` and C-style typedefs.
(alias_declaration
  name: (type_identifier) @name) @definition.type

(type_definition
  declarator: (type_identifier) @name) @definition.type

; Includes: the whole directive is the edge source text.
(preproc_include) @reference.import

; Call sites.
(call_expression
  function: (identifier) @name) @reference.call

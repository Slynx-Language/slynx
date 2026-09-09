# Grammar of the Slynx language

## This document specifies the grammar of the Slynx language as implemented by the parser on the `main` branch, using EBNF as the meta-syntax. It reflects what the lexer and parser can actually consume today.

## Utilities

```ebnf
<id> ::= <letter> | <letter> <id>
<letter> ::= "a" | ... | "z" | "A" | ... | "Z" | "_"
<digits> ::= <number> | <number> <digits>
<number> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
<empty> ::= ""
```

## Lexical Structure

### Comments

```ebnf
<line_comment> ::= "//" <any_except_newline>
<block_comment> ::= "/*" <any> "*/"
```

Both are skipped by the lexer.

### Number Literals

```ebnf
<binary_lit> ::= "0b" <binary_digits>
<hex_lit>    ::= "0x" <hex_digits>
<octal_lit>  ::= "0o" <octal_digits>
<decimal_int_lit> ::= <number> { <number> | "_" }
<float_lit> ::= <number> { <number> | "_" } "." <number> { <number> | "_" }
```

- `_` may appear as a digit separator inside numeric literals.
- Integer literals parse as `i32`, float literals as `f32`.

### String Literals

```ebnf
<string_lit> ::= '"' { <string_char> | <escape> } '"'
<escape> ::= "\\n" | "\\t" | "\\r" | "\\\\" | "\\\"" | "\\" <any>
```

## Statements

Statements are separated by semicolons (`;`).

```ebnf
<statement> ::= <let_stmt>
              | <while_stmt>
              | <return_stmt>
              | <assign_stmt>
              | <expr_stmt>

<let_stmt> ::= "let" "mut" <id> <type_opt> "=" <expr> ";" | "let" <id> <type_opt> "=" <expr> ";"
<type_opt> ::= ":" <type> | <empty>
<while_stmt> ::= "while" <expr> <block>
<return_stmt> ::= "return" <expr> ";" | "return" ";"
<assign_stmt> ::= <assignable_expr> "=" <expr> ";"
<expr_stmt> ::= <expr> ";"
<assignable_expr> ::= <id> | <field_access> | <index_access> | <deref_expr>
<block> ::= "{" { <statement> } "}"
```

Notes:

- `let` requires an initializer (`= <expr>`).
- A bare `return;` is only meaningful in a `void` function.

## Expressions

### Precedence

From lowest to highest binding:

1. `||` (logical or)
2. `&&` (logical and)
3. `matches <pattern>`
4. `==` `<` `>` `<=` `>=` (comparison)
5. `<<` `&` `|` `^` (shift/bitwise)
6. `+` `-` (additive)
7. `*` `/` (multiplicative)
8. unary: `&` `&mut` `*` (reference/dereference), literals, calls, postfix

```ebnf
<expr> ::= <if_expr> | <array_lit> | <vector_lit> | <deref_expr> | <ref_expr> | <logical>
<logical> ::= <match_expr> { ("&&" | "||") <match_expr> }
<match_expr> ::= <comparison> { "matches" <primary> }
<comparison> ::= <bit_ops> { ("==" | "<" | ">" | "<=" | ">=") <bit_ops> }
<bit_ops> ::= <additive> { ("<<" | "&" | "|" | "^") <bit_ops> }
<additive> ::= <multiplicative> { ("+" | "-") <multiplicative> }
<multiplicative> ::= <primary> { ("*" | "/") <primary> }

<primary> ::= <int_lit>
            | <float_lit>
            | <string_lit>
            | "true"
            | "false"
            | "null"
            | <id>
            | "(" <expr> ")"
            | <tuple_expr>
            | <func_call>
            | <generic_func_call>
            | <object_expr>
            | <generic_object_expr>
            | <component_expr>
            | <generic_component_expr>

<tuple_expr> ::= "(" <expr> "," { <expr> "," } ")" | "()"
<func_call> ::= <id> "(" <expr_list> ")"
<generic_func_call> ::= <id> "<" <type_list> ">" "(" <expr_list> ")"
<object_expr> ::= <id> "(" { <named_arg> "," } ")"
<generic_object_expr> ::= <id> "<" <type_list> ">" "(" { <named_arg> "," } ")"
<named_arg> ::= <id> ":" <expr>

<expr_list> ::= <expr> | <expr> "," <expr_list> | <empty>

<deref_expr> ::= "*" <expr>
<ref_expr> ::= "&" <expr> | "&mut" <expr>
```

### Postfix

```ebnf
<postfix_chain> ::= <primary> { <postfix> }
<postfix> ::= <field_access> | <method_call> | <tuple_access> | <index_access>
<field_access> ::= "." <id>
<method_call> ::= "." <id> "(" <expr_list> ")"
<tuple_access> ::= "." <non_negative_int>
<index_access> ::= "[" <index> "]"
<index> ::= "" | ":" | <expr> | <expr> ":" | ":" <expr> | <expr> ":" <expr>
```

The index forms map to:

| Syntax | Meaning |
|--------|---------|
| `a[i]` | element access (index) |
| `a[:]` | slice: whole collection |
| `a[i:]` | slice: from index `i` to the end |
| `a[:i]` | slice: from the start to index `i` (exclusive) |
| `a[i:j]` | slice: from `i` to `j` (exclusive) |

### If Expression

```ebnf
<if_expr> ::= "if" <expr> <block> <else_clause>
<else_clause> ::= "else" <if_expr>
                | "else" <block>
                | <empty>
<block> ::= "{" { <statement> } "}"
```

`if` always behaves as an expression. `else if` is valid and chains as nested `if`.

## Types

```ebnf
<type> ::= <primitive_type>
         | <id>
         | "Self"
         | "()"
         | <tuple_type>
         | <ref_type>
         | <array_type>
         | <vector_type>
         | <generic_type>
         | <nullable_type>

<primitive_type> ::= "int" | "float" | "bool" | "str" | "void" | "Component"
<tuple_type> ::= "(" <type> { "," <type> } ")"
<ref_type> ::= "&" <type> | "&mut" <type>
<array_type> ::= "[" <expr> "]" <type>
<vector_type> ::= "[]" <type>
<generic_type> ::= <id> "<" <type_list> ">"
<nullable_type> ::= <type> "?"
<type_list> ::= <type> | <type> "," <type_list>

<arg> ::= <id> ":" <type>
        | "self"
        | "&self"
        | "&mut" "self"
<typed_name> ::= <id> ":" <type>
```

Notes:

- `Name<T...>` syntax is shared by generic types and generic declarations.
- The return type of a function is required.

## Declarations

### Program

```ebnf
<program> ::= { <declaration> }
<declaration> ::= <extern_block>
                | <import_decl>
                | <attributes> <visibility> <top_level_decl>
<top_level_decl> ::= <alias_decl>
                   | <object_decl>
                   | <component_decl>
                   | <func_decl>
                   | <stylesheet_decl>
                   | <static_decl>
                   | <enum_decl>
```

The nine top-level declaration kinds are: `object`, `component`, `func`, `alias`, `enum`, `static`, `stylesheet`, `import`, `extern`.

### Attributes

```ebnf
<attributes> ::= { <attribute> }
<attribute> ::= "@" <id> "(" { <string_lit> "," } ")"
```

### Visibility

```ebnf
<visibility> ::= "pub" | <empty>
```

`pub(parent)` / `pub(child)` are only valid on component members.

### Alias

```ebnf
<alias_decl> ::= "alias" <generic_name> "=" <type> ";"
```

### Object

```ebnf
<object_decl> ::= "object" <generic_name> "{" { <object_member> "," } "}"
<object_member> ::= <field> | <method>
<field> ::= <id> ":" <type>
<method> ::= <func_decl>
```

### Component

```ebnf
<component_decl> ::= "component" <generic_name> "{" { <component_member> } "}"
<component_member> ::= <child_component> | <prop_decl>
<child_component> ::= <component_expr>
<prop_decl> ::= <member_visibility> "prop" <id> ";"
              | <member_visibility> "prop" <id> ":" <type> ";"
              | <member_visibility> "prop" <id> ":" <type> "=" <expr> ";"
              | <member_visibility> "prop" <id> "=" <expr> ";"
<member_visibility> ::= "pub" | "pub" "(" "parent" ")" | "pub" "(" "child" ")" | <empty>

<component_expr> ::= <id> "{" { <component_member_value> "," } "}"
<generic_component_expr> ::= <id> "<" <type_list> ">" "{" { <component_member_value> "," } "}"
<component_member_value> ::= <named_arg> | <component_expr>
```

### Function

```ebnf
<func_decl> ::= "func" <generic_name> "(" <arg_list> ")" ":" <type> <func_body>
<func_body> ::= "->" <expr> ";"
              | "{" { <statement> } "}"
<arg_list> ::= <arg> | <arg> "," <arg_list> | <empty>
```

### Enum

```ebnf
<enum_decl> ::= "enum" <generic_name> <enum_repr> "{" { <enum_variant> "," } "}"
<enum_repr> ::= ":" <type> | <empty>
<enum_variant> ::= <attributes> <variant_name> <variant_kind>
<variant_kind> ::= "=" <expr>           (raw valued)
                 | "(" <type_list> ")"  (associated)
                 | "{" { <typed_name> "," } "}"  (struct-like)
                 | <empty>              (raw)
```

### Static

```ebnf
<static_decl> ::= "static" <id> ":" <type> "=" <expr> ";"
```

### Stylesheet

```ebnf
<stylesheet_decl> ::= "stylesheet" <generic_name> "(" <arg_list> ")" <uses> "{" <stylesheet_members> "}"
<uses> ::= "uses" <func_call> { "," <func_call> } | <empty>
<stylesheet_members> ::= <styles_block> | <statement> { "," <stylesheet_members> }
<styles_block> ::= "styles" "{" { <style_block> } "}"
<style_block> ::= <style_state> "{" { <style_prop> | <style_block> } "}"
<style_prop> ::= <id> ":" <expr> ["," | ";"]
<style_state> ::= <id> { "." <id> } [ "(" <expr> [ ":" <id> ] ")" ]
```

### Import

```ebnf
<import_decl> ::= "import" <module_path> <using> ";"
<module_path> ::= <id> { "." <id> }
<using> ::= "using" <import_usage>
          | "using" "{" { <import_usage> "," } "}"
          | <empty>
<import_usage> ::= <id> | <id> "as" <id>
```

### Extern Blocks

```ebnf
<extern_block> ::= "extern" "{" { <declaration> } "}"
```

Inside an extern block, only declaration *signatures* are parsed (types only, followed by `;`).

## Operators

### Binary Operators

Supported binary operators:

| Operator | Production |
|----------|------------|
| `*` `/` | multiplicative |
| `+` `-` | additive |
| `<<` `&` `|` `^` | shift/bitwise |
| `==` `<` `>` `<=` `>=` | comparison |
| `&&` `||` | logical |
| `matches` | pattern match |

Notes:

- `>>` (right shift) is **not** a lexer token. It was removed because `A<B>>` in expression context conflicts with generic type syntax. There is no `ShiftRight` token.
- `!=` is **not** supported. There is no not-equal token or production.
- Unary bitwise negation (`~`) is lexed as `BitNot` but **not** parsed.
- Compound assignment (`+=`, `-=`, `*=`, `/=`) is lexed (`PlusEq`, `SubEq`, `StarEq`, `SlashEq`) but **not** parsed.
- `for` loops, and `break`/`continue`, are **not** implemented.

## Unimplemented Constructs

The following appear in design material but are not part of the parsed grammar:

- `&atomic T` reference type
- `~` unary not
- `!=` not-equal
- compound assignment operators
- `for` loops
- `break` / `continue`
- `>>` shift right (removed due to generic syntax conflict)
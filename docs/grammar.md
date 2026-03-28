# Grammar of the Slynx language

## This document specifies the entire grammar of the Slynx language version 0.0.1 using BNF as the meta-syntax.


### utils

```bnf
<id> ::= <letter_or_underscore> | <letter_or_underscore> <id>
<letter_or_underscore> ::= "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n" | "o" | "p" | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" | "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z" | "_"
<digits> ::= <number> | <number> <digits>
<number> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
<empty> ::= 
<opMultiplicative> ::= "*" | "/"
<opAdditive> ::= "-" | "+"
<block> ::= "{" <statement_list> "}"
```

### Expresion

```bnf

  <op> ::= "==" | "<" | ">" | "<=" | ">="
  <compound_assign> ::= <expr> "+=" <expr> | <expr> "-=" <expr> | <expr> "*=" <expr> | <expr> "/=" <expr>
  <multiplicative> ::= <primary> <multiplicative_tail>
  <multiplicative_tail> ::= <opMultiplicative> <primary> <multiplicative_tail>
                            | <empty>
  <additive> ::= <multiplicative> <additive_tail>
  
  <additive_tail> ::= <opAdditive> <multiplicative> <additive_tail>
                  | <empty>
  <comparison> ::= <additive> <comparison_tail>
                  
  <comparison_tail> ::= <op> <additive> <comparison_tail>
                                      | <empty>
  <logical> ::= <comparison> <logical_tail>
  <logical_tail> ::= <logical_op> <comparison> <logical_tail>
                     | <empty>
  <logical_op> ::= "&&" | "||"
  <expr> ::= <if> |  <logical>
  <expr_list> ::= <expr> | <expr> "," <expr_list> | <empty>
  <statement_list> ::= <statement> <statement_list> | <empty>
  <statement> ::= <var> | <assignment>  | <expr_stmt>
  <expr_stmt> ::= <expr> ";"
  <assignment> ::= <assignable_expr> "=" <expr> ";"
  <assignable_expr> ::= <id> | <field_access>
```

```bnf
<field_access> ::= <primary> "." <id>
<bool> ::= <logical> | <bool_lit>
<bool_lit> ::= "true" | "false"
<primary> ::= <int_lit>
            | <float_lit>
            | <bool_lit>
            | <string_lit>
            | <id>
            | <compound_assign> ";"
            | <field_access>
            | "(" <expr> ")"
<string_chars> ::= "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "a" | "b" | "c" | "d" | "e" | "f" | "g" | "h" | "i" | "j" | "k" | "l" | "m" | "n" | "o" | "q" | "r" | "s" | "t" | "u" | "v" | "w" | "x" | "y" | "z" | "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O" | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z" | "!" | "#" | "$" | "%" | "&" | "'" | "(" | ")" | "*" | "+" | "," | "-" | "." | "/" | ":" | ";" | "<" | "=" | ">" | "?" | "@" | "[" | "]" | "^" | "_" | "`" | "{" | "|" | "}" | "~" | " " | " " | " "
<int_lit> ::= <digits>
<float_lit> ::= <digits> "." <digits>
<bool_lit> ::= "true" | "false"
<string_lit> ::= "\"" <string_chars> "\""
<type_opt> ::= ":" <type> | <empty>
<type> ::= "int" | "float" | "bool" | <id>

```

### Objects
```bnf
  <field>  ::= <id> ":" <type>
  <field_list> ::= <field> | <field> "," <field_list>
  <object_decl> ::= "object" <id> "{" <field_list> "}"
  <object_expr> ::= <id> "(" <named_arg_list> ")"
  <named_arg_list> ::= <named_arg> | <named_arg> "," <named_arg_list>
  <named_arg> ::= <id> ":" <expr>
```

### Functions

```bnf
  <func> ::= "func" <id> "(" <args> ")" ":" <type> <func_body> 
  <func_body> ::= "->" <expr> ";" | "{" <statement_list> "}"
  
  <args> ::= <arg> | <arg> "," <args> | <empty>
  <arg> ::= <id> ":" <type>
  
  <func_call> ::= <id> "(" <expr_list> ")"

```
### variables

```bnf
<var> ::= "let" <mut_opt> <id> <type_opt> "=" <expr> ";" | <var_opt>
<var_opt> ::= "let" <mut_opt> <id> <type_opt> ";"
<mut_opt> ::= "mut" | <empty>

```
### Conditionals

```bnf
<if> ::= "if" <bool> <block> <elseif> "else" <block>
<elseif> ::= "else if" <bool> <block> | "else if" <bool> <block> <elseif> | <empty>
```

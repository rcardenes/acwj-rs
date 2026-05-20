compound_statement: '{' '}'          // empty, i.e. no statement
     |      '{' statement '}'
     |      '{' statement statements '}'
     ;

statement: print_statement
     |     declaration
     |     assignment_statement
     |     if_statement
     |     while_statement
     |     for_statement
     ;

print_statement: 'print' expression ';'  ;

declaration: 'int' identifier ';'
     |       'char' identifier ';'
     |       'void' identifier ';'
     ;

assignment_statement: identifier '=' expression ';'   ;

if_statement: if_head
     |        if_head 'else' compound_statement
     ;

if_head: 'if' '(' true_false_expression ')' compound_statement  ;

while_statement: 'while' '(' true_false_expression ')' compound_statement  ;

for_statement: 'for' '(' preop_statement ';'
                         true_false_expression ';'
                         postop_statement ')' compound_statement  ;

preop_statement:  statement  ;        (for now)
postop_statement: statement  ;        (for now)

function_declaration: 'void' identifier '(' ')' compound_statement   ;

identifier: T_IDENT ;

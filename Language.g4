grammar Language;
// @lexer::members{
// def nextToken(self):
//     v = super().nextToken()
//     print(v)
//     return v
// }
// @parser::members{
//     self.lol="lol"
// }
prog: (stmt)+;
stmt: expr (NEWLINE+|EOF) #exprStmt
    | assignment (NEWLINE+|EOF) #assignmentStmt
    ;
expr: ID (func_arg (COMMA func_arg)*)? #idExpr
    | expr OBRAK  NEWLINE* expr NEWLINE* CBRAK #listAccess
    | left=expr op=(MULT|DIV|MOD) right=expr #multExpr
    | left=expr op=(PLUS|MINUS) right=expr #addExpr
    | OPAR NEWLINE* expr NEWLINE* CPAR #parExpr
    | value #valueExpr
    ;

value: DECIMAL #numberAtom
    | EXPDECIMAL #expnumberAtom
    | INT #intAtom
    | BOOL #boolAtom
    | STRING #stringAtom
    | NULL #nullAtom
    ;
func_arg: name=ID SEMICOL ASSIGN expr #namedArg
    | ID NEWLINE (stmt NEWLINE)* expr NEWLINE? END #blockArg
    | expr #exprArg
    ;
assignment: name=ID ASSIGN expr;
SEMICOL:';';
COL:':';
COMMA: ',';
FUNCTION: 'fn';
CLASS: 'type';
IF: 'if';
ELSE:'else';
ELIF: 'elif';
UNTIL: 'until';
WHILE: 'while';
FOR: 'for';
END: 'end';
MATCH:'match';
BOOL:(TRUE|FALSE);
DECIMAL: [0-9]+'.'([0-9]*)?
|'.' [0-9]+;
INT: [0-9]+;
EXPDECIMAL:DECIMAL'e'('+'|'-')[0-9]+;
TRUE:'true';
FALSE:'false';
NULL:'null';
OR:'or';
AND:'and';
EQ:'==';
NEQ:'!=';
GT:'>';
LT:'<';
GTEQ:'>=';
LTEQ:'<=';
PLUS:'+';
MINUS:'-';
MULT:'*';
DIV:'/';
MOD:'%';
NOT:'not';
ASSIGN: '=';
CONSTASSIGN:'is';
OPAR:'(';
CPAR:')';
OBRACE:'{';
CBRACE:'}';
OBRAK: '[';
CBRAK: ']';
STRING:'"'(~["\n\r]|('\\'[nrab]))*'"';
ID: [a-zA-Z_][a-zA-Z0-9_]*;
COMMENT:('#' ~[\r\n]* | '#*' (.)* '*#')->skip;
NEWLINE: '\n';
WS: [ \t\r]+ -> skip;
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
stmt: expr #exprStmt
    | ID ASSIGN expr  #assignment
    ;
expr: ID (func_arg)* #idExpr
    | left=expr op=(MULT|DIV|MOD) right=expr #multExpr
    | left=expr op=(PLUS|MINUS) right=expr #addExpr
    | OPAR expr CPAR #parExpr
    | DECIMAL #numberAtom
    | EXPDECIMAL #expnumberAtom
    | INT #intAtom
    | BOOL #boolAtom
    | STRING #stringAtom
    | NULL #nullAtom
    ;
func_arg: ID ASSIGN expr #namedArg
    | ID (stmt)* expr END #blockArg
    | expr #exprArg
    ;
SEMICOL:';';
COL:':';
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
STRING:'"'(~["\n\r]|('\\'[nrab]))*'"';
ID: [a-zA-Z_][a-zA-Z0-9_]*;
COMMENT:('#' ~[\r\n]* | '#*' (.)* '*#')->skip;
WS: [ \t\n\r]+ -> skip;
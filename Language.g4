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
    | USE (DOTTEDID|ID) (NEWLINE+|EOF) #useStmt
    ;
expr: ID (func_arg (COMMA func_arg)*)? #idExpr
    | expr OBRAK  NEWLINE* expr NEWLINE* CBRAK #listAccess
    | OPAR MINUS expr CPAR #uminusExpr
    | NOT expr #unotExpr
    | left=expr op=(MULT|DIV|MOD) right=expr #multExpr
    | left=expr op=(PLUS|MINUS) right=expr #addExpr
    | left=expr op=(EQ|NEQ|LT|GT|LTEQ|GTEQ) right=expr #relatExpr
    | left=expr op=(BITAND|BITOR|BITXOR) right=expr #bitwiseExpr
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
func_arg: expr #exprArg
    | name=ID COL expr #namedArg
    | ID NEWLINE? (stmt NEWLINE)* expr NEWLINE? END #blockArg
    ;
assignment: name=ID ASSIGN expr;
SEMICOL:';';
COL:':';
DOTTEDID: ID (DOT ID)+;
COMMA: ',';
FUNCTION: 'fn';
CLASS: 'type';
// IF: 'if';
// ELSE:'else';
// ELIF: 'elif';
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
USE: 'use';
NULL:'null';
BITOR:'|';
BITAND:'&';
BITXOR:'^';
AND:'and';
OR:'or';
DOT: '.';
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
STRING:'"'(~["\n\r]|('\\'[nrbA])|('\\'[0-9a-fA-F]{2}))*'"';
ID: [a-zA-Z_][a-zA-Z0-9_]*;
COMMENT:('#' ~[\r\n]* | '#*' (.)* '*#')->skip;
NEWLINE: [\n\r]+;
WS: [ \t]+ -> skip;
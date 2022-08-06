# Generated from Language.g4 by ANTLR 4.10.1
# encoding: utf-8
from antlr4 import *
from io import StringIO
import sys
if sys.version_info[1] > 5:
	from typing import TextIO
else:
	from typing.io import TextIO

def serializedATN():
    return [
        4,1,43,67,2,0,7,0,2,1,7,1,2,2,7,2,2,3,7,3,1,0,4,0,10,8,0,11,0,12,
        0,11,1,1,1,1,1,1,1,1,3,1,18,8,1,1,2,1,2,1,2,5,2,23,8,2,10,2,12,2,
        26,9,2,1,2,1,2,1,2,1,2,1,2,1,2,1,2,1,2,1,2,1,2,3,2,38,8,2,1,2,1,
        2,1,2,1,2,1,2,1,2,5,2,46,8,2,10,2,12,2,49,9,2,1,3,1,3,1,3,1,3,1,
        3,5,3,56,8,3,10,3,12,3,59,9,3,1,3,1,3,1,3,1,3,3,3,65,8,3,1,3,0,1,
        4,4,0,2,4,6,0,2,1,0,30,32,1,0,28,29,77,0,9,1,0,0,0,2,17,1,0,0,0,
        4,37,1,0,0,0,6,64,1,0,0,0,8,10,3,2,1,0,9,8,1,0,0,0,10,11,1,0,0,0,
        11,9,1,0,0,0,11,12,1,0,0,0,12,1,1,0,0,0,13,18,3,4,2,0,14,15,5,41,
        0,0,15,16,5,34,0,0,16,18,3,4,2,0,17,13,1,0,0,0,17,14,1,0,0,0,18,
        3,1,0,0,0,19,20,6,2,-1,0,20,24,5,41,0,0,21,23,3,6,3,0,22,21,1,0,
        0,0,23,26,1,0,0,0,24,22,1,0,0,0,24,25,1,0,0,0,25,38,1,0,0,0,26,24,
        1,0,0,0,27,28,5,36,0,0,28,29,3,4,2,0,29,30,5,37,0,0,30,38,1,0,0,
        0,31,38,5,14,0,0,32,38,5,16,0,0,33,38,5,15,0,0,34,38,5,13,0,0,35,
        38,5,40,0,0,36,38,5,19,0,0,37,19,1,0,0,0,37,27,1,0,0,0,37,31,1,0,
        0,0,37,32,1,0,0,0,37,33,1,0,0,0,37,34,1,0,0,0,37,35,1,0,0,0,37,36,
        1,0,0,0,38,47,1,0,0,0,39,40,10,9,0,0,40,41,7,0,0,0,41,46,3,4,2,10,
        42,43,10,8,0,0,43,44,7,1,0,0,44,46,3,4,2,9,45,39,1,0,0,0,45,42,1,
        0,0,0,46,49,1,0,0,0,47,45,1,0,0,0,47,48,1,0,0,0,48,5,1,0,0,0,49,
        47,1,0,0,0,50,51,5,41,0,0,51,52,5,34,0,0,52,65,3,4,2,0,53,57,5,41,
        0,0,54,56,3,2,1,0,55,54,1,0,0,0,56,59,1,0,0,0,57,55,1,0,0,0,57,58,
        1,0,0,0,58,60,1,0,0,0,59,57,1,0,0,0,60,61,3,4,2,0,61,62,5,11,0,0,
        62,65,1,0,0,0,63,65,3,4,2,0,64,50,1,0,0,0,64,53,1,0,0,0,64,63,1,
        0,0,0,65,7,1,0,0,0,8,11,17,24,37,45,47,57,64
    ]

class LanguageParser ( Parser ):

    grammarFileName = "Language.g4"

    atn = ATNDeserializer().deserialize(serializedATN())

    decisionsToDFA = [ DFA(ds, i) for i, ds in enumerate(atn.decisionToState) ]

    sharedContextCache = PredictionContextCache()

    literalNames = [ "<INVALID>", "';'", "':'", "'fn'", "'type'", "'if'", 
                     "'else'", "'elif'", "'until'", "'while'", "'for'", 
                     "'end'", "'match'", "<INVALID>", "<INVALID>", "<INVALID>", 
                     "<INVALID>", "'true'", "'false'", "'null'", "'or'", 
                     "'and'", "'=='", "'!='", "'>'", "'<'", "'>='", "'<='", 
                     "'+'", "'-'", "'*'", "'/'", "'%'", "'not'", "'='", 
                     "'is'", "'('", "')'", "'{'", "'}'" ]

    symbolicNames = [ "<INVALID>", "SEMICOL", "COL", "FUNCTION", "CLASS", 
                      "IF", "ELSE", "ELIF", "UNTIL", "WHILE", "FOR", "END", 
                      "MATCH", "BOOL", "DECIMAL", "INT", "EXPDECIMAL", "TRUE", 
                      "FALSE", "NULL", "OR", "AND", "EQ", "NEQ", "GT", "LT", 
                      "GTEQ", "LTEQ", "PLUS", "MINUS", "MULT", "DIV", "MOD", 
                      "NOT", "ASSIGN", "CONSTASSIGN", "OPAR", "CPAR", "OBRACE", 
                      "CBRACE", "STRING", "ID", "COMMENT", "WS" ]

    RULE_prog = 0
    RULE_stmt = 1
    RULE_expr = 2
    RULE_func_arg = 3

    ruleNames =  [ "prog", "stmt", "expr", "func_arg" ]

    EOF = Token.EOF
    SEMICOL=1
    COL=2
    FUNCTION=3
    CLASS=4
    IF=5
    ELSE=6
    ELIF=7
    UNTIL=8
    WHILE=9
    FOR=10
    END=11
    MATCH=12
    BOOL=13
    DECIMAL=14
    INT=15
    EXPDECIMAL=16
    TRUE=17
    FALSE=18
    NULL=19
    OR=20
    AND=21
    EQ=22
    NEQ=23
    GT=24
    LT=25
    GTEQ=26
    LTEQ=27
    PLUS=28
    MINUS=29
    MULT=30
    DIV=31
    MOD=32
    NOT=33
    ASSIGN=34
    CONSTASSIGN=35
    OPAR=36
    CPAR=37
    OBRACE=38
    CBRACE=39
    STRING=40
    ID=41
    COMMENT=42
    WS=43

    def __init__(self, input:TokenStream, output:TextIO = sys.stdout):
        super().__init__(input, output)
        self.checkVersion("4.10.1")
        self._interp = ParserATNSimulator(self, self.atn, self.decisionsToDFA, self.sharedContextCache)
        self._predicates = None



        self.lol="lol"



    class ProgContext(ParserRuleContext):
        __slots__ = 'parser'

        def __init__(self, parser, parent:ParserRuleContext=None, invokingState:int=-1):
            super().__init__(parent, invokingState)
            self.parser = parser

        def stmt(self, i:int=None):
            if i is None:
                return self.getTypedRuleContexts(LanguageParser.StmtContext)
            else:
                return self.getTypedRuleContext(LanguageParser.StmtContext,i)


        def getRuleIndex(self):
            return LanguageParser.RULE_prog

        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitProg" ):
                return visitor.visitProg(self)
            else:
                return visitor.visitChildren(self)




    def prog(self):

        localctx = LanguageParser.ProgContext(self, self._ctx, self.state)
        self.enterRule(localctx, 0, self.RULE_prog)
        self._la = 0 # Token type
        try:
            self.enterOuterAlt(localctx, 1)
            self.state = 9 
            self._errHandler.sync(self)
            _la = self._input.LA(1)
            while True:
                self.state = 8
                self.stmt()
                self.state = 11 
                self._errHandler.sync(self)
                _la = self._input.LA(1)
                if not ((((_la) & ~0x3f) == 0 and ((1 << _la) & ((1 << LanguageParser.BOOL) | (1 << LanguageParser.DECIMAL) | (1 << LanguageParser.INT) | (1 << LanguageParser.EXPDECIMAL) | (1 << LanguageParser.NULL) | (1 << LanguageParser.OPAR) | (1 << LanguageParser.STRING) | (1 << LanguageParser.ID))) != 0)):
                    break

        except RecognitionException as re:
            localctx.exception = re
            self._errHandler.reportError(self, re)
            self._errHandler.recover(self, re)
        finally:
            self.exitRule()
        return localctx


    class StmtContext(ParserRuleContext):
        __slots__ = 'parser'

        def __init__(self, parser, parent:ParserRuleContext=None, invokingState:int=-1):
            super().__init__(parent, invokingState)
            self.parser = parser


        def getRuleIndex(self):
            return LanguageParser.RULE_stmt

     
        def copyFrom(self, ctx:ParserRuleContext):
            super().copyFrom(ctx)



    class ExprStmtContext(StmtContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.StmtContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def expr(self):
            return self.getTypedRuleContext(LanguageParser.ExprContext,0)


        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitExprStmt" ):
                return visitor.visitExprStmt(self)
            else:
                return visitor.visitChildren(self)


    class AssignmentContext(StmtContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.StmtContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def ID(self):
            return self.getToken(LanguageParser.ID, 0)
        def ASSIGN(self):
            return self.getToken(LanguageParser.ASSIGN, 0)
        def expr(self):
            return self.getTypedRuleContext(LanguageParser.ExprContext,0)


        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitAssignment" ):
                return visitor.visitAssignment(self)
            else:
                return visitor.visitChildren(self)



    def stmt(self):

        localctx = LanguageParser.StmtContext(self, self._ctx, self.state)
        self.enterRule(localctx, 2, self.RULE_stmt)
        try:
            self.state = 17
            self._errHandler.sync(self)
            la_ = self._interp.adaptivePredict(self._input,1,self._ctx)
            if la_ == 1:
                localctx = LanguageParser.ExprStmtContext(self, localctx)
                self.enterOuterAlt(localctx, 1)
                self.state = 13
                self.expr(0)
                pass

            elif la_ == 2:
                localctx = LanguageParser.AssignmentContext(self, localctx)
                self.enterOuterAlt(localctx, 2)
                self.state = 14
                self.match(LanguageParser.ID)
                self.state = 15
                self.match(LanguageParser.ASSIGN)
                self.state = 16
                self.expr(0)
                pass


        except RecognitionException as re:
            localctx.exception = re
            self._errHandler.reportError(self, re)
            self._errHandler.recover(self, re)
        finally:
            self.exitRule()
        return localctx


    class ExprContext(ParserRuleContext):
        __slots__ = 'parser'

        def __init__(self, parser, parent:ParserRuleContext=None, invokingState:int=-1):
            super().__init__(parent, invokingState)
            self.parser = parser


        def getRuleIndex(self):
            return LanguageParser.RULE_expr

     
        def copyFrom(self, ctx:ParserRuleContext):
            super().copyFrom(ctx)


    class ParExprContext(ExprContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.ExprContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def OPAR(self):
            return self.getToken(LanguageParser.OPAR, 0)
        def expr(self):
            return self.getTypedRuleContext(LanguageParser.ExprContext,0)

        def CPAR(self):
            return self.getToken(LanguageParser.CPAR, 0)

        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitParExpr" ):
                return visitor.visitParExpr(self)
            else:
                return visitor.visitChildren(self)


    class IntAtomContext(ExprContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.ExprContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def INT(self):
            return self.getToken(LanguageParser.INT, 0)

        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitIntAtom" ):
                return visitor.visitIntAtom(self)
            else:
                return visitor.visitChildren(self)


    class AddExprContext(ExprContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.ExprContext
            super().__init__(parser)
            self.left = None # ExprContext
            self.op = None # Token
            self.right = None # ExprContext
            self.copyFrom(ctx)

        def expr(self, i:int=None):
            if i is None:
                return self.getTypedRuleContexts(LanguageParser.ExprContext)
            else:
                return self.getTypedRuleContext(LanguageParser.ExprContext,i)

        def PLUS(self):
            return self.getToken(LanguageParser.PLUS, 0)
        def MINUS(self):
            return self.getToken(LanguageParser.MINUS, 0)

        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitAddExpr" ):
                return visitor.visitAddExpr(self)
            else:
                return visitor.visitChildren(self)


    class StringAtomContext(ExprContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.ExprContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def STRING(self):
            return self.getToken(LanguageParser.STRING, 0)

        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitStringAtom" ):
                return visitor.visitStringAtom(self)
            else:
                return visitor.visitChildren(self)


    class BoolAtomContext(ExprContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.ExprContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def BOOL(self):
            return self.getToken(LanguageParser.BOOL, 0)

        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitBoolAtom" ):
                return visitor.visitBoolAtom(self)
            else:
                return visitor.visitChildren(self)


    class ExpnumberAtomContext(ExprContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.ExprContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def EXPDECIMAL(self):
            return self.getToken(LanguageParser.EXPDECIMAL, 0)

        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitExpnumberAtom" ):
                return visitor.visitExpnumberAtom(self)
            else:
                return visitor.visitChildren(self)


    class NumberAtomContext(ExprContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.ExprContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def DECIMAL(self):
            return self.getToken(LanguageParser.DECIMAL, 0)

        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitNumberAtom" ):
                return visitor.visitNumberAtom(self)
            else:
                return visitor.visitChildren(self)


    class MultExprContext(ExprContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.ExprContext
            super().__init__(parser)
            self.left = None # ExprContext
            self.op = None # Token
            self.right = None # ExprContext
            self.copyFrom(ctx)

        def expr(self, i:int=None):
            if i is None:
                return self.getTypedRuleContexts(LanguageParser.ExprContext)
            else:
                return self.getTypedRuleContext(LanguageParser.ExprContext,i)

        def MULT(self):
            return self.getToken(LanguageParser.MULT, 0)
        def DIV(self):
            return self.getToken(LanguageParser.DIV, 0)
        def MOD(self):
            return self.getToken(LanguageParser.MOD, 0)

        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitMultExpr" ):
                return visitor.visitMultExpr(self)
            else:
                return visitor.visitChildren(self)


    class NullAtomContext(ExprContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.ExprContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def NULL(self):
            return self.getToken(LanguageParser.NULL, 0)

        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitNullAtom" ):
                return visitor.visitNullAtom(self)
            else:
                return visitor.visitChildren(self)


    class IdExprContext(ExprContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.ExprContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def ID(self):
            return self.getToken(LanguageParser.ID, 0)
        def func_arg(self, i:int=None):
            if i is None:
                return self.getTypedRuleContexts(LanguageParser.Func_argContext)
            else:
                return self.getTypedRuleContext(LanguageParser.Func_argContext,i)


        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitIdExpr" ):
                return visitor.visitIdExpr(self)
            else:
                return visitor.visitChildren(self)



    def expr(self, _p:int=0):
        _parentctx = self._ctx
        _parentState = self.state
        localctx = LanguageParser.ExprContext(self, self._ctx, _parentState)
        _prevctx = localctx
        _startState = 4
        self.enterRecursionRule(localctx, 4, self.RULE_expr, _p)
        self._la = 0 # Token type
        try:
            self.enterOuterAlt(localctx, 1)
            self.state = 37
            self._errHandler.sync(self)
            token = self._input.LA(1)
            if token in [LanguageParser.ID]:
                localctx = LanguageParser.IdExprContext(self, localctx)
                self._ctx = localctx
                _prevctx = localctx

                self.state = 20
                self.match(LanguageParser.ID)
                self.state = 24
                self._errHandler.sync(self)
                _alt = self._interp.adaptivePredict(self._input,2,self._ctx)
                while _alt!=2 and _alt!=ATN.INVALID_ALT_NUMBER:
                    if _alt==1:
                        self.state = 21
                        self.func_arg() 
                    self.state = 26
                    self._errHandler.sync(self)
                    _alt = self._interp.adaptivePredict(self._input,2,self._ctx)

                pass
            elif token in [LanguageParser.OPAR]:
                localctx = LanguageParser.ParExprContext(self, localctx)
                self._ctx = localctx
                _prevctx = localctx
                self.state = 27
                self.match(LanguageParser.OPAR)
                self.state = 28
                self.expr(0)
                self.state = 29
                self.match(LanguageParser.CPAR)
                pass
            elif token in [LanguageParser.DECIMAL]:
                localctx = LanguageParser.NumberAtomContext(self, localctx)
                self._ctx = localctx
                _prevctx = localctx
                self.state = 31
                self.match(LanguageParser.DECIMAL)
                pass
            elif token in [LanguageParser.EXPDECIMAL]:
                localctx = LanguageParser.ExpnumberAtomContext(self, localctx)
                self._ctx = localctx
                _prevctx = localctx
                self.state = 32
                self.match(LanguageParser.EXPDECIMAL)
                pass
            elif token in [LanguageParser.INT]:
                localctx = LanguageParser.IntAtomContext(self, localctx)
                self._ctx = localctx
                _prevctx = localctx
                self.state = 33
                self.match(LanguageParser.INT)
                pass
            elif token in [LanguageParser.BOOL]:
                localctx = LanguageParser.BoolAtomContext(self, localctx)
                self._ctx = localctx
                _prevctx = localctx
                self.state = 34
                self.match(LanguageParser.BOOL)
                pass
            elif token in [LanguageParser.STRING]:
                localctx = LanguageParser.StringAtomContext(self, localctx)
                self._ctx = localctx
                _prevctx = localctx
                self.state = 35
                self.match(LanguageParser.STRING)
                pass
            elif token in [LanguageParser.NULL]:
                localctx = LanguageParser.NullAtomContext(self, localctx)
                self._ctx = localctx
                _prevctx = localctx
                self.state = 36
                self.match(LanguageParser.NULL)
                pass
            else:
                raise NoViableAltException(self)

            self._ctx.stop = self._input.LT(-1)
            self.state = 47
            self._errHandler.sync(self)
            _alt = self._interp.adaptivePredict(self._input,5,self._ctx)
            while _alt!=2 and _alt!=ATN.INVALID_ALT_NUMBER:
                if _alt==1:
                    if self._parseListeners is not None:
                        self.triggerExitRuleEvent()
                    _prevctx = localctx
                    self.state = 45
                    self._errHandler.sync(self)
                    la_ = self._interp.adaptivePredict(self._input,4,self._ctx)
                    if la_ == 1:
                        localctx = LanguageParser.MultExprContext(self, LanguageParser.ExprContext(self, _parentctx, _parentState))
                        localctx.left = _prevctx
                        self.pushNewRecursionContext(localctx, _startState, self.RULE_expr)
                        self.state = 39
                        if not self.precpred(self._ctx, 9):
                            from antlr4.error.Errors import FailedPredicateException
                            raise FailedPredicateException(self, "self.precpred(self._ctx, 9)")
                        self.state = 40
                        localctx.op = self._input.LT(1)
                        _la = self._input.LA(1)
                        if not((((_la) & ~0x3f) == 0 and ((1 << _la) & ((1 << LanguageParser.MULT) | (1 << LanguageParser.DIV) | (1 << LanguageParser.MOD))) != 0)):
                            localctx.op = self._errHandler.recoverInline(self)
                        else:
                            self._errHandler.reportMatch(self)
                            self.consume()
                        self.state = 41
                        localctx.right = self.expr(10)
                        pass

                    elif la_ == 2:
                        localctx = LanguageParser.AddExprContext(self, LanguageParser.ExprContext(self, _parentctx, _parentState))
                        localctx.left = _prevctx
                        self.pushNewRecursionContext(localctx, _startState, self.RULE_expr)
                        self.state = 42
                        if not self.precpred(self._ctx, 8):
                            from antlr4.error.Errors import FailedPredicateException
                            raise FailedPredicateException(self, "self.precpred(self._ctx, 8)")
                        self.state = 43
                        localctx.op = self._input.LT(1)
                        _la = self._input.LA(1)
                        if not(_la==LanguageParser.PLUS or _la==LanguageParser.MINUS):
                            localctx.op = self._errHandler.recoverInline(self)
                        else:
                            self._errHandler.reportMatch(self)
                            self.consume()
                        self.state = 44
                        localctx.right = self.expr(9)
                        pass

             
                self.state = 49
                self._errHandler.sync(self)
                _alt = self._interp.adaptivePredict(self._input,5,self._ctx)

        except RecognitionException as re:
            localctx.exception = re
            self._errHandler.reportError(self, re)
            self._errHandler.recover(self, re)
        finally:
            self.unrollRecursionContexts(_parentctx)
        return localctx


    class Func_argContext(ParserRuleContext):
        __slots__ = 'parser'

        def __init__(self, parser, parent:ParserRuleContext=None, invokingState:int=-1):
            super().__init__(parent, invokingState)
            self.parser = parser


        def getRuleIndex(self):
            return LanguageParser.RULE_func_arg

     
        def copyFrom(self, ctx:ParserRuleContext):
            super().copyFrom(ctx)



    class NamedArgContext(Func_argContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.Func_argContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def ID(self):
            return self.getToken(LanguageParser.ID, 0)
        def ASSIGN(self):
            return self.getToken(LanguageParser.ASSIGN, 0)
        def expr(self):
            return self.getTypedRuleContext(LanguageParser.ExprContext,0)


        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitNamedArg" ):
                return visitor.visitNamedArg(self)
            else:
                return visitor.visitChildren(self)


    class BlockContext(Func_argContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.Func_argContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def ID(self):
            return self.getToken(LanguageParser.ID, 0)
        def expr(self):
            return self.getTypedRuleContext(LanguageParser.ExprContext,0)

        def END(self):
            return self.getToken(LanguageParser.END, 0)
        def stmt(self, i:int=None):
            if i is None:
                return self.getTypedRuleContexts(LanguageParser.StmtContext)
            else:
                return self.getTypedRuleContext(LanguageParser.StmtContext,i)


        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitBlock" ):
                return visitor.visitBlock(self)
            else:
                return visitor.visitChildren(self)


    class ExprArgContext(Func_argContext):

        def __init__(self, parser, ctx:ParserRuleContext): # actually a LanguageParser.Func_argContext
            super().__init__(parser)
            self.copyFrom(ctx)

        def expr(self):
            return self.getTypedRuleContext(LanguageParser.ExprContext,0)


        def accept(self, visitor:ParseTreeVisitor):
            if hasattr( visitor, "visitExprArg" ):
                return visitor.visitExprArg(self)
            else:
                return visitor.visitChildren(self)



    def func_arg(self):

        localctx = LanguageParser.Func_argContext(self, self._ctx, self.state)
        self.enterRule(localctx, 6, self.RULE_func_arg)
        try:
            self.state = 64
            self._errHandler.sync(self)
            la_ = self._interp.adaptivePredict(self._input,7,self._ctx)
            if la_ == 1:
                localctx = LanguageParser.NamedArgContext(self, localctx)
                self.enterOuterAlt(localctx, 1)
                self.state = 50
                self.match(LanguageParser.ID)
                self.state = 51
                self.match(LanguageParser.ASSIGN)
                self.state = 52
                self.expr(0)
                pass

            elif la_ == 2:
                localctx = LanguageParser.BlockContext(self, localctx)
                self.enterOuterAlt(localctx, 2)
                self.state = 53
                self.match(LanguageParser.ID)
                self.state = 57
                self._errHandler.sync(self)
                _alt = self._interp.adaptivePredict(self._input,6,self._ctx)
                while _alt!=2 and _alt!=ATN.INVALID_ALT_NUMBER:
                    if _alt==1:
                        self.state = 54
                        self.stmt() 
                    self.state = 59
                    self._errHandler.sync(self)
                    _alt = self._interp.adaptivePredict(self._input,6,self._ctx)

                self.state = 60
                self.expr(0)
                self.state = 61
                self.match(LanguageParser.END)
                pass

            elif la_ == 3:
                localctx = LanguageParser.ExprArgContext(self, localctx)
                self.enterOuterAlt(localctx, 3)
                self.state = 63
                self.expr(0)
                pass


        except RecognitionException as re:
            localctx.exception = re
            self._errHandler.reportError(self, re)
            self._errHandler.recover(self, re)
        finally:
            self.exitRule()
        return localctx



    def sempred(self, localctx:RuleContext, ruleIndex:int, predIndex:int):
        if self._predicates == None:
            self._predicates = dict()
        self._predicates[2] = self.expr_sempred
        pred = self._predicates.get(ruleIndex, None)
        if pred is None:
            raise Exception("No predicate with index:" + str(ruleIndex))
        else:
            return pred(localctx, predIndex)

    def expr_sempred(self, localctx:ExprContext, predIndex:int):
            if predIndex == 0:
                return self.precpred(self._ctx, 9)
         

            if predIndex == 1:
                return self.precpred(self._ctx, 8)
         





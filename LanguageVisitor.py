# Generated from Language.g4 by ANTLR 4.10.1
from antlr4 import *
if __name__ is not None and "." in __name__:
    from .LanguageParser import LanguageParser
else:
    from LanguageParser import LanguageParser

# This class defines a complete generic visitor for a parse tree produced by LanguageParser.

class LanguageVisitor(ParseTreeVisitor):

    # Visit a parse tree produced by LanguageParser#prog.
    def visitProg(self, ctx:LanguageParser.ProgContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#exprStmt.
    def visitExprStmt(self, ctx:LanguageParser.ExprStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#assignmentStmt.
    def visitAssignmentStmt(self, ctx:LanguageParser.AssignmentStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#useStmt.
    def visitUseStmt(self, ctx:LanguageParser.UseStmtContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#bitwiseExpr.
    def visitBitwiseExpr(self, ctx:LanguageParser.BitwiseExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#parExpr.
    def visitParExpr(self, ctx:LanguageParser.ParExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#listAccess.
    def visitListAccess(self, ctx:LanguageParser.ListAccessContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#valueExpr.
    def visitValueExpr(self, ctx:LanguageParser.ValueExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#addExpr.
    def visitAddExpr(self, ctx:LanguageParser.AddExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#unotExpr.
    def visitUnotExpr(self, ctx:LanguageParser.UnotExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#uminusExpr.
    def visitUminusExpr(self, ctx:LanguageParser.UminusExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#relatExpr.
    def visitRelatExpr(self, ctx:LanguageParser.RelatExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#multExpr.
    def visitMultExpr(self, ctx:LanguageParser.MultExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#idExpr.
    def visitIdExpr(self, ctx:LanguageParser.IdExprContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#numberAtom.
    def visitNumberAtom(self, ctx:LanguageParser.NumberAtomContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#expnumberAtom.
    def visitExpnumberAtom(self, ctx:LanguageParser.ExpnumberAtomContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#intAtom.
    def visitIntAtom(self, ctx:LanguageParser.IntAtomContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#boolAtom.
    def visitBoolAtom(self, ctx:LanguageParser.BoolAtomContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#stringAtom.
    def visitStringAtom(self, ctx:LanguageParser.StringAtomContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#nullAtom.
    def visitNullAtom(self, ctx:LanguageParser.NullAtomContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#exprArg.
    def visitExprArg(self, ctx:LanguageParser.ExprArgContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#namedArg.
    def visitNamedArg(self, ctx:LanguageParser.NamedArgContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#blockArg.
    def visitBlockArg(self, ctx:LanguageParser.BlockArgContext):
        return self.visitChildren(ctx)


    # Visit a parse tree produced by LanguageParser#assignment.
    def visitAssignment(self, ctx:LanguageParser.AssignmentContext):
        return self.visitChildren(ctx)



del LanguageParser
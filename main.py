from enum import Enum
from typing import List
from langtypes import Types, Value
from functions import Function
from llvmlite import ir, binding as llvm
from LanguageLexer import LanguageLexer
from LanguageParser import LanguageParser
from LanguageVisitor import LanguageVisitor
from antlr4 import InputStream, CommonTokenStream

from utils import generate_format

class BlockState(Enum):
    anon = ()
    cond = ()
    func = ()

class LV(LanguageVisitor):
    def __init__(self, filename:str="a.out.ll") -> None:
        super().__init__()
        self.module = ir.Module(filename)
        self.funcs = [Function(self.module, "main", [Types.i64, Types.stringptr.as_pointer()], ["argc", "argv"], Types.i64)]
        self.funcs[-1].scopehnd.add_entry_block()
        #self.builders = [self.funcs[-1].scopehnd.add_entry_block()]
        #self.scopes = [dict()]
        self.voidptr_ty = Types.i8.as_pointer()
        printf_ty = ir.FunctionType(Types.i32, [self.voidptr_ty], var_arg=True)
        self.printf = ir.Function(self.module, printf_ty, name="printf")
        self.last_expr = None
        self._prev_last_expr = None
    # def visitChildren(self, node):
    #     #print(type(node))
    #     return super().visitChildren(node)
    def visitAssignment(self, ctx: LanguageParser.AssignmentContext):
        id = str(ctx.ID())
        print(type(ctx.expr()))
        expr: ir.Constant = self.visit(ctx.expr())
        print("assignment",id,expr)
        self.funcs[-1].scopehnd.set(id, expr)
        print(self.funcs[-1].scopehnd.scopes)
        return expr
    def visitIntAtom(self, ctx: LanguageParser.IntAtomContext):
        val = Types.i64(int(str(ctx.INT() or 0)))
        return val
    def visitNumberAtom(self, ctx: LanguageParser.NumberAtomContext):
        val = Types.floatt(float(str(ctx.DECIMAL() or 0)))
        return val
    def visitAddExpr(self, ctx: LanguageParser.AddExprContext):
        left: ir.Constant = self.visit(ctx.left)
        right: ir.Constant = self.visit(ctx.right)
        op = ctx.op.type
        match [left.type, right.type]:
            case [x,y] if x in Types.ints and y in Types.ints and x==y:
                return (self.funcs[-1].builder.add if op==LanguageParser.PLUS else self.funcs[-1].builder.sub)(left, right)
            case [Types.floatt, y] if y in Types.ints:
                right = self.funcs[-1].builder.sitofp(right, Types.floatt)
                return (self.funcs[-1].builder.fadd if op==LanguageParser.PLUS else self.funcs[-1].builder.fsub)(left,right)
            case [x, Types.floatt] if x in Types.ints:
                left = self.funcs[-1].builder.sitofp(left, Types.floatt)
                return (self.funcs[-1].builder.fadd if op==LanguageParser.PLUS else self.funcs[-1].builder.fsub)(left,right)
            case [Types.floatt, Types.floatt]:
                return (self.funcs[-1].builder.fadd if op==LanguageParser.PLUS else self.funcs[-1].builder.fsub)(left,right)
            case _:
                raise ValueError(f"cant combine {left} and {right}")
    def visitParExpr(self, ctx: LanguageParser.ParExprContext):
        return self.visit(ctx.expr())
    def visitMultExpr(self, ctx: LanguageParser.MultExprContext):
        left: ir.Constant = self.visit(ctx.left)
        right: ir.Constant = self.visit(ctx.right)
        op = ctx.op.type
        match [left.type, right.type, op]:
            case [x,y,LanguageParser.MULT] if x in Types.ints and y in Types.ints and x==y:
                return self.funcs[-1].builder.mul(left, right)
            case [x,y,LanguageParser.DIV] if x in Types.ints and y in Types.ints and x==y:
                return self.funcs[-1].builder.sdiv(left, right)
            case [x,y,LanguageParser.MOD] if x in Types.ints and y in Types.ints and x==y:
                return self.funcs[-1].builder.srem(left, right)
            case [Types.floatt, y, LanguageParser.MULT] if y in Types.ints:
                right = self.funcs[-1].builder.sitofp(right, Types.floatt)
                return self.funcs[-1].builder.fmul(left,right)
            case [Types.floatt, y, LanguageParser.DIV] if y in Types.ints:
                right = self.funcs[-1].builder.sitofp(right, Types.floatt)
                return self.funcs[-1].builder.fdiv(left,right)
            case [Types.floatt, y, LanguageParser.MOD] if y in Types.ints:
                right = self.funcs[-1].builder.sitofp(right, Types.floatt)
                return self.funcs[-1].builder.frem(left,right)
            case [x, Types.floatt, LanguageParser.MULT] if x in Types.ints:
                left = self.funcs[-1].builder.sitofp(left, Types.floatt)
                return self.funcs[-1].builder.fmul(left,right)
            case [x, Types.floatt, LanguageParser.DIV] if x in Types.ints:
                left = self.funcs[-1].builder.sitofp(left, Types.floatt)
                return self.funcs[-1].builder.fdiv(left,right)
            case [x, Types.floatt, LanguageParser.MOD] if x in Types.ints:
                left = self.funcs[-1].builder.sitofp(left, Types.floatt)
                return self.funcs[-1].builder.frem(left,right)
            case [Types.floatt, Types.floatt, LanguageParser.MULT]:
                return self.funcs[-1].builder.mul(left, right)
            case [Types.floatt, Types.floatt, LanguageParser.DIV]:
                return self.funcs[-1].builder.sdiv(left, right)
            case [Types.floatt, Types.floatt, LanguageParser.MOD]:
                return self.funcs[-1].builder.srem(left, right)
            case _:
                raise ValueError(f"cant combine {left} and {right}")
    
    def visitExprStmt(self, ctx: LanguageParser.ExprStmtContext):
         self.last_expr=super().visitExprStmt(ctx)
         return self.last_expr
    
    def visitListAccess(self, ctx: LanguageParser.ListAccessContext):
        lis = self.visit(ctx.expr()[0])
        index = self.visit(ctx.expr()[1])
        print(lis,index)
        return self.funcs[-1].builder.load(self.funcs[-1].builder.gep(lis,[index]))

    def visitIdExpr(self, ctx: LanguageParser.IdExprContext):
        id = str(ctx.ID())
        print(id)
        exprs: List[Value] = ctx.func_arg()
        for i,expr in enumerate(exprs):
            print(type(expr))
            exprs[i] = self.visit(expr)
        print("exprs", exprs)
        if id=="print":
            print(exprs)
            fmt_arg = generate_format(exprs, self.module, self.funcs[-1].builder.fpext)
            fmt_arg = self.funcs[-1].builder.bitcast(fmt_arg, self.voidptr_ty)
            return self.funcs[-1].builder.call(self.printf, [fmt_arg, *exprs])
        elif id=="i8":
            if len(exprs)==0:
                return Types.i8(0)
            if exprs[0].type in Types.ints:
                return self.funcs[-1].builder.trunc(exprs[0], Types.i8)
        elif id=="i16":
            if exprs[0].type in Types.ints:
                return self.funcs[-1].builder.trunc(exprs[0], Types.i16)
        elif id=="i32":
            if exprs[0].type in Types.ints:
                return self.funcs[-1].builder.trunc(exprs[0], Types.i32)
        elif len(exprs) == 0:
            try:
                return self.funcs[-1].scopehnd.get(id)
            except ValueError:
                fn=list(filter(lambda f:f.name==id and len(f.arg_t)==0, self.funcs))[0]
                return self.funcs[-1].builder.call(fn, [])


if __name__ == "__main__":
    data = InputStream("x=(5.5*8+3/2.)\nprint argc, x, argv[0]")
    lexer = LanguageLexer(data)
    stream = CommonTokenStream(lexer)
    parser = LanguageParser(stream)
    tree = parser.prog()
    visitor = LV()
    visitor.visit(tree)
    visitor.funcs[-1].builder.ret((visitor.last_expr if visitor.last_expr is not None and visitor.last_expr.type == Types.i64 else None) or ir.IntType(64)(0))
    f=repr(visitor.module)
    print(f)
    file = open("a.out.ll","w")
    file.write(f)
    file.close()
    llvm.initialize()
    llvm.initialize_native_target()
    llvm.initialize_native_asmprinter()
    llvm_module = llvm.parse_assembly(str(visitor.module))
    tm = llvm.Target.from_default_triple().create_target_machine()
    print(tm.triple)
    with llvm.create_mcjit_compiler(llvm_module, tm) as ee:
        ee.finalize_object()
        print(tm.emit_assembly(llvm_module))
        open("out","wb").write(tm.emit_object(llvm_module))


from enum import Enum
from langtypes import Types
from functions import Function
from llvmlite import ir, binding as llvm
from LanguageLexer import LanguageLexer
from LanguageParser import LanguageParser
from LanguageVisitor import LanguageVisitor
from antlr4 import InputStream, CommonTokenStream

class BlockState(Enum):
    anon = ()
    cond = ()
    func = ()

class LV(LanguageVisitor):
    def __init__(self, filename:str="a.out.ll") -> None:
        super().__init__()
        self.module = ir.Module(filename)
        self.funcs = [Function(self.module, "main", [], Types.i64)]
        self.funcs[-1].scopehnd.add_entry_block()
        #self.builders = [self.funcs[-1].scopehnd.add_entry_block()]
        self.scopes = [dict()]
        self.voidptr_ty = Types.i8.as_pointer()
        fmt_i = "%d\n\0"
        c_fmti = ir.Constant(ir.ArrayType(Types.i8, len(fmt_i)),
                            bytearray(fmt_i.encode("utf8")))
        self.global_fmti = ir.GlobalVariable(self.module, c_fmti.type, name="fstri")
        self.global_fmti.linkage = 'internal'
        self.global_fmti.global_constant = True
        self.global_fmti.initializer = c_fmti  # type: ignore
        fmt_f = "%f\n\0"
        c_fmtf = ir.Constant(ir.ArrayType(Types.i8, len(fmt_f)),
                            bytearray(fmt_f.encode("utf8")))
        self.global_fmtf = ir.GlobalVariable(self.module, c_fmtf.type, name="fstrf")
        self.global_fmtf.linkage = 'internal'
        self.global_fmtf.global_constant = True
        self.global_fmtf.initializer = c_fmtf #type: ignore
        printf_ty = ir.FunctionType(Types.i32, [self.voidptr_ty], var_arg=True)
        self.printf = ir.Function(self.module, printf_ty, name="printf")
        self.last_expr = None
        self._prev_last_expr = None


    def visitAssignment(self, ctx: LanguageParser.AssignmentContext):
        id = str(ctx.ID())
        expr = self.visit(ctx.expr())
        print(id,expr)
        self.scopes[-1][id] = expr
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
        op = str(ctx.op)
        match [left.type, right.type]:
            case [Types.i64, Types.i64]:
                return (self.funcs[-1].builder.add if op=="+" else self.funcs[-1].builder.sub)(left, right)
            case [Types.floatt, Types.i64]:
                right = self.funcs[-1].builder.sitofp(right, Types.floatt)
                return (self.funcs[-1].builder.fadd if op=="+" else self.funcs[-1].builder.fsub)(left,right)
            case [Types.i64, Types.floatt]:
                left = self.funcs[-1].builder.sitofp(left, Types.floatt)
                return (self.funcs[-1].builder.fadd if op=="+" else self.funcs[-1].builder.fsub)(left,right)
            case [Types.floatt, Types.floatt]:
                return (self.funcs[-1].builder.fadd if op=="+" else self.funcs[-1].builder.fsub)(left,right)
            case _:
                pass
    def visitExprStmt(self, ctx: LanguageParser.ExprStmtContext):
        self.last_expr=super().visitExprStmt(ctx)
        return self.last_expr


    def visitIdExpr(self, ctx: LanguageParser.IdExprContext):
        id = str(ctx.ID())
        print(id)
        exprs = ctx.func_arg()
        if len(exprs) == 0:
            return self.scopes[-1][id]
        elif id=="print":
            expr = self.visit(exprs[0])
            print(expr)
            match expr.type:
                case Types.i64|Types.i32|Types.i16|Types.i8:
                    fmti_arg = self.funcs[-1].builder.bitcast(self.global_fmti, self.voidptr_ty)
                    return self.funcs[-1].builder.call(self.printf, [fmti_arg, expr])
                case Types.floatt:
                    fmtf_arg = self.funcs[-1].builder.bitcast(self.global_fmtf, self.voidptr_ty)
                    expr = self.funcs[-1].builder.fpext(expr, Types.double)
                    return self.funcs[-1].builder.call(self.printf, [fmtf_arg, expr])

if __name__ == "__main__":
    data = InputStream("x=5.5-2\nprint x")
    lexer = LanguageLexer(data)
    stream = CommonTokenStream(lexer)
    parser = LanguageParser(stream)
    tree = parser.prog()
    visitor = LV()
    visitor.visit(tree)
    visitor.funcs[-1].builder.ret((visitor.last_expr if visitor.last_expr.type == Types.i64 else None) or ir.IntType(64)(0))
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


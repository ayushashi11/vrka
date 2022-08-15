from enum import Enum
import re
import subprocess
from typing import List, Tuple
from langtypes import Types, Value
from functions import Function
from llvmlite import ir, binding as llvm
from LanguageLexer import LanguageLexer
from langbuiltins import libtable, func_builtins, clibs
from LanguageParser import LanguageParser
from LanguageVisitor import LanguageVisitor
from antlr4 import InputStream, CommonTokenStream

from utils import cast_list, generate_format, test_equalities

backsnum_regex = re.compile(r"\\([0-9a-fA-F]{2})")

class BlockState(Enum):
    anon = ()
    cond = ()
    func = ()

class LV(LanguageVisitor):
    def __init__(self, filename:str="a") -> None:
        super().__init__()
        self.module = ir.Module(filename)
        self.funcs = [Function(self.module, "lol", [], [], ir.VoidType()),Function(self.module, "vrkaMain", [Types.i32, Types.stringptr.as_pointer()], ["argc", "argv"], Types.i32)]
        self.funcs[-1].scopehnd.add_entry_block()
        #self.builders = [self.funcs[-1].scopehnd.add_entry_block()]
        #self.scopes = [dict()]
        self.voidptr_ty = Types.i8.as_pointer()
        printf_ty = ir.FunctionType(Types.i32, [self.voidptr_ty], var_arg=True)
        self.printf = ir.Function(self.module, printf_ty, name="printf")
        self.last_expr = None
        self._prev_last_expr = None
        self.definitions = []
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
    def visitStringAtom(self, ctx: LanguageParser.StringAtomContext):
        st = str(ctx.STRING()).strip("\"'").replace("\\t","\t").replace("\\\\","\\").replace("\\A","\a").replace("\\n","\n").replace("\\r","\r")
        for el in backsnum_regex.findall(st):
            print('\\'+el, el)
            st=st.replace('\\'+el,chr(int(el,16)))
        st = Types.generatestring(st)
        print(st)
        stptr = self.funcs[-1].builder.alloca(st.type)
        self.funcs[-1].builder.store(st, stptr)
        print(st.type,stptr.type)
        return stptr
    def visitUseStmt(self, ctx: LanguageParser.UseStmtContext):
        val = str(ctx.ID() or ctx.DOTTEDID())
        self.definitions.append(val.upper())
        for defn in func_builtins[val.upper()]:
            self.funcs.insert(0, defn(self.module))

    def visitIntAtom(self, ctx: LanguageParser.IntAtomContext):
        val = Types.i64(int(str(ctx.INT() or 0)))
        return val
    def visitNumberAtom(self, ctx: LanguageParser.NumberAtomContext):
        val = Types.floatt(float(str(ctx.DECIMAL() or 0)))
        return val
    def visitExpnumberAtom(self, ctx: LanguageParser.ExpnumberAtomContext):
        val = Types.floatt(float(str(ctx.EXPDECIMAL() or 0)))
        return val
    def visitBoolAtom(self, ctx: LanguageParser.BoolAtomContext):
        val = str(ctx.BOOL())
        return Types.bool(int(val=="true"))
    def visitNamedArg(self, ctx: LanguageParser.NamedArgContext):
        id = str(ctx.ID())
        expr = ctx.expr()
        return ("named", id, expr)
    def visitUminusExpr(self, ctx: LanguageParser.UminusExprContext):
        print("uminus")
        expr = self.visit(ctx.expr())
        match expr.type:
            case x if x in Types.ints:
                return self.funcs[-1].builder.neg(expr)
            case Types.floatt:
                return self.funcs[-1].builder.fneg(expr)
            case Types.string:
                if "STRUTILS" not in self.definitions:
                    self.definitions.append("STRUTILS")
                    self.funcs.insert(0,Function(self.module, "strrev", [Types.stringptr], ["st"], Types.stringptr))
                expr = self.funcs[-1].builder.bitcast(expr, Types.stringptr)
                return self.funcs[-1].builder.call(self.getFunction("strrev", [Types.stringptr]).fnptr, [expr])
            case Types.stringptr:
                if "STRUTILS" not in self.definitions:
                    self.definitions.append("STRUTILS")
                    self.funcs.insert(0,Function(self.module, "strrev", [Types.stringptr], ["st"], Types.stringptr))
                return self.funcs[-1].builder.call(self.getFunction("strrev", [Types.stringptr]).fnptr, [expr])
            case x:
                raise ValueError("unary minus not defined")
    def getFunction(self, name, ftype, exprs=[]) -> Function:
        lis = list(filter(lambda f: f.name==name and f.arg_t==ftype, self.funcs))
        if len(lis)==0: 
            lis = list(filter(lambda f: f.name==name and test_equalities(f.arg_t,ftype), self.funcs))
            cast_list(exprs, lis[0].arg_t, self.funcs[-1].builder)
        assert len(lis)==1, f"{lis} {ftype[0]}"
        return lis[0]
    def visitRelatExpr(self, ctx: LanguageParser.RelatExprContext):
        left=self.visit(ctx.left)
        right=self.visit(ctx.right)
        match ctx.op.type:
            case LanguageParser.EQ: op="=="
            case LanguageParser.NEQ: op="!="
            case LanguageParser.GT: op=">"
            case LanguageParser.LT: op="<"
            case LanguageParser.GTEQ: op=">="
            case LanguageParser.LTEQ: op="<="
            case _: raise ValueError("unknown comparison operation")
        return self.funcs[-1].builder.icmp_signed(op,left,right)
    def visitBitwiseExpr(self, ctx: LanguageParser.BitwiseExprContext):
        left = self.visit(ctx.left)
        right = self.visit(ctx.right)
        match ctx.op.type:
            case LanguageParser.BITAND: return self.funcs[-1].builder.and_(left, right)
            case LanguageParser.BITOR: return self.funcs[-1].builder.or_(left, right)
            case LanguageParser.BITXOR: return self.funcs[-1].builder.xor(left, right)
    def visitAddExpr(self, ctx: LanguageParser.AddExprContext):
        left: ir.Constant = self.visit(ctx.left)
        right: ir.Constant = self.visit(ctx.right)
        op = ctx.op.type
        print("add",op)
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
        print("listaccess",lis,index)
        return self.funcs[-1].builder.load(self.funcs[-1].builder.gep(lis,[index]))

    def visitIdExpr(self, ctx: LanguageParser.IdExprContext):
        id = str(ctx.ID())
        print(id)
        exprs: List[Value|Tuple[str,str,LanguageParser.ExprContext]] = ctx.func_arg()
        for i,expr in enumerate(exprs):
            print(type(expr))
            exprs[i] = self.visit(expr)
        print("exprs", exprs)
        if id=="print":
            print(exprs)
            fmt_arg = generate_format(exprs, self.module, self.funcs[-1].builder,end="")
            fmt_arg = self.funcs[-1].builder.bitcast(fmt_arg, self.voidptr_ty)
            return self.funcs[-1].builder.call(self.printf, [fmt_arg, *exprs])
        elif id=="println":
            print(exprs)
            fmt_arg = generate_format(exprs, self.module, self.funcs[-1].builder)
            fmt_arg = self.funcs[-1].builder.bitcast(fmt_arg, self.voidptr_ty)
            return self.funcs[-1].builder.call(self.printf, [fmt_arg, *exprs])
        elif id=="i8":
            if len(exprs)==0:
                return Types.i8(0)
            if exprs[0].type in Types.ints:
                return self.funcs[-1].builder.trunc(exprs[0], Types.i8)
        elif id=="i16":
            if len(exprs)==0:
                return Types.i16(0)
            if exprs[0].type in Types.ints:
                return self.funcs[-1].builder.trunc(exprs[0], Types.i16)
        elif id=="i32":
            if len(exprs)==0:
                return Types.i32(0)
            if exprs[0].type in Types.ints:
                return self.funcs[-1].builder.trunc(exprs[0], Types.i32)
        elif id=="float":
            return Types.floatt(0.0)
        elif id=="string":
            st = Types.generatestring("")
            stptr = self.funcs[-1].builder.alloca(st.type)
            self.funcs[-1].builder.store(st, stptr)
            return stptr
        elif id=="if":
            with self.funcs[-1].builder.if_else(exprs[0]) as (then, otherwise):
                with then:
                    then_block = self.funcs[-1].builder.basic_block
                    do: Tuple[str,str,LanguageParser.ExprContext] = list(filter(lambda x: isinstance(x,tuple) and x[1]=="do", exprs))[0]
                    do_val: Value = self.visit(do[2])
                with otherwise:
                    else_block = self.funcs[-1].builder.basic_block
                    else_: Tuple[str,str,LanguageParser.ExprContext] = list(filter(lambda x: isinstance(x,tuple) and x[1]=="else", exprs))[0]
                    else_val: Value = self.visit(else_[2])
            assert do_val.type==else_val.type, "If else must return equal types"
            out_phi = self.funcs[-1].builder.phi(do_val.type)
            out_phi.add_incoming(do_val, then_block)
            out_phi.add_incoming(else_val,else_block)
            return out_phi
        elif len(exprs) > 0:
            fn = self.getFunction(id, list(map(lambda x:x.type, exprs)), exprs)
            return self.funcs[-1].builder.call(fn.fnptr, exprs)
        elif len(exprs) == 0:
            try:
                print("id")
                return self.funcs[-1].scopehnd.get(id)
            except ValueError:
                fn=self.getFunction(id,[])
                return self.funcs[-1].builder.call(fn.fnptr, [])


if __name__ == "__main__":
    #data = InputStream("x=(5.5*8+3/2.)\nprint argc, x, (if argc>=2, do: (argv[argc-1]), else: argv[0])\nif false, do: (print 1), else: print 0")
    data = InputStream("use strutils\nuse math\nprintln strrepr \"\\n\\a0\\0a\\t\"\nprintln pow 4.5,5")
    lexer = LanguageLexer(data)
    stream = CommonTokenStream(lexer)
    parser = LanguageParser(stream)
    tree = parser.prog()
    visitor = LV()
    visitor.visit(tree)
    visitor.funcs[-1].builder.ret((visitor.last_expr if visitor.last_expr is not None and visitor.last_expr.type == Types.i32 else None) or ir.IntType(32)(0))
    f=repr(visitor.module)
    llvm.initialize()
    llvm.initialize_native_target()
    llvm.initialize_native_asmprinter()
    llvm_module = llvm.parse_assembly(str(visitor.module))
    tm = llvm.Target.from_default_triple().create_target_machine()
    print(tm.triple)
    with llvm.create_mcjit_compiler(llvm_module, tm) as ee:
        ee.finalize_object()
        f=tm.emit_assembly(llvm_module)
        with open("out.S","w") as file:
            file.write(f)
        print(llvm_module)
        open("out","wb").write(tm.emit_object(llvm_module))
        args = ["clang", "-fstack-protector", "stdlib/main.o", "stdlib/utils.o", "out","-o","prog"]
        #stdlibargs = ["clang", "-c", "stdlib/main.c", "stdlib/utils.c"]
        for df in visitor.definitions:
            print("linking",df)
            args.append(libtable[df])
            for clib in clibs[df]:
                args.append(clib)
        #print("Building stdlib")
        #subprocess.Popen(stdlibargs)
        print("compiling into executable")
        subprocess.Popen(args)

import re, os, uuid
import llvmlite.ir as ir
from llvmlite.ir.values import Function
import llvmlite.binding as llvm
from lark import Lark, Token, ast_utils, Tree, v_args
from lark.tree import pydot__tree_to_png
from lark.visitors import Interpreter
from typing import List
import ctypes, argparse, pathlib
from .utils import cast_binop, cast

class LateInitStr:
    def __init__(self, st=None) -> None:
        self.st = st
    def __str__(self) -> str:
        if self.st is None:
            raise ValueError("LateInitStr not initialized")
        return self.st
    def __repr__(self) -> str:
        return str(self)

BoolType = ir.IntType(1)
class UIntType(ir.IntType):
    pass
@v_args(inline=True)
class FileParser(Interpreter):
    INTREGEX = r"^i[0-9]+$"
    UINTREGEX = r"^u[0-9]+$"
    def __init__(self, file_name: str):
        super().__init__()
        self.module = ir.Module(file_name)
        Str = ir.global_context.get_identified_type("Str")
        Str.set_body(ir.IntType(8).as_pointer(), ir.IntType(64))
        Ret = ir.global_context.get_identified_type("Ret")
        ret_array = ir.ArrayType(ir.IntType(8), Str.get_abi_size(llvm.create_target_data("")))
        Ret.set_body(ir.IntType(8),ret_array)
        RetOk = ir.global_context.get_identified_type("Ret::Ok")
        RetOk.set_body(ir.IntType(8))
        RetErr = ir.global_context.get_identified_type("Ret::Err")
        RetErr.set_body(ir.IntType(8), Str)
        self.structs = {
            "Str":{"ir":Str, "args": {"len": {"type": ir.IntType(64), "index": 1}}},
            "Ret":{"ir":Ret, "args": {"tag": {"type": ir.IntType(8), "index": 0}, "raw_data": {"type": ret_array, "index": 1}}, "enum_names":["Ok","Err"]},
            "Ret::Ok":{"ir":RetOk, "args": {"tag": {"type": ir.IntType(8), "index": 0}}},
            "Ret::Err":{"ir":RetErr, "args": {"tag": {"type": ir.IntType(8), "index": 0}, "raw_data": {"type": Str, "index": 1}}},
        }
        self.builder = None
        self.func = None
        self.class_ = None
        self.fblock = None
        self.context = None
        voidptr_ty = ir.IntType(8).as_pointer()
        nst_ty = ir.FunctionType(Str, [voidptr_ty, ir.IntType(64)])
        self.new_str = ir.Function(self.module, nst_ty, 'new_str')
        self.new_str.linkage = 'external'
        print_ty = ir.FunctionType(ir.VoidType(), [Str])
        print_ = ir.Function(self.module, print_ty, 'print')
        print_.linkage = 'external'
        print_i_ty = ir.FunctionType(ir.VoidType(), [ir.IntType(64)])
        self.print_i = ir.Function(self.module, print_i_ty, 'print_i')
        self.print_i.linkage = 'external'
        print_bool_ty = ir.FunctionType(ir.VoidType(), [ir.IntType(1)])
        self.print_bool = ir.Function(self.module, print_bool_ty, 'print_bool')
        self.print_bool.linkage = 'external'
        print_i128_ty = ir.FunctionType(ir.VoidType(), [ir.IntType(128)])
        self.print_i128 = ir.Function(self.module, print_i128_ty, 'print_i128')
        self.print_i128.linkage = 'external'
        print_f_ty = ir.FunctionType(ir.VoidType(), [ir.DoubleType()])
        self.print_f = ir.Function(self.module, print_f_ty, 'print_f')
        self.print_f.linkage = 'external'
        println_ty = ir.FunctionType(ir.VoidType(), [Str])
        println_ = ir.Function(self.module, println_ty, 'println')
        println_.linkage = 'external'
        println_i_ty = ir.FunctionType(ir.VoidType(), [ir.IntType(64)])
        self.println_i = ir.Function(self.module, println_i_ty, 'println_i')
        self.println_i.linkage = 'external'
        println_bool_ty = ir.FunctionType(ir.VoidType(), [ir.IntType(1)])
        self.println_bool = ir.Function(self.module, println_bool_ty, 'println_bool')
        self.println_bool.linkage = 'external'
        println_i128_ty = ir.FunctionType(ir.VoidType(), [ir.IntType(128)])
        self.println_i128 = ir.Function(self.module, println_i128_ty, 'println_i128')
        self.println_i128.linkage = 'external'
        println_f_ty = ir.FunctionType(ir.VoidType(), [ir.DoubleType()])
        self.println_f = ir.Function(self.module, println_f_ty, 'println_f')
        self.println_f.linkage = 'external'
        self.str_rev_ty = ir.FunctionType(Str.as_pointer(), [Str])
        self.str_rev = ir.Function(self.module, self.str_rev_ty, 'vrka_Str_reverse')
        self.str_rev.linkage = 'external' 
        self.vars = {
            "print": {"scope": ir.global_context, "type": print_ty, "value": print_, "const": True},
            "println": {"scope": ir.global_context, "type": println_ty, "value": println_, "const": True},
            "Str::reverse": {"scope": ir.global_context, "type": self.str_rev_ty, "value": self.str_rev, "const": True},
        }
        self.fn_prefix = ''
        #declare a global variable called __FILE__ with the value of the file name
    def __print(self, args):
        if isinstance(args[0].type, ir.PointerType):
            return self.__print([self.builder.load(args[0])])
        if isinstance(args[0].type,ir.IntType) and id(args[0].type)==id(BoolType):
            return self.builder.call(self.print_bool,[args[0]])
        elif isinstance(args[0].type,ir.IntType) and args[0].type.width>64:
            return self.builder.call(self.print_i128,[cast(args[0], ir.IntType(128), self.builder)])
        elif isinstance(args[0].type,ir.IntType):
            return self.builder.call(self.print_i,[self.builder.zext(args[0], ir.IntType(64))])
        elif (isinstance(args[0].type,ir.FloatType) or isinstance(args[0].type,ir.DoubleType)):
            return self.builder.call(self.print_f,[self.builder.fpext(args[0], ir.DoubleType())])
        elif args[0].type!=self.structs["Str"]["ir"]:
            return self.builder.call(self.vars[f"print_{args[0].type.name}"]["value"],[args[0]])
        else:
            return self.builder.call(self.vars["print"]["value"], args)
    def __println(self, args):
        if isinstance(args[0].type, ir.PointerType):
            return self.__println([self.builder.load(args[0])])
        if isinstance(args[0].type,ir.IntType) and id(args[0].type)==id(BoolType):
            return self.builder.call(self.println_bool,[args[0]])
        elif isinstance(args[0].type,ir.IntType) and args[0].type.width>64:
            return self.builder.call(self.println_i128,[cast(args[0], ir.IntType(128), self.builder)])
        elif isinstance(args[0].type,ir.IntType):
            return self.builder.call(self.println_i,[self.builder.zext(args[0], ir.IntType(64))])
        elif (isinstance(args[0].type,ir.FloatType) or isinstance(args[0].type,ir.DoubleType)):
            return self.builder.call(self.println_f,[self.builder.fpext(args[0], ir.DoubleType())])
        elif args[0].type!=self.structs["Str"]["ir"]:
            return self.builder.call(self.vars[f"println_{args[0].type.name}"]["value"],[args[0]])
        else:
            return self.builder.call(self.vars["println"]["value"], args)
    def assign(self, left, expr):
        left = self.visit(left)
        expr = self.visit(expr)
        print("assign", left, expr)
        if isinstance(left, tuple):
            if left[1]:
                if left[0] in self.vars:
                    raise ValueError(f"Variable {left[0]} is constant")
                self.vars[left[0]] = {"scope": self.func, "type": expr.type, "value": expr, "const": True}
                return
            ptr = self.builder.alloca(expr.type, name=left[0])
            self.vars[left[0]] = {"scope": self.func, "type": expr.type, "value": ptr, "const": False}
            left = ptr
        return self.builder.store(expr, left)
    def name_dotted(self, *names):
        n = '.'.join(map(lambda x:str(x.children[0]), names))
        return n
    def name(self, n):
        return str(n)
    def type_name_dotted(self, *names):
        n = '.'.join(map(lambda x:str(x.children[0]), names))
        return n
    def type_name(self, n):
        return str(n)
    def generic_type(self, proper, typelist):
        #TODO: use typelist
        return self.visit(proper)
    def proper_type(self, t):
        typ = self.visit(t)
        print("typ", typ)
        if re.match(self.INTREGEX,typ):
            return ir.IntType(int(typ[1:]))
        elif re.match(self.UINTREGEX, typ):
            return UIntType(int(typ[1:]))
        elif typ=="float":
            return ir.FloatType()
        elif typ=="double":
            return ir.DoubleType()
        elif typ=="void":
            return ir.VoidType()
        elif typ=="bool":
            return BoolType
        try:
            return self.structs[typ]["ir"]
        except KeyError:
            return typ
    def shift_expr(self, *operands):
        ret = self.visit(operands[-1])
        #right to left
        for i in range(len(operands)-2,-1,-2):
            op = operands[i]
            val = self.visit(operands[i-1])
            print(ret, op, val)
            ret, val = cast_binop(ret, val, self.builder)
            if op==">>":
                ret = self.builder.ashr(val, ret)
            elif op=="<<":
                ret = self.builder.shl(val, ret)
        return ret
    def ref_type(self, t):
        k=self.visit(t).as_pointer()
        print("ref", k)
        return k
    def lambdef(self, params, ret_type, fnblock):
        return self.funcdef(f"anon_{uuid.uuid4()}", params, ret_type, fnblock)
    def funcdef(self, name, params, ret_type, fnblock: Tree):
        name = self.fn_prefix+self.visit(name) if not isinstance(name, str) else name
        if name=="main": name="vrmain"
        params = self.visit(params) if params is not None else {}
        ret = self.visit(ret_type) if ret_type is not None else ir.VoidType()
        print("func", name, params, ret)
        func_type = ir.FunctionType(ret, params.values())
        func = ir.Function(self.module, func_type, name)
        if name!="vrmain":
            func.linkage = "internal"
        self.vars[name] = {"scope": ir.global_context, "type": func_type, "value": func, "const": True}
        for arg, name_ in zip(func.args, params.keys()):
            arg.name = name_
            self.vars[name_] = {"scope": func, "type": arg.type, "value": arg, "const": True}
        block = func.append_basic_block('entry')
        self.builder = ir.IRBuilder(block)
        self.func = func
        self.fblock = block
        self.visit(fnblock)
        if ret_type is None:
            if fnblock.children[-1].data != "return_stmt":
                self.builder.ret_void()
        for k,v in filter(lambda x: x[1]["scope"]==func, self.vars.copy().items()):
            print("removing", k, v)
            self.vars.pop(k)
        self.func = None
        self.fblock = None
        self.builder = None
        return func

    def parameters(self, typed, optional):
        optionals = self.visit(optional) if optional is not None else {}
        out = {}
        for param in typed.children:
            name = self.visit(param.children[0])
            typ = self.visit(param.children[1])
            out[name] = typ
        out|=(optionals)
        return out
    def var(self, v):
        va = self.visit(v)
        print("var", va)
        try:
            var = self.vars[va]
        except KeyError:
            return self.structs[va]["ir"]
        if not var["const"]:
            return self.builder.load(var["value"])
        return var["value"]
    def funccall(self, expr, args):
        expr: Function = self.visit(expr)
        args = self.visit(args)
        print(expr, args)
        if expr == self.vars["print"]["value"]:
            res = self.__print(args)
            if res is not None: 
                return res
        if expr == self.vars["println"]["value"]:
            res = self.__println(args)
            if res is not None:
                return res
        return self.builder.call(expr, list(map(lambda kv: cast(kv[1], expr.function_type.args[kv[0]], self.builder), enumerate(args))))
    def string(self, s):
        k = self.visit(s)
        return self.__string(k)
    def __string(self, s):
        k = s+"\0"
        ln = len(k)
        c = ir.Constant(ir.ArrayType(ir.IntType(8), ln), bytearray(k.encode('utf8')))
        cptr = self.builder.alloca(c.type)
        self.builder.store(c, cptr)
        k = self.builder.call(self.new_str, [self.builder.bitcast(cptr,ir.IntType(8).as_pointer()), ir.Constant(ir.IntType(64),ln)])
        print("string", k, k.type)
        return k
    def return_stmt(self, lis):
        if lis is None:
            return self.builder.ret_void()
        l = self.visit(lis)
        return self.builder.ret(l)
    def number(self, n, typ):
        num = str(n)
        typ = self.visit(typ) if typ is not None else ir.IntType(32)
        if num.startswith('0x'):
            return ir.Constant(typ, int(num[2:], 16))
        elif num.startswith('0b'):
            return ir.Constant(typ, int(num[2:], 2))
        elif num.startswith('0o'):
            return ir.Constant(typ, int(num[2:], 8))
        elif '.' in num or "e" in num.lower():
            return ir.Constant(ir.FloatType() if typ==ir.IntType(32) else typ, float(num))
        elif num.startswith('0'):
            return ir.Constant(typ, int(num, 8))
        else:
            return ir.Constant(typ, int(num))
        raise ValueError(f"Invalid number: {n}")
    def getattrib(self, expr, atr):
        ex = self.visit(expr)
        attr = self.visit(atr)
        print("getattr", ex, attr)
        if isinstance(ex.type, ir.PointerType):
            ptr = ex
            if attr=="deref":
                return self.builder.load(ex)
            attrptr = self.builder.gep(ptr,[ir.Constant(ir.IntType(32),0),ir.Constant(ir.IntType(32), self.structs[ex.type.name]["args"][attr]["index"])])
            print(attrptr.type)
        # else:
        #     self.builder.store(ex,ptr)
        return self.builder.extract_value(ex, self.structs[ex.type.name]["args"][attr]["index"])
    def fn_type(self, params, ret):
        pars = self.visit(params) if params is not None else []
        ret = self.visit(ret) if ret is not None else ir.VoidType()
        return ir.FunctionType(ret, pars)
    def optional_type(self, type):
        print("optional", type)
        raise NotImplementedError()
    def getsubscrip(self, expr, sub):
        ex = self.visit(expr)
        sb = self.visit(sub)
        print("subscript", ex, sb)
        #TODO: check if ex is a pointer
        itemptr = self.builder.gep(ex,[sb])
        return self.builder.load(itemptr)
    def assign_left_normal(self, mut, name):
        name = name.children[0].value
        if name in self.vars:
            if self.vars[name]["const"]:
                raise ValueError(f"Variable {name} is constant")
            return self.vars[name]["value"]
        return name, mut is None
    def comparison(self, *operands: Tree):
        ret = self.visit(operands[0])
        for i in range(1,len(operands),2):
            op = operands[i].children[0].value
            val = self.visit(operands[i+1])
            print(ret, op, val)
            ret, val = cast_binop(ret, val, self.builder)
            if op=="<>": op="!="
            if ret.type==ir.FloatType() or ret.type==ir.DoubleType():
                ret = self.builder.fcmp_ordered(op, ret, val)
            else:
                ret = self.builder.icmp_signed(op, ret, val)
        return ret
    def ternary(self, iftrue, cond, iffalse):
        cond = self.visit(cond)
        print("ternary", cond, iftrue, iffalse)
        with self.builder.if_else(cond) as (then, else_):
            with then:
                bb_then = self.builder.basic_block
                iftrue = self.visit(iftrue)
            with else_:
                bb_else = self.builder.basic_block
                iffalse = self.visit(iffalse)
        iftrue, iffalse = cast_binop(iftrue, iffalse, self.builder)
        retval = self.builder.phi(iftrue.type)
        retval.add_incoming(iftrue, bb_then)
        retval.add_incoming(iffalse, bb_else)
        return retval
    def if_stmt(self, cond, block, elifs, else_):
        cond = self.visit(cond)
        print("if", cond)
        if else_ is None:
            with self.builder.if_then(cond) as then:
                self.visit(block)
            return
        with self.builder.if_else(cond) as (then, else_bb):
            with then:
                self.visit(block)
            with else_bb:
                if len(elifs.children)!=0:
                    el0 = elifs.children[0]
                    self.if_stmt(el0.children[0], el0.children[1], Tree("elifs",elifs.children[1:]), else_)
                else:
                    self.visit(else_)
    def assign_left_subscript(self, expr, sub):
        ex = self.visit(expr)
        sb = self.visit(sub)
        return self.builder.gep(ex,[sb])
    def while_stmt(self, name, cond, block): #, else_):
        print("while", name, cond)
        cond_ = self.visit(cond)
        with self.builder.if_then(cond_):
            self.visit(block)
            bloc = self.builder.basic_block
            cond_ = self.visit(cond)
            # if else_ is not None:
            #     with self.builder.if_else(cond) as (then, else_bb):
            #         with then:
            #             self.builder.branch(bloc)
            #         with else_bb:
            #             self.visit(else_)
            #     return
            with self.builder.if_then(cond_):
                self.builder.branch(bloc)
    def arith_expr(self, *operands):
        ret = self.visit(operands[0])
        for i in range(1,len(operands),2):
            op = operands[i]
            val = self.visit(operands[i+1])
            print(ret, op, val)
            ret, val = cast_binop(ret, val, self.builder)
            if op=="+":
                if ret.type==ir.FloatType() or ret.type==ir.DoubleType():
                    ret = self.builder.fadd(ret, val)
                else:
                    ret = self.builder.add(ret, val)
            elif op=="-":
                if ret.type==ir.FloatType() or ret.type==ir.DoubleType():
                    ret = self.builder.fsub(ret, val)
                else:
                    ret = self.builder.sub(ret, val)
        return ret
    def term(self, *terms):
        ret = self.visit(terms[0])
        for i in range(1,len(terms),2):
            op = terms[i]
            val = self.visit(terms[i+1])
            print(ret, op, val)
            ret, val = cast_binop(ret, val, self.builder)
            if op=="*":
                if ret.type==ir.FloatType() or ret.type==ir.DoubleType():
                    ret = self.builder.fmul(ret, val)
                else:
                    ret = self.builder.mul(ret, val)
            elif op=="/":
                if ret.type==ir.FloatType() or ret.type==ir.DoubleType():
                    ret = self.builder.fdiv(ret, val)
                else:
                    ret = self.builder.sdiv(ret, val)
            elif op=="%":
                if ret.type==ir.FloatType() or ret.type==ir.DoubleType():
                    ret = self.builder.frem(ret, val)
                else:
                    ret = self.builder.srem(ret, val)
        return ret
    def unary(self, op, expr):
        ex = self.visit(expr)
        if op=="-":
            if op.type==ir.FloatType() or op.type==ir.DoubleType():
                return self.builder.fneg(ex)
            return self.builder.neg(ex)
        elif op=="~":
            return self.builder.not_(ex)
        elif op=="&":
            ptr = self.builder.alloca(ex.type)
            self.builder.store(ex,ptr)
            return ptr
        elif op=="*":
            return self.builder.load(ex)
        return ex
    def classdef(self, name, type_list_generic, params, funcs):
        name = self.visit(name)
        params = self.visit(params) if params is not None else {}
        if type_list_generic is None:
            classvars = {}
            vartypes = []
            for i, (k,v) in enumerate(params.items()):
                vartypes.append(v)
                classvars[k] = {"type": v, "index": i}
            class_ = ir.global_context.get_identified_type(name)
            class_.set_body(*vartypes)
            self.structs[name] = {"ir": class_, "args": classvars}
            self.fn_prefix += f"{name}::"
            self.class_, temp = class_, self.class_
            self.visit(funcs)
            self.fn_prefix = self.fn_prefix[:-len(name)-2]
            self.class_ = temp
            return class_
    def decorator(self,name, params):
        name = self.visit(name)
        params = self.visit(params)
        print("decorator", name, params)
        if name=="derive" and params==["debug"]:
            return "debug"
    def decorated(self, decs ,class_or_fn):
        decs = self.visit(decs)
        print("decorated", decs, class_or_fn)
        if decs == ["debug"]:
            class_ :ir.IdentifiedStructType= self.visit(class_or_fn)
            print("building debug for", class_.name)
            fn_ty = ir.FunctionType(ir.VoidType(), [class_])
            fn = ir.Function(self.module, fn_ty, f"print_{class_.name}")
            fn.linkage = "internal"
            bloc = fn.append_basic_block("entry")
            builder = ir.IRBuilder(bloc)
            temp, self.builder = self.builder, builder
            self.__print([self.__string(f"{class_.name}{{")])
            cvars:dict = self.structs[class_.name]["args"]
            if len(cvars)>=1:
                cvars_items = list(cvars.items()) 
                cvar, cvar_dat = cvars_items[0]
                print("cvar", cvar, cvar_dat)
                self.__print([self.__string(cvar+":")])
                ptr = self.builder.alloca(class_)
                self.builder.store(fn.args[0], ptr)
                self.__print([self.builder.load(self.builder.gep(ptr, [ir.Constant(ir.IntType(32),0),ir.Constant(ir.IntType(32),cvar_dat["index"])]))])
                for cvar, cvar_dat in cvars_items[1:]:
                    print("cvar", cvar, cvar_dat)
                    self.__print([self.__string(", "+cvar+":")])
                    self.__print([self.builder.load(self.builder.gep(ptr, [ir.Constant(ir.IntType(32),0),ir.Constant(ir.IntType(32),cvar_dat["index"])]))])
            self.__print([self.__string("}")])
            self.builder.ret_void()
            self.builder = temp
            self.vars[f"print_{class_.name}"] = {"scope": ir.global_context, "type": fn_ty, "value": fn, "const": True}
            fn_ty = ir.FunctionType(ir.VoidType(), [class_])
            fn = ir.Function(self.module, fn_ty, f"println_{class_.name}")
            fn.linkage = "internal"
            bloc = fn.append_basic_block("entry")
            builder = ir.IRBuilder(bloc)
            temp, self.builder = self.builder, builder
            self.__print([self.__string(f"{class_.name}{{")])
            cvars:dict = self.structs[class_.name]["args"]
            if len(cvars)>=1:
                cvars_items = list(cvars.items()) 
                cvar, cvar_dat = cvars_items[0]
                print("cvar", cvar, cvar_dat)
                self.__print([self.__string(cvar+":")])
                ptr = self.builder.alloca(class_)
                self.builder.store(fn.args[0], ptr)
                self.__print([self.builder.load(self.builder.gep(ptr, [ir.Constant(ir.IntType(32),0),ir.Constant(ir.IntType(32),cvar_dat["index"])]))])
                for cvar, cvar_dat in cvars_items[1:]:
                    print("cvar", cvar, cvar_dat)
                    self.__print([self.__string(", "+cvar+":")])
                    self.__print([self.builder.load(self.builder.gep(ptr, [ir.Constant(ir.IntType(32),0),ir.Constant(ir.IntType(32),cvar_dat["index"])]))])
            self.__println([self.__string("}")])
            self.builder.ret_void()
            self.builder = temp
            self.vars[f"println_{class_.name}"] = {"scope": ir.global_context, "type": fn_ty, "value": fn, "const": True}
        return decs
    def insexpr(self, type, args):
        typ = self.visit(type)
        args = self.visit(args)
        print("insexpr", typ, args)
        ptr = self.builder.alloca(typ)
        for i, arg in enumerate(args):
            self.builder.store(arg, self.builder.gep(ptr, [ir.Constant(ir.IntType(32),0),ir.Constant(ir.IntType(32),i)]))
        return self.builder.load(ptr)
    def band_expr(self, *operands):
        ret = self.visit(operands[0])
        for i in range(1,len(operands)):
            val = self.visit(operands[i])
            print(ret, val)
            ret, val = cast_binop(ret, val, self.builder)
            ret = self.builder.and_(ret, val)
        return ret
    def bor_expr(self, *operands):
        ret = self.visit(operands[0])
        for i in range(1,len(operands)):
            val = self.visit(operands[i])
            print(ret, val)
            ret, val = cast_binop(ret, val, self.builder)
            ret = self.builder.or_(ret, val)
        return ret
    def bxor_expr(self, *operands):
        ret = self.visit(operands[0])
        for i in range(1,len(operands)):
            val = self.visit(operands[i])
            print(ret, val)
            ret, val = cast_binop(ret, val, self.builder)
            ret = self.builder.xor(ret, val)
        return ret
    def not_expr(self, expr):
        ex = self.visit(expr)
        return self.builder.not_(ex)
    def land_expr(self, *operands):
        ret = self.visit(operands[0])
        print("land", operands)
        for i in range(1,len(operands)):
            val = self.visit(operands[i])
            print(ret, val)
            ret, val = cast_binop(ret, val, self.builder)
            ret = self.builder.and_(ret, val)
        return ret
    def lor_expr(self, *operands):
        ret = self.visit(operands[0])
        for i in range(1,len(operands)):
            val = self.visit(operands[i])
            print(ret, val)
            ret, val = cast_binop(ret, val, self.builder)
            ret = self.builder.or_(ret, val)
        return ret
    def line_string(self, *strings):
        _dbl_q_start, *strs, _dbl_q_end = strings
        strs = list(map(lambda x: self.visit(x), strs))
        print("line_string", strs)
        return "".join(strs)
    def string_char_simple(self, s):
        return str(s)
    def string_char_escaped(self, s):
        return eval(f'"{s}"')
    string_char_x = string_char_escaped
    string_char_utf8 = string_char_escaped
    string_char_utf16 = string_char_escaped
    def methodcall(self, expr, method_name, args):
        expr = self.visit(expr)
        method_name = self.visit(method_name)
        args = [expr,*(self.visit(args) if args is not None else [])]
        print("methodcall", method_name, args)
        fn = self.vars[f"{expr.type.name}::{method_name}"]["value"]
        return self.builder.call(fn,  list(map(lambda kv: cast(kv[1], fn.function_type.args[kv[0]], self.builder), enumerate(args))))
    def self_type(self):
        print("self", self.class_)
        return self.class_
    def enumdef(self, name, _types, members):
        name = self.visit(name)
        print("enumdef", name, members)
        self.class_, temp = ir.global_context.get_identified_type(name), self.class_
        l = self.visit(members)
        lnames, ls = zip(*l)
        ls = filter(lambda x: x is not None, ls)
        max_size = max(map(lambda x: sum(map(lambda y:y.get_abi_size(llvm.create_target_data("")),x)), ls))
        print("max_size", max_size)
        array_ty = ir.ArrayType(ir.IntType(8), max_size)
        self.class_.set_body(ir.IntType(8), array_ty)
        self.structs[name] = {"ir": self.class_, "args": {"tag": {"type": ir.IntType(8), "index": 0}, "raw_data": {"type": array_ty, "index": 1}}, "enum_names": lnames}
        self.class_ = temp
        return name
    def normal_enum_mem(self, name):
        name = self.visit(name)
        print("enum_mem", name)
        class_ = ir.global_context.get_identified_type(self.class_.name+"::"+name)
        class_.set_body(ir.IntType(8))
        self.structs[self.class_.name+"::"+name] = {"ir": class_, "args": {"tag": {"type": ir.IntType(8), "index": 0}}}
        return (name, None)
    def enum_mem_with_params(self, name, type_list):
        name = self.visit(name)
        types = self.visit(type_list)
        print("enum_mem_with_params", name, types)
        class_ = ir.global_context.get_identified_type(self.class_.name+"::"+name)
        class_.set_body(ir.IntType(8), *types)
        self.structs[self.class_.name+"::"+name] = {"ir": class_, "args": {"tag": {"type": ir.IntType(8), "index": 0}}}
        return (name, types)
    
    def enum_insexpr(self, expr, name, args):
        expr: ir.Type = self.visit(expr)
        name = self.visit(name)
        args = self.visit(args) if args is not None else []
        print("enum_insexpr", expr, name, args)
        variant_type = self.structs[expr.name+"::"+name]["ir"]
        varptr = self.builder.alloca(variant_type)
        self.builder.store(ir.Constant(ir.IntType(8), self.structs[expr.name]["enum_names"].index(name)), self.builder.gep(varptr, [ir.Constant(ir.IntType(32),0),ir.Constant(ir.IntType(32),0)]))
        for i,arg in enumerate(args):
            self.builder.store(arg, self.builder.gep(varptr, [ir.Constant(ir.IntType(32),0),ir.Constant(ir.IntType(32),i+1)]))
        retptr = self.builder.bitcast(varptr, expr.as_pointer())
        return self.builder.load(retptr)
def build(args):
    file = args.file
    filename = file.name if file.name!="<stdin>" else "a.out.vrka"
    filename_path = pathlib.Path(filename)
    filename_out = filename_path.stem
    llvm.initialize()
    llvm.initialize_native_target()
    llvm.initialize_native_asmprinter()
    parser = Lark(open('vrka.lark').read(), parser='lalr', start='file_input', debug=True)
    #print(parser.terminals)
    tree = parser.parse(file.read())
    file.close()
    print(tree.pretty())
    vis = FileParser(filename)
    vis.visit(tree)
    #print(vis.structs)
    mod = llvm.parse_assembly(str(vis.module))
    with llvm.create_module_pass_manager() as pm:
        with llvm.create_pass_manager_builder() as pmb:
            pmb.populate(pm)
        pm.run_with_remarks(mod)
    with llvm.create_function_pass_manager(mod) as fpm:
        with llvm.create_pass_manager_builder() as pmb:
            pmb.populate(fpm)
        for f in mod.functions:
            fpm.run_with_remarks(f)
    print(mod)
    tm = llvm.Target.from_default_triple().create_target_machine()
    vis.module.triple = tm.triple
    
    if args.emit:
        if "llvm" in args.emit:
            with open(filename_path.with_suffix(".ll"), 'w') as f:
                f.write(str(mod))
        if "assembly" in args.emit:
            with open(filename_path.with_suffix(".s"), "w") as f:
                f.write(tm.emit_assembly(mod))
        if "tree" in args.emit:
            pydot__tree_to_png(tree, str(filename_path.with_name("tree.png")))
    with llvm.create_mcjit_compiler(mod, tm) as ee:
        ee.finalize_object()
        with open(f'{filename_out}.o', 'wb') as f:
            f.write(tm.emit_object(mod))
        os.system(f'clang {filename_out}.o -L"stdlib/target/release/" -lstdlib -lm -g -s -o {filename_out}')
    return filename_out
def run(args):
    binary = build(args)
    os.chmod(binary, 0o755)
    os.system(f"./{binary}")
    os.remove(binary)
    os.remove(binary+".o")
def parse_only(args):
    file = args.file
    parser = Lark(open('vrka.lark').read(), parser='lalr', start=args.start_rule, debug=True)
    tree = parser.parse(file.read())
    file.close()
    print(tree.pretty())
    #vst=FileParser("test")
    #vst.visit(tree)
if __name__=="__main__": 
    VERSION = "version 0.0.5"
    parser = argparse.ArgumentParser("vrkac",description="vrka compiler", epilog=VERSION)
    subparsers_building = parser.add_subparsers()
    build_parser = subparsers_building.add_parser("build", aliases=["b"], epilog=VERSION)
    build_parser.add_argument("file", type=argparse.FileType())
    build_parser.add_argument("--emit", action="append", choices=["llvm", "llvm_bc", "assembly", "tree"])
    build_parser.set_defaults(func=build)
    run_parser = subparsers_building.add_parser("run", aliases=["r"], epilog=VERSION)
    run_parser.add_argument("file", type=argparse.FileType())
    run_parser.set_defaults(func=run, emit=None)
    parse_parser = subparsers_building.add_parser("parse", aliases=["p"], epilog=VERSION)
    parse_parser.add_argument("file", type=argparse.FileType())
    parse_parser.add_argument("-s","--start-rule", default="file_input", choices=["file_input","type","line_string"])
    parse_parser.set_defaults(func=parse_only)
    args = parser.parse_args()
    if "func" in args:
        args.func(args)
    else:
        print("interactive")
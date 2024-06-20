from lark import Lark, Token, ast_utils, Tree, v_args, logger
from lark.visitors import Interpreter, CollapseAmbiguities
from lark.tree import Meta
from .v_ast import *
from .utils import Result
from .v_types import VrType, UnitType
from .scopes import ScopeManager
from .ops import add_and_mul, shift, unary_op, bitwise_op, comparison, logical_op
from .codegen import Codegen, TypeGen
from pprint import pprint
from functools import wraps
import re,colorama, sys, logging
from llvmlite import ir, binding as llvm
import ctypes
colorama.init()
logger.setLevel(logging.WARN)

def visit_children_decor(func):
    @wraps(func)
    def inner(cls, meta, *args: Token| Tree):
        args = [cls.visit(arg) if isinstance(arg, Tree) else None if arg is None else (getattr(cls, arg.type) if arg.type in dir(cls.__class__) else lambda x:x)(arg) for arg in args] #type: ignore
        return func(cls, meta, *args)
    return inner
@v_args(inline=True, meta=True)
class AstBuilder(Interpreter):
    INTREGEX = r'i\d+'
    UINTREGEX = r'u\d+'
    RAW_STRING = r'r(#*)("+)(.*)\2\1'

    def __init__(self, source:str) -> None:
        super().__init__()
        self.source = source
        self.structs = {}
        self.scope = ScopeManager()
        self.scope.add_var("print", VrType.Function(VrType.FuncOpts({"s":VrType.String()}, VrType.Void())), Function("print", {"s":VrType.String()}, {}, [], VrType.Void()), True)

    def error(self, token: Token|Meta, msg: str):
        line_no = token.end_line or 0
        start_pos = token.start_pos or 0
        end_pos = token.end_pos or (start_pos+1)
        lines = source.splitlines()
        source_begin, line, source_end = "\n".join(lines[(line_no-2 if line_no>=2 else 0):line_no-1]), lines[line_no-1], "\n".join(lines[line_no:line_no+2])
        print(source_begin) if source_begin!='' else ''
        print(line[:start_pos]+colorama.Fore.LIGHTRED_EX+line[start_pos:end_pos]+colorama.Fore.RESET+line[end_pos:])
        print(start_pos*' '+colorama.Fore.LIGHTRED_EX+(end_pos-start_pos)*'^'+colorama.Fore.RESET)
        print(source_end) if source_end!='' else ''
        print(msg)
        exit(1)
    @visit_children_decor
    def string_concat(self, _meta:Meta, *strings: Expr.Constant) -> Expr:
        string = strings[0]
        for st in strings[1:]:
            string.constant.value += st.constant.value
        return string
    @visit_children_decor
    def string(self,_meta:Meta, string:str) -> Expr:
        return Expr.Constant(VrType.String(),Constant(string, VrType.String()))
    @visit_children_decor
    def line_string(self, _meta:Meta, *chars: str):
        _dbl_qo_start, *chars, _dbl_qo_end = chars #type: ignore
        return "".join(chars)
    @visit_children_decor
    def string_char_simple(self, _meta:Meta, char: Token):
        return str(char)
    @visit_children_decor
    def string_char_escaped(self, _meta:Meta, char: Token) -> str: #type: ignore
        try: 
            return eval(f'"{char}"')
        except (UnicodeError, SyntaxError) as e:
            self.error(char, "invalid unicode byte")
    string_char_utf8 = string_char_escaped
    string_char_utf16 = string_char_escaped
    string_char_x = string_char_escaped
    @visit_children_decor
    def number(self, _meta: Meta,num:int|float|complex, typ: VrType|None) -> Expr: #type: ignore
        if isinstance(num, float):
            typ = typ or VrType.Float()
            return Expr.Constant(typ, Constant(num, typ))
        elif isinstance(num, int):
            typ = typ or VrType.Integer(32)
            return Expr.Constant(typ, Constant(num, typ))
    #@visit_children_decor
    def FLOAT_NUMBER(self, num:Token):
        try:
            return float(num)
        except ValueError:
            self.error(num,"invalid float literal")
        
    #@visit_children_decor
    def DEC_NUMBER(self, num:Token):
        try:
            return int(num)
        except ValueError:
            self.error(num, "invalid integer literal")
 
    #@visit_children_decor
    def BIN_NUMBER(self, num:Token):
        try:
            return int(num[2:],2)
        except ValueError:
            self.error(num, "invalid binary literal")

    #@visit_children_decor
    def OCT_NUMBER(self, num:Token):
        try:
            return int(num[2:],8)
        except ValueError:
            self.error(num,"invalid octal literal")
 
    #@visit_children_decor
    def HEX_NUMBER(self, num: Token):
        try:
            return int(num[2:],16)
        except ValueError:
            self.error(num, "invalid hexadecimal literal")

    @visit_children_decor
    def name(self, _meta:Meta, n:Token):
        return n

    @visit_children_decor
    def string_name(self, _meta: Meta, _at:Token, name:str):
        return name
   
    @visit_children_decor
    def type_name(self, _meta:Meta, name:Token):
        if re.fullmatch(self.INTREGEX,name):
            return VrType.Integer(int(name[1:]))
        elif re.fullmatch(self.UINTREGEX,name):
            return VrType.UnsignedInt(int(name[1:]))
        elif name == 'float':
            return VrType.Float()
        elif name == 'double':
            return VrType.Double()
        elif name == 'void':
            return VrType.Void()
        elif name == 'bool':
            return VrType.Bool()
        elif name == 'Str':
            return VrType.String()
        else:
            try:
                return self.structs[name]
            except KeyError:
                self.error(name, f"undefined type {name}")
    
    @visit_children_decor
    def int_type_name(self, _meta: Meta, name:Token):
        return VrType.Integer(int(name[1:]))
    @visit_children_decor
    def uint_type_name(self, _meta: Meta, name:Token):
        return VrType.UnsignedInt(int(name[1:]))
    @visit_children_decor
    def float_type_name(self, _meta: Meta, name:Token):
        return VrType.Float()
    @visit_children_decor
    def double_type_name(self, _meta: Meta, name:Token):
        return VrType.Double()
    @visit_children_decor
    def void_type_name(self, _meta: Meta, name:Token):
        return VrType.Void()
    @visit_children_decor
    def bool_type_name(self, _meta: Meta, name:Token):
        return VrType.Bool()

    @visit_children_decor
    def type_name_dotted(self, _meta:Meta, *names:Token):
        if len(names) == 1:
            return names[0]
    @visit_children_decor
    def generic_type(self, _meta:Meta, name:VrType, *args:VrType): #type: ignore
        return name
    @visit_children_decor
    def ref_type(self, _meta:Meta, name:VrType): #type: ignore
        return VrType.Pointer(name)
    @visit_children_decor
    def optional_type(self, _meta:Meta, name:VrType): #type: ignore
        return VrType.Optional(name)
    @visit_children_decor
    def list_type(self, _meta:Meta, name:VrType): #type: ignore
        return VrType.List(name)
    @visit_children_decor
    def fn_type(self, _meta:Meta, params: List[VrType], ret_type):
        params = [] if params is None else params
        ret_type = VrType.Void() if ret_type is None else ret_type
        return VrType.Function(VrType.FuncOpts(params, ret_type))
    @visit_children_decor
    def term(self, _meta: Meta, factor:Expr, *factors_ops: Expr|BinOp): #type:ignore
        for i in range(1,len(factors_ops),2):
            try:
                factor = add_and_mul(factor, str(factors_ops[i-1]), factors_ops[i])
            except ValueError as e:
                self.error(factors_ops[i-1], e.args[0] or "")
        return factor
    @visit_children_decor
    def arith_expr(self, _meta: Meta, term: Expr, *term_ops: Expr|BinOp): #type:ignore
        for i in range(1,len(term_ops),2):
            try:
                term = add_and_mul(term, str(term_ops[i-1]), term_ops[i])
            except ValueError as e:
                self.error(term_ops[i-1], e.args[0] or "")
        return term
    @visit_children_decor
    def shift_expr(self, _meta: Meta, *arith_ops: Expr|BinOp): #type:ignore
        # right to left
        arith = arith_ops[-1]
        for i in range(len(arith_ops)-2,-1,-2):
            try:
                arith = shift(arith_ops[i-1], arith_ops[i], arith)
            except ValueError as e:
                self.error(arith_ops[i-1], e.args[0] or "")
        return arith
    @visit_children_decor
    def const_true(self, _meta: Meta):
        return Expr.Constant(VrType.Bool(), Constant(True, VrType.Bool()))
    @visit_children_decor
    def const_false(self, _meta: Meta):
        return Expr.Constant(VrType.Bool(), Constant(False, VrType.Bool()))
    @visit_children_decor
    def await_expr(self, _meta: Meta,  _await: Token, exp:Expr):
        return Expr.UnaryExp(exp.return_type, exp, "await")
    @visit_children_decor
    def unary(self, _meta: Meta, op:Token, exp:Expr):
        try:
            return unary_op(str(op), exp)
        except ValueError as e:
            self.error(op, e.args[0] or "")
    @visit_children_decor
    def band_expr(self, _meta: Meta, *shift_ops: Expr|BinOp): #type:ignore
        shift = shift_ops[0]
        for i in range(1,len(shift_ops)):
            try:
                shift = bitwise_op(shift, "&", shift_ops[i])
            except ValueError as e:
                self.error(_meta, e.args[0] or "")
        return shift
    @visit_children_decor
    def bxor_expr(self, _meta: Meta, *band_ops: Expr|BinOp): #type:ignore
        band = band_ops[0]
        for i in range(1,len(band_ops)):
            try:
                band = bitwise_op(band, "^", band_ops[i])
            except ValueError as e:
                self.error(_meta, e.args[0] or "")
        return band
    @visit_children_decor
    def bor_expr(self, _meta: Meta, *bxor_ops: Expr|BinOp): #type:ignore
        bxor = bxor_ops[0]
        for i in range(1,len(bxor_ops)):
            try:
                bxor = bitwise_op(bxor, "|", bxor_ops[i])
            except ValueError as e:
                self.error(_meta, e.args[0] or "")
        return bxor
    @visit_children_decor
    def comparison(self, _meta:Meta, *bor_ops: Expr|BinOp): #type:ignore
        #print(bor_ops)
        if len(bor_ops) == 1:
            return bor_ops[0]
        comp = comparison(bor_ops[0], str(bor_ops[1]), bor_ops[2])
        prev = bor_ops[2]
        for i in range(3,len(bor_ops),2):
            try:
                comp = logical_op(comp, 'and', comparison(prev, str(bor_ops[i]), bor_ops[i+1]))
                prev = bor_ops[i+1]
            except ValueError as e:
                self.error(bor_ops[i], e.args[0] or "")
        return comp
    @visit_children_decor
    def not_expr(self, _meta:Meta, exp:Expr):
        return unary_op("not", exp)
    @visit_children_decor
    def land_expr(self, _meta:Meta, *comp_ops: Expr|BinOp): #type:ignore
        comp = comp_ops[0]
        for i in range(1,len(comp_ops)):
            try:
                comp = logical_op(comp, 'and', comp_ops[i])
            except ValueError as e:
                self.error(_meta, e.args[0] or "")
        return comp
    @visit_children_decor
    def lor_expr(self, _meta:Meta, *land_ops: Expr|BinOp): #type:ignore
        land = land_ops[0]
        for i in range(1,len(land_ops)):
            try:
                land = logical_op(land, 'or', land_ops[i])
            except ValueError as e:
                self.error(_meta, e.args[0] or "")
        return land
    @visit_children_decor
    def ternary(self, _meta:Meta, lhs:Expr, cond:Expr, rhs:Expr):
        if lhs.return_type != rhs.return_type:
            self.error(_meta, f"mismatched types {lhs.return_type} and {rhs.return_type}")
        return Expr.IfExpr(lhs.return_type, cond, lhs, rhs)
    @visit_children_decor
    def COMMENT(self, token:Token):
        print(token)
    @visit_children_decor
    def raw_string(self, _meta: Meta, string: Token):
        matc = re.match(self.RAW_STRING, string, re.I|re.S)
        if matc is None:
            self.error(string, "invalid raw string")
        st = matc.group(3) #type: ignore
        return Expr.Constant(VrType.String(),Constant(st, VrType.String()))
    @visit_children_decor
    def if_stmt(self, _meta: Meta, cond: Expr, then, elifs, else_):
        if len(elifs) == 0:
            return Stmt.IfElse(cond, then, else_)
        elif_, *elifs = elifs
        else_ = self.if_stmt(_meta, elif_[0], elif_[1], elifs, else_)
        return Stmt.IfElse(cond, then, else_)
    @visit_children_decor
    def elif_(self, _meta: Meta, cond: Expr, then):
        return (cond, then)
    @visit_children_decor
    def elifs(self, _meta: Meta, *elifs):
        return elifs
    @visit_children_decor
    def return_stmt(self, _meta: Meta, _return: Token, expr: Expr):
        return Stmt.Return(expr)

    def funcdef(self, _meta: Meta, name, params, ret_type, body):#name: str, params, ret_type, body):
        #TODO: use optionalparams
        name = str(self.visit(name))
        params, optionals = self.visit(params) if params is not None else ({}, {})
        ret_type = self.visit(ret_type) if ret_type is not None else VrType.Void()
        self.scope.push_scope(Context.FunctionLocal(name)) #type: ignore
        for i,(k,v) in enumerate(params.items()):
            self.scope.add_var(k,v,Expr.FuncArg(v,i),True)
        body = self.visit(body)
        # pprint(name)
        # pprint(params)
        # pprint(ret_type)
        # pprint(body)
        vars = self.scope.pop_scope()
        fn = Function(name, params, optionals, body, ret_type, vars)
        self.scope.add_var(name, VrType.Function(VrType.FuncOpts(params, ret_type)), fn, True)
        return fn

    @visit_children_decor
    def block(self, _meta: Meta, *stmts_expr):
        *stmts, expr = stmts_expr
        if expr is not None:
            stmts.append(Stmt.Return(expr))
        return stmts
    @visit_children_decor
    def typedparam(self, _meta: Meta, name, type):
        return (str(name), type)
    @visit_children_decor
    def optionalparam(self, _meta: Meta, name, type):
        return (str(name), type)
    @visit_children_decor
    def typedparams(self, _meta:Meta, *params):
        return [*params]
    @visit_children_decor
    def optionalparams(self, _meta:Meta, *params):
        return [*params]
    @visit_children_decor
    def parameters(self, _meta: Meta, *params):
        optionals = []
        if len(params)==2 and type(params[0])==list:
            params, optionals = params
        else:
            params, optionals = None, params[0]
        if optionals is None: optionals = []
        if params is None: params = []
        return {k:v for k,v in params}, {k:v for k,v in optionals}
    @visit_children_decor
    def var(self, _meta: Meta, name):
        sv = self.scope.get_var(name)
        if sv is None:
            self.error(name, f"undefined variable {name}")
        if sv.const: #type: ignore
            return sv.value #type: ignore
        return Expr.UnaryExp(sv.type.to, sv.value, "*") #type: ignore
    @visit_children_decor
    def bind_name(self, _meta: Meta, mut, name):
        return (name, mut is None)
    @visit_children_decor
    def char_type_name(self, _meta: Meta, name:Token):
        return VrType.Char()
    @visit_children_decor
    def unit_type_name(self, _meta: Meta, name:Token):
        return VrType.Unit()
    @visit_children_decor
    def const_unit(self, _meta: Meta):
        return Expr.Constant(VrType.Unit(), Constant(None, VrType.Unit()))
    @visit_children_decor
    def assign_left_normal(self, _meta: Meta, expr):
        sv = self.scope.get_var(str(expr[0]))
        if sv is None:
            return (str(expr[0]), expr[1])
        if sv.const:
            self.error(expr[0], f"{expr[0]} is constant")
        return sv.value
    @visit_children_decor
    def assign(self, _meta: Meta, left, expr):
        if isinstance(left, tuple):
            var = left[0]
            if not left[1]:
                t = expr.return_type
                expr, tmp = Expr.Alloc(VrType.Pointer(t), t, var), expr
                self.scope.add_var(var, expr.return_type, expr, left[1])
                return Stmt.Assign(expr, tmp)
            else:
                self.scope.add_var(var, expr.return_type, expr, left[1])
                #return Stmt.Assign(var, expr)
        else:
            if left.type != expr.return_type:
                self.error(_meta, f"mismatched types {left.type} and {expr.return_type}")
            return Stmt.Assign(left, expr)
    def block_expr(self, _meta: Meta, *stmts_expr):
        *stmts, expr = stmts_expr
        print("block_expr", stmts, expr)
        self.scope.push_scope(Context.BlockLocal()) #type: ignore
        stmts = [self.visit(stmt) for stmt in stmts]
        expr = self.visit(expr)
        self.scope.pop_scope()
        return Expr.Block(expr.return_type, stmts+[Stmt.ExprStmt(expr)])
    @visit_children_decor
    def funccall(self, _meta: Meta, fn: Function, params):
        fn_params = fn.params
        if len(params) != len(fn_params):
            self.error(_meta, f"expected {len(fn_params)} arguments, got {len(params)}")
        for i,(p,fp) in enumerate(zip(params, fn_params.values())):
            if p.return_type != fp:
                self.error(_meta, f"mismatched types {p.return_type} and {fp}")
        return Expr.Call(fn.return_type ,fn, params, {})
    @visit_children_decor
    def expr_stmt(self, _meta: Meta, expr: Expr):
        return Stmt.ExprStmt(expr)
    @visit_children_decor
    def list_expr(self, _meta: Meta, exprs: List[Expr]):
        expr_return_type = exprs[0].return_type
        for expr in exprs[1:]:
            if expr.return_type != expr_return_type:
                self.error(_meta, f"mismatched types {expr.return_type} and {expr_return_type}")
        return Expr.Constant(VrType.List(expr_return_type), Constant(exprs, VrType.List(expr_return_type)))
    @visit_children_decor
    def char(self, _meta: Meta, char: Token):
        print(char)
        return Expr.Constant(VrType.Char(), Constant(char[0], VrType.Char()))
class c_unit_type(ctypes.Structure):
    __fields__ = []
if __name__ == '__main__':
    source = sys.stdin.read()
    ab=AstBuilder(source)
    parser = Lark.open('vrka.lark', parser='earley', start='file_input', lexer="dynamic", debug=True, propagate_positions=True)
    tree = parser.parse(source)
    print(tree.pretty())
    mir = ab.visit(tree)
    pprint(mir)
    module = ir.Module("<main>")
    fn: Function
    for fn in mir:
        typegen = TypeGen(module)
        params = [typegen.visit(arg) for arg in fn.params.values()]
        ret_type = typegen.visit(fn.return_type)
        if isinstance(ret_type, ir.BaseStructType):
            params.append(ir.PointerType(ret_type))
            ret_type = ir.VoidType()
        # if isinstance(fn.return_type, VrType.List):
        #     params.append(ir.PointerType(typegen.visit(fn.return_type.of)))
        function = ir.Function(module, ir.FunctionType(ret_type, [arg for arg in params], False), fn.name)
        codegen = Codegen(module, function, fn.return_type, vars=fn.vars)
        codegen.visit(fn.body)
        print(function)
    #pprint(ab.scope)
    mod = llvm.parse_assembly(str(module))
    mod.verify()
    pmb = llvm.create_pass_manager_builder()
    pm = llvm.create_module_pass_manager()
    fpm = llvm.create_function_pass_manager(mod)
    pmb.opt_level = 3
    pmb.loop_vectorize = True
    pmb.slp_vectorize = True
    pmb.populate(pm)
    pm.add_constant_merge_pass()
    pm.add_dead_arg_elimination_pass()
    pm.add_cfg_simplification_pass()
    pm.run(mod)
    with open("main.ll","w") as f:
        print(mod, file=f)
    print("Verified module")
    llvm.initialize()
    llvm.initialize_native_target()
    llvm.initialize_native_asmprinter()
    target = llvm.Target.from_default_triple()
    target_machine = target.create_target_machine()
    with llvm.create_mcjit_compiler(mod, target_machine) as ee:
        print("assembly")
        print(target_machine.emit_assembly(mod))
        ee.finalize_object()
        print("Running code")
        func_ptr = ee.get_function_address("test")
        cfunc = ctypes.CFUNCTYPE(ctypes.c_int32)(func_ptr)
        ptr = cfunc()
        print(ptr)
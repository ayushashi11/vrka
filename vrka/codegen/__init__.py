from ..v_types import VrType, Unit
from ..v_ast import Expr
from .typegen import TypeGen
from ..scopes import ScopeVar
from llvmlite import ir, binding as llvm
from werkzeug.utils import secure_filename
class Codegen:
    def __init__(self, module: ir.Module, function: ir.Function, ret_type: VrType, target_data = None, vars: dict[str, ScopeVar] = {}):
        self.module = module
        self.function = function
        self.block_ = function.append_basic_block('entry')
        self.builder = ir.IRBuilder(self.block_)
        self.ret_type = ret_type
        self.strings = {}
        self.malloc_ty = ir.FunctionType(ir.PointerType(ir.IntType(8)), [ir.IntType(64)], False)
        try:
            self.malloc = next(filter(lambda x: x.name=="malloc", module.functions))
        except StopIteration:
            self.malloc=ir.Function(self.module, self.malloc_ty, 'malloc')
        self.target_data = target_data or llvm.create_target_data('e-m:e-i64:64-f80:128-n8:16:32:64-S128')
        self.vars = {}
        for name, scopevar in vars.items():
            print(name, scopevar)
            if isinstance(scopevar.value, Expr.Constant):
                continue
            self.vars[name] = self.builder.alloca(TypeGen(module).visit(scopevar.value.type), name=name)
    def visit(self, node):
        node_type = node.__class__.__name__.lower()
        *node_class, sub_node_type = node_type.split('.')
        print(node_class, sub_node_type)
        if sub_node_type == 'return':
            sub_node_type = 'return_'
        return getattr(self, sub_node_type)(node)
    def return_(self, node):
        if not self.ret_type.check_eq(node.exp.return_type) and not (node.exp.return_type == VrType.String() and self.ret_type == VrType.List(VrType.UnsignedInt(8))):
            raise ValueError(f"Return type mismatch: {self.ret_type} != {node.exp.return_type}")
        value = self.visit(node.exp)
        if isinstance(self.ret_type, VrType.Void) or isinstance(self.ret_type, VrType.Unit) or isinstance(self.ret_type, VrType.String):
            self.builder.store(value, self.function.args[-1])
            return self.builder.ret_void()
        return self.builder.ret(value)
    def constant(self, node):
        ret_typ = node.return_type
        print(ret_typ)
        match ret_typ:
            case VrType.Integer(ln):
                return ir.Constant(ir.IntType(ln), node.constant.value)
            case VrType.UnsignedInt(len_):
                return ir.Constant(ir.IntType(len_), node.constant.value)
            case VrType.Float():
                return ir.Constant(ir.FloatType(), node.constant.value)
            case VrType.Double():
                return ir.Constant(ir.DoubleType(), node.constant.value)
            case VrType.String():
                arr=ir.Constant(ir.ArrayType(ir.IntType(8), len(node.constant.value) + 1), bytearray(node.constant.value.encode()+b'\0'))
                global_str = ir.GlobalVariable(self.module, arr.type, name=secure_filename(f"str{node.constant.value}"))
                global_str.linkage = 'internal'
                global_str.global_constant = True
                global_str.initializer = arr     #type: ignore
                self.strings[node.constant.value] = global_str
                return self.builder.bitcast(global_str, ir.PointerType(ir.IntType(8)))
            case VrType.Bool():
                return ir.Constant(ir.IntType(1), node.constant.value)
            case VrType.Void():
                return None
            case VrType.Unit():
                return Unit
            case VrType.Char():
                return ir.Constant(ir.IntType(8), ord(node.constant.value))
            case VrType.List():
                list_type: ir.Type = TypeGen(self.module).visit(node.constant.value[0].return_type)

                exprs = [self.visit(constant) for constant in node.constant.value]
                ptr = self.builder.call(self.malloc, [ir.Constant(ir.IntType(64), len(exprs) * list_type.get_abi_size(self.target_data))])
                ptr = self.builder.bitcast(ptr, ir.PointerType(list_type))
                for i, expr in enumerate(exprs):
                    self.builder.store(expr, self.builder.gep(ptr, [ir.Constant(ir.IntType(64), i)]))
                return ptr
            case _:
                raise ValueError(f"Unknown constant type {ret_typ}")
    def list(self, nodes):
        for node in nodes:
            print("list", node)
            self.visit(node)
    def nonetype(self, node):
        pass
    def cast(self, node):
        value = self.visit(node.exp)
        match (node.exp.return_type, node.return_type):
            case (VrType.Integer(ln1), VrType.Integer(ln2)) if ln1 < ln2:
                return self.builder.sext(value, ir.IntType(ln2))
            case (VrType.Integer(ln1), VrType.Integer(ln2)) if ln1 > ln2:
                return self.builder.trunc(value, ir.IntType(ln2))
            case (VrType.Integer(ln1), VrType.UnsignedInt(ln2)) if ln1 < ln2:
                return self.builder.sext(value, ir.IntType(ln2))
            case (VrType.Integer(ln1), VrType.UnsignedInt(ln2)) if ln1 > ln2:
                return self.builder.trunc(value, ir.IntType(ln2))
            case (VrType.UnsignedInt(ln1), VrType.Integer(ln2)) if ln1 < ln2:
                return self.builder.zext(value, ir.IntType(ln2))
            case (VrType.UnsignedInt(ln1), VrType.Integer(ln2)) if ln1 > ln2:
                return self.builder.trunc(value, ir.IntType(ln2))
            case (VrType.UnsignedInt(ln1), VrType.UnsignedInt(ln2)) if ln1 < ln2:
                return self.builder.zext(value, ir.IntType(ln2))
            case (VrType.UnsignedInt(ln1), VrType.UnsignedInt(ln2)) if ln1 > ln2:
                return self.builder.trunc(value, ir.IntType(ln2))
            case (VrType.Float(), VrType.Double()):
                return self.builder.fpext(value, ir.DoubleType())
            case (VrType.Double(), VrType.Float()):
                return self.builder.fptrunc(value, ir.FloatType())
            case (VrType.Integer(_), VrType.Float()):
                return self.builder.sitofp(value, ir.FloatType())
            case (VrType.Integer(_), VrType.Double()):
                return self.builder.sitofp(value, ir.DoubleType())
            case (VrType.UnsignedInt(_), VrType.Float()):
                return self.builder.uitofp(value, ir.FloatType())
            case (VrType.UnsignedInt(_), VrType.Double()):
                return self.builder.uitofp(value, ir.DoubleType())
            case (VrType.Float(), VrType.Integer(_)):
                return self.builder.fptosi(value, ir.IntType(ln2))
            case (VrType.Double(), VrType.Integer(_)):
                return self.builder.fptosi(value, ir.IntType(ln2))
            case (VrType.Float(), VrType.UnsignedInt(_)):
                return self.builder.fptoui(value, ir.IntType(ln2))
            case (VrType.Double(), VrType.UnsignedInt(_)):
                return self.builder.fptoui(value, ir.IntType(ln2))
            case (x,y) if x.check_eq(y):
                return value
            case _:
                raise ValueError(f"Unknown cast {node.exp.return_type} to {node.return_type}")
    def binexp(self, node):
        print(node.lhs, node.op, node.rhs)
        lhs = self.visit(node.lhs)
        rhs = self.visit(node.rhs)
        match (node.lhs.return_type, node.op):
            case (VrType.Integer(ln)|VrType.UnsignedInt(ln), '+'):
                return self.builder.add(lhs, rhs)
            case (VrType.Integer(ln)|VrType.UnsignedInt(ln), '-'):
                return self.builder.sub(lhs, rhs)
            case (VrType.Integer(ln)|VrType.UnsignedInt(ln), '*'):
                return self.builder.mul(lhs, rhs)
            case (VrType.Integer(ln), '/'):
                return self.builder.sdiv(lhs, rhs)
            case (VrType.Integer(ln), '%'):
                return self.builder.srem(lhs, rhs)
            case (VrType.UnsignedInt(ln), '/'):
                return self.builder.udiv(lhs, rhs)
            case (VrType.UnsignedInt(ln), '%'):
                return self.builder.urem(lhs, rhs)
            case (VrType.Float()|VrType.Double(), '+'):
                return self.builder.fadd(lhs, rhs)
            case (VrType.Float()|VrType.Double(), '-'):
                return self.builder.fsub(lhs, rhs)
            case (VrType.Float()|VrType.Double(), '*'):
                return self.builder.fmul(lhs, rhs)
            case (VrType.Float()|VrType.Double(), '/'):
                return self.builder.fdiv(lhs, rhs)
            case (VrType.Float()|VrType.Double(), '%'):
                return self.builder.frem(lhs, rhs)
            case (VrType.Integer(ln)|VrType.UnsignedInt(ln), '<<'):
                return self.builder.shl(lhs, rhs)
            case (VrType.Integer(ln), '>>'):
                return self.builder.ashr(lhs, rhs)
            case (VrType.Integer(ln)|VrType.UnsignedInt(ln), '&'):
                return self.builder.and_(lhs, rhs)
            case (VrType.Integer(ln)|VrType.UnsignedInt(ln), '|'):
                return self.builder.or_(lhs, rhs)
            case (VrType.Integer(ln)|VrType.UnsignedInt(ln), '^'):
                return self.builder.xor(lhs, rhs)
            case (VrType.UnsignedInt(ln), '>>'):
                return self.builder.lshr(lhs, rhs)
            case (VrType.Float()|VrType.Double(), op):
                return self.builder.fcmp_unordered(op, lhs, rhs)
            case (VrType.Float()|VrType.Double(), '<>'):
                return self.builder.fcmp_unordered('!=', lhs, rhs)
            case (VrType.Integer(ln), op):
                return self.builder.icmp_signed(op, lhs, rhs)
            case (VrType.Integer(ln), '<>'):
                return self.builder.icmp_signed('!=', lhs, rhs)
            case (VrType.UnsignedInt(ln), op):
                return self.builder.icmp_unsigned(op, lhs, rhs)
            case (VrType.UnsignedInt(ln), '<>'):
                return self.builder.icmp_unsigned('!=', lhs, rhs)
            case (VrType.Bool(), 'and'):
                return self.builder.and_(lhs, rhs)
            case (VrType.Bool(), 'or'):
                return self.builder.or_(lhs, rhs)
    
    def assign(self, node):
        value = self.visit(node.right)
        self.builder.store(value, self.vars[node.left.name])
    
    def unaryexp(self, node):
        value = self.visit(node.exp)
        match (node.op, node.exp.return_type):
            case ('-', VrType.Integer(ln)|VrType.UnsignedInt(ln)):
                return self.builder.neg(value)
            case ('-', VrType.Float()|VrType.Double()):
                return self.builder.fneg(value)
            case ('~', VrType.Integer(ln)|VrType.UnsignedInt(ln)):
                return self.builder.not_(value)
            case ('not', VrType.Bool()):
                return self.builder.not_(value)
            case ('&', _):
                return self.builder.gep(value, [ir.Constant(ir.IntType(64), 0)])
            case ('*', VrType.Pointer(_)):
                return self.builder.load(value)
    
    def alloc(self, node):
        return self.vars[node.name]
            
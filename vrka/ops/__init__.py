from ..v_ast import Expr
from ..v_types import VrType
from .addition import add_and_mul

def shift(lhs: Expr, op: str, rhs: Expr) -> Expr.BinExp:
    match (lhs.return_type, rhs.return_type):
        case (VrType.Integer(a), VrType.Integer(b)) if a>b: #and isinstance(rhs, Expr.Constant):
            return Expr.BinExp(lhs.return_type, lhs, str(op), Expr.Cast(lhs.return_type, rhs))
        case (VrType.Integer(a), VrType.Integer(b)) if a<b: #and isinstance(lhs, Expr.Constant):
            return Expr.BinExp(rhs.return_type, Expr.Cast(rhs.return_type, lhs), str(op), rhs)
        case (VrType.Integer(a), VrType.Integer(b)):
            return Expr.BinExp(lhs.return_type, lhs, str(op), rhs)
        case (a,b):
            raise ValueError(f"unsupported types for shift operations: {a} and {b}")

def unary_op(op: str, exp: Expr) -> Expr.UnaryExp:
    match (op, exp.return_type):
        case ('-', VrType.Integer(_)|VrType.UnsignedInt(_)|VrType.Float()|VrType.Double()):
            return Expr.UnaryExp(exp.return_type, exp, '-')
        case ('~', VrType.Integer(_)|VrType.UnsignedInt(_)):
            return Expr.UnaryExp(exp.return_type, exp, '~')
        case ('not', VrType.Bool()):
            return Expr.UnaryExp(exp.return_type, exp, 'not')
        case ('&', _):
            return Expr.UnaryExp(VrType.Pointer(exp.return_type), exp, '&')
        case ('*', VrType.Pointer(_)):
            return Expr.UnaryExp(exp.return_type.to, exp, '*')
        case (a,b):
            raise ValueError(f"unsupported unary operation {a} for {b}")

def bitwise_op(lhs: Expr, op: str, rhs: Expr) -> Expr.BinExp:
    match (lhs.return_type, rhs.return_type):
        case (VrType.Integer(_), VrType.Integer(_)):
            return Expr.BinExp(lhs.return_type, lhs, op, rhs)
        case (VrType.UnsignedInt(_), VrType.UnsignedInt(_)):
            return Expr.BinExp(lhs.return_type, lhs, op, rhs)
        case (a,b):
            raise ValueError(f"unsupported types for bitwise {op}: {a} and {b}")


def comparison(lhs: Expr, op: str, rhs: Expr) -> Expr.BinExp:
    match (lhs.return_type, rhs.return_type):
        case (VrType.Integer(a), VrType.Integer(b)) if a>b: #and isinstance(rhs, Expr.Constant):
            return Expr.BinExp(VrType.Bool(), lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (VrType.Integer(a), VrType.Integer(b)) if a<b: #and isinstance(lhs, Expr.Constant):
            return Expr.BinExp(VrType.Bool(), Expr.Cast(rhs.return_type, lhs), op, rhs)
        case (VrType.Integer(a), VrType.Integer(b)):
            return Expr.BinExp(VrType.Bool(), lhs, op, rhs)
        case (VrType.Float(), VrType.Double()):
            return Expr.BinExp(VrType.Bool(), Expr.Cast(rhs.return_type,lhs), op, rhs)
        case (VrType.Double(), VrType.Float()):
            return Expr.BinExp(VrType.Bool(), lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (VrType.Float(), VrType.Float()):
            return Expr.BinExp(VrType.Bool(), lhs, op, rhs)
        case (VrType.Double(), VrType.Double()):
            return Expr.BinExp(VrType.Bool(), lhs, op, rhs)
        case (VrType.Integer(_), VrType.Float()):
            return Expr.BinExp(VrType.Bool(), Expr.Cast(rhs.return_type,lhs), op, rhs)
        case (VrType.Float(), VrType.Integer(_)):
            return Expr.BinExp(VrType.Bool(), lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (VrType.Integer(_), VrType.Double()):
            return Expr.BinExp(VrType.Bool(), Expr.Cast(rhs.return_type,lhs), op, rhs)
        case (VrType.Double(), VrType.Integer(_)):
            return Expr.BinExp(VrType.Bool(), lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (VrType.UnsignedInt(a), VrType.UnsignedInt(b)) if a>b: #and isinstance(rhs, Expr.Constant):
            return Expr.BinExp(VrType.Bool(), lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (VrType.UnsignedInt(a), VrType.UnsignedInt(b)) if a<b: #and isinstance(lhs, Expr.Constant):
            return Expr.BinExp(VrType.Bool(), Expr.Cast(rhs.return_type, lhs), op, rhs)
        case (VrType.UnsignedInt(a), VrType.UnsignedInt(b)):
            return Expr.BinExp(VrType.Bool(), lhs, op, rhs)
        case (VrType.UnsignedInt(a), VrType.Integer(b)) if a>=b: #and isinstance(rhs, Expr.Constant):
            return Expr.BinExp(VrType.Bool(), lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (VrType.UnsignedInt(a), VrType.Integer(b)) if a<b: #and isinstance(lhs, Expr.Constant):
            return Expr.BinExp(VrType.Bool(), Expr.Cast(VrType.UnsignedInt(b), lhs), op, Expr.Cast(VrType.UnsignedInt(b), rhs))
        case (VrType.Integer(a), VrType.UnsignedInt(b)) if a>=b: #and isinstance(rhs, Expr.Constant):
            return Expr.BinExp(VrType.Bool(), Expr.Cast(rhs.return_type, lhs), op, rhs)
        case (VrType.Integer(a), VrType.UnsignedInt(b)) if a<b: #and isinstance(lhs, Expr.Constant):
            return Expr.BinExp(VrType.Bool(), Expr.Cast(VrType.UnsignedInt(b), lhs), op, Expr.Cast(VrType.UnsignedInt(b), rhs))
        case (a,b):
            raise ValueError(f"unsupported types for comparison: {a} and {b}")


def logical_op(lhs: Expr, op: str, rhs: Expr) -> Expr.BinExp:
    match (lhs.return_type, rhs.return_type):
        case (VrType.Bool(), VrType.Bool()):
            return Expr.BinExp(lhs.return_type, lhs, op, rhs)
        case (VrType.Optional(x),y) if x==y and op=="or":
            return Expr.BinExp(lhs.return_type, lhs, "if_null", rhs)
        case (x, VrType.Optional(y)) if x==y and op=="or":
            return Expr.BinExp(rhs.return_type, rhs, "if_null", lhs)
        case (a,b):
            raise ValueError(f"unsupported types for logical operations: {a} and {b}")

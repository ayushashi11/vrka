from ..v_types import VrType
from ..v_ast import Expr

def add_and_mul(lhs: Expr, op: str, rhs: Expr) -> Expr.BinExp:
    match (lhs.return_type, rhs.return_type):
        case (VrType.Integer(a), VrType.Integer(b)) if a>b: #and isinstance(rhs, Expr.Constant):
            return Expr.BinExp(lhs.return_type, lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (VrType.Integer(a), VrType.Integer(b)) if a<b: #and isinstance(lhs, Expr.Constant):
            return Expr.BinExp(rhs.return_type, Expr.Cast(rhs.return_type, lhs), op, rhs)
        case (VrType.Integer(a), VrType.Integer(b)):
            return Expr.BinExp(lhs.return_type, lhs, op, rhs)
        case (VrType.UnsignedInt(a), VrType.UnsignedInt(b)) if a>b: #and isinstance(rhs, Expr.Constant):
            return Expr.BinExp(lhs.return_type, lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (VrType.UnsignedInt(a), VrType.UnsignedInt(b)) if a<b: #and isinstance(lhs, Expr.Constant):
            return Expr.BinExp(rhs.return_type, Expr.Cast(rhs.return_type, lhs), op, rhs)
        case (VrType.UnsignedInt(a), VrType.UnsignedInt(b)):
            return Expr.BinExp(lhs.return_type, lhs, op, rhs)
        case (VrType.Float(), VrType.Double()):
            return Expr.BinExp(rhs.return_type, Expr.Cast(rhs.return_type,lhs), op, rhs)
        case (VrType.Double(), VrType.Float()):
            return Expr.BinExp(lhs.return_type, lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (VrType.Float(), VrType.Float()):
            return Expr.BinExp(lhs.return_type, lhs, op, rhs)
        case (VrType.Double(), VrType.Double()):
            return Expr.BinExp(lhs.return_type, lhs, op, rhs)
        case (VrType.Integer(_)|VrType.UnsignedInt(_), VrType.Float()):
            return Expr.BinExp(rhs.return_type, Expr.Cast(rhs.return_type,lhs), op, rhs)
        case (VrType.Float(), VrType.Integer(_)|VrType.UnsignedInt(_)):
            return Expr.BinExp(lhs.return_type, lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (VrType.Integer(_)|VrType.UnsignedInt(_), VrType.Double()):
            return Expr.BinExp(rhs.return_type, Expr.Cast(rhs.return_type,lhs), op, rhs)
        case (VrType.Double(), VrType.Integer(_)|VrType.UnsignedInt(_)):
            return Expr.BinExp(lhs.return_type, lhs, op, Expr.Cast(lhs.return_type, rhs))
        case (a,b):
            raise ValueError(f"unsupported types for arithmetic: {a} and {b}")

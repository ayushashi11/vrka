from typing import List, Optional, Self, Literal, Dict, Tuple
from alg_types import alg, variant
from dataclasses import dataclass
from .utils import Result
from .v_types import VrType

@alg
class Context:
    pass
@dataclass
class Constant:
    value: int|float|str|None|object
    type: VrType
MathOp = Literal['+']|Literal['-']|Literal['*']|Literal['/']|Literal['%']
BitOp = Literal['<<']|Literal['>>']|Literal['&']|Literal['|']|Literal['^']
LogOp = Literal['and']|Literal['or']
CompOp = Literal['==']|Literal['!=']|Literal['<']|Literal['>']|Literal['<=']|Literal['>=']
BinOp = MathOp|BitOp|LogOp|CompOp
UnaryOp = Literal['-']|Literal['~']|Literal['not']|Literal['&']|Literal['*']|Literal["await"]

@alg
@dataclass
class Expr:
    return_type: VrType
    @variant
    def BinExp(lhs: Self, op: BinOp, rhs: Self): ...
    @variant
    def UnaryExp(exp: Self, op: UnaryOp): ...
    @variant
    @staticmethod
    def Constant(constant: Constant): ...
    @variant
    @staticmethod
    def Var(name: str, context: Context): ...
    @variant
    def Call(func: Self, args: List[Self], optionals: Dict[str, Self]): ...
    @variant
    def IfExpr(cond: Self, lhs: Self, rhs: Self): ...
    @variant
    def MethodCall(exp: Self, method: str, args: List[Self], optionals: Dict[str, Self]): ...
    @variant
    def Instantiation(type: VrType, args: List[Self], optionals: Dict[str, Self]): ... #type:ignore
    @variant
    def GetAttr(exp: Self, attr: str): ...
    @variant
    def Subscript(exp: Self, subs:Self|Tuple[Self|None, Self|None, Self|None]): ...

@alg
class AssignLeft:
    @variant
    @staticmethod
    def Named(name:str, mut:bool): ...
    @variant
    @staticmethod
    def Subscript(exp:Expr.Subscript): ...
    @variant
    @staticmethod
    def GetAttr(exp: Expr.GetAttr): ...

@alg
class Stmt:
    @variant
    def Assign(left:AssignLeft, right:Expr): ...
    @variant
    def ExprStmt(exp: Expr): ...
    @variant
    def IfElse(if_:Expr, then: List[Self], else_: Self|List[Self]|None): ... #type:ignore
    @variant
    def While(cond: Expr, body: List[Self], name: str|None): ... #type:ignore

@dataclass
class Function:
    name: str
    params: Dict[str, VrType]
    optionals: Dict[str, Tuple[VrType, Expr]]
    body: List[Stmt]
    return_type: VrType

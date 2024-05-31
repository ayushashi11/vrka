from typing import TypeVar, Generic, Callable, Self
from alg_types import alg, variant
import llvmlite.ir as ir

def cast_binop(lhs: ir.NamedValue, rhs: ir.NamedValue, builder: ir.builder.IRBuilder) -> tuple[ir.NamedValue, ir.NamedValue]:
    #TODO: use ops as well determining the cast
    match (lhs.type, rhs.type):
        case (ir.IntType(), ir.IntType()) if lhs.type.width > rhs.type.width:
            return lhs, builder.sext(rhs, lhs.type)
        case (ir.IntType(), ir.IntType()) if lhs.type.width < rhs.type.width:
            return builder.sext(lhs, rhs.type), rhs
        case (ir.IntType(), ir.IntType()):
            return lhs, rhs
        case (ir.FloatType(), ir.IntType()):
            return lhs, builder.sitofp(rhs, lhs.type)
        case (ir.IntType(), ir.FloatType()):
            return builder.sitofp(lhs, rhs.type), rhs
        case (ir.DoubleType(), ir.IntType()):
            return lhs, builder.sitofp(rhs, lhs.type)
        case (ir.IntType(), ir.DoubleType()):
            return builder.sitofp(lhs, rhs.type), rhs
        case (ir.FloatType(), ir.DoubleType()):
            return builder.fpext(lhs, rhs.type), rhs
        case (ir.DoubleType(), ir.FloatType()):
            return lhs, builder.fpext(rhs, lhs.type)
        case (x,y) if x==y:
            return lhs, rhs
        case _:
            return -1

def cast(from_: ir.NamedValue, to: ir.Type, builder: ir.IRBuilder):
    match (from_.type, to):
        case (ir.IntType(), ir.IntType()) if from_.type.width > to.width:
            return builder.trunc(from_, to)
        case (ir.IntType(), ir.IntType()) if from_.type.width < to.width:
            return builder.sext(from_, to)
        case (ir.IntType(), ir.IntType()):
            return from_
        case (ir.FloatType(), ir.IntType()):
            return builder.fptosi(from_, to)
        case (ir.IntType(), ir.FloatType()):
            return builder.sitofp(from_, to)
        case (ir.DoubleType(), ir.IntType()):
            return builder.fptosi(from_, to)
        case (ir.IntType(), ir.DoubleType()):
            return builder.sitofp(from_, to)
        case (ir.FloatType(), ir.DoubleType()):
            return builder.fpext(from_, to)
        case (ir.DoubleType(), ir.FloatType()):
            return builder.fptrunc(from_, to)
        case (x,y) if x==y:
            return from_
        case _:
            return -1

T,U = TypeVar("T"), TypeVar("U")
@alg
class Result(Generic[T,U]):
    @variant
    def Ok(x:T): ...

    @variant
    def Err(st:U): ...
    
    def is_ok(self):
        return isinstance(self, Result.Ok)
    
    def is_err(self):
        return isinstance(self, Result.Err)
    
    def map(self, f: Callable[[T], T]) -> Self:
        match self:
            case Result.Ok(x):
                return Result.Ok(f(x))
            case Result.Err(st):
                return Result.Err(st)
    
    def map_err(self, f: Callable[[U], U]) -> Self:
        match self:
            case Result.Ok(x):
                return Result.Ok(x)
            case Result.Err(st):
                return Result.Err(f(st))
    
    def unwrap(self) -> T:
        match self:
            case Result.Ok(x):
                return x
            case Result.Err(st):
                raise Exception(st)
    
    def unwrap_err(self) -> U:
        match self:
            case Result.Ok(x):
                raise Exception(x)
            case Result.Err(st):
                return st
    
    def unwrap_or(self, default: T) -> T:
        match self:
            case Result.Ok(x):
                return x
            case Result.Err(st):
                return default
    
    def unwrap_or_else(self, f: Callable[[], T]) -> T:
        match self:
            case Result.Ok(x):
                return x
            case Result.Err(st):
                return f()
    
    def expect(self, msg: str) -> T:
        match self:
            case Result.Ok(x):
                return x
            case Result.Err(_st):
                raise Exception(msg)
    
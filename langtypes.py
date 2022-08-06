from llvmlite import ir
from typing import Union
Value = Union[ir.Argument,ir.Constant]
class Types:
    i64 = ir.IntType(64)
    i32 = ir.IntType(32)
    i16 = ir.IntType(16)
    i8 = ir.IntType(8)
    ints = [i64,i32,i16,i8]
    floatt = ir.FloatType()
    double = ir.DoubleType()
    voidptr = i8.as_pointer()
    char = ir.IntType(8)
    stringptr = char.as_pointer()
    
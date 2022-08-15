from dataclasses import dataclass
from llvmlite import ir
from typing import Any, Union
Value = Union[ir.Argument,ir.Constant]
class _int_matches_all:
    def __eq__(self, __o: object) -> bool:
        return isinstance(__o,int)
    def __format__(self, __format_spec: str) -> str:
        print(__format_spec)
        return "<any int>"
class Types:
    i64 = ir.IntType(64)
    i32 = ir.IntType(32)
    i16 = ir.IntType(16)
    i8 = ir.IntType(8)
    bool = ir.IntType(1)
    ints = [i64,i32,i16,i8]
    floatt = ir.FloatType()
    double = ir.DoubleType()
    voidptr = i8.as_pointer()
    char = ir.IntType(8)
    stringptr = char.as_pointer()
    string = ir.ArrayType(char, _int_matches_all()).as_pointer()
    strictstring = lambda length: ir.ArrayType(Types.char, length).as_pointer()
    generatestring = lambda data: ir.Constant(ir.ArrayType(Types.char, len(data.encode("utf-8"))+1), bytearray((data+"\0").encode("utf-8")))

if __name__=='__main__':
    st = Types.generatestring("lol")
    #st = Types.stringptr("lol")
    print(st)
    print(Types.bool)
    print(ir.LiteralStructType([Types.i64, ir.LiteralStructType([Types.stringptr],packed=True)]))
    print(f"{_int_matches_all():%d}")
    print(ir.Constant(ir.LiteralStructType([Types.i64, Types.stringptr]),[0,Types.generatestring("data")]))
    
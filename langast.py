from dataclasses import dataclass
from typing import Any, ClassVar, Dict, List, Literal, Optional, Union
from llvmlite import ir

@dataclass
class Type:
    internal_value: int
    def gen_ir(self) -> ir.Type: pass #type: ignore
    def __ge__(self, o) -> bool: return self.internal_value>=o.internal_value
    def __gt__(self, o) -> bool: return self.internal_value>o.internal_value
    def __le__(self, o) -> bool: return self.internal_value<=o.internal_value
    def __lt__(self, o) -> bool: return self.internal_value<o.internal_value

@dataclass
class Node:
    pass

class UndefinedError(TypeError): pass

@dataclass(init=False, eq=False)
class UndefinedForNow(Type):
    @dataclass
    class _UFNData:
        dat: Optional[Type]
        source: List[Type]
    children: ClassVar[Dict[int, _UFNData]] = dict()
    def __init__(self, source) -> None:
        source.append(self)
        UndefinedForNow.children[id(self)] = UndefinedForNow._UFNData(None, source)
    
    def __eq__(self, o: Type) -> bool:
        data = UndefinedForNow.children[id(self)]
        if data.dat is not None:
            return data.dat==o
        else:
            data.dat = o
            data.source.remove(self)
            return True

    def __getattribute__(self, name: str) -> Any:
        data = UndefinedForNow.children[id(self)]
        if data.dat is None:
            return UndefinedForNow(data.source)
        else:
            return data.dat.__getattribute__(name)
    
    def __repr__(self) -> str:
        data = UndefinedForNow.children[id(self)].dat
        if data is None: return "undefined"
        else: return repr(data)
    
    def gen_ir(self) -> ir.Type:
        raise UndefinedError("attempt to call IR on undefined")

@dataclass(kw_only=True)
class Int(Type):
    size: Literal[8]|Literal[16]|Literal[32]|Literal[64]
    internal_value: int = 1
    def gen_ir(self) -> ir.Type:
        return ir.IntType(self.size)

@dataclass
class Float(Type):
    internal_value: int = 2
    def gen_ir(self) -> ir.Type:
        return ir.FloatType()

@dataclass
class Double(Type):
    internal_value: int = 3
    def gen_ir(self) -> ir.Type:
        return ir.DoubleType()

@dataclass
class Bool(Type):
    internal_value: int = 100
    def gen_ir(self) -> ir.Type:
        return ir.IntType(1)

@dataclass
class Constant(Node):
    value: Any
    type: Type

@dataclass
class Add(Node):
    left: Node
    right: Node

@dataclass
class Sub(Node):
    left: Node
    right: Node

@dataclass
class Mul(Node):
    left: Node
    right: Node

@dataclass
class Div(Node):
    left: Node
    right: Node

@dataclass
class Mod(Node):
    left: Node
    right: Node

if __name__=="__main__":
    undefineds = []
    x=UndefinedForNow(undefineds)
    print(undefineds)
    if x==5:
        print(x)
    if x==6:
        print(x)
    else:
        print("x is set to",x)
        print(undefineds)
    print(Int(size=8)<=(Int(size=32)))

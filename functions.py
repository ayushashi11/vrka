from typing import List, Dict
from langtypes import Types, Value
from llvmlite import ir
class ScopeHandler:
    def __init__(self, func:ir.Function, scopes=[dict()]) -> None:
        self.scopes: List[Dict[str, Value]] = scopes
        self.func = func
        self.blocks = []
        self.builders = []
        self._eblox=0
        self._condblox=0
        self._anonblox=0
        self._loopblox=0
        self._i = len(scopes)
    def get(self, var: str) -> Value:
        for i in range(len(self.scopes)-1, -1, -1):
            match self.scopes[i].get(var, None):
                case None:
                    pass
                case x:
                    return x
        raise ValueError(f"undefined variable {var}")
    def set(self, var:str, value: Value):
        print(f"SET {var}={value}")
        for i in range(len(self.scopes)-1, -1, -1):
            match self.scopes[i].get(var, None):
                case None:
                    pass
                case x if x.type==value.type:
                        self.scopes[i][var] = value
                        break
                case x:
                        raise ValueError(f"cant assign value of {value.type} to variable of type {x.type}")
        else:
            self.scopes[-1][var] = value
    def add_entry_block(self) -> ir.IRBuilder:
        block = self.func.append_basic_block(f"entry{self._eblox}")
        builder = ir.IRBuilder(block)
        self.blocks.append(block)
        self.builders.append(builder)
        self._eblox+=1
        return builder
    def add_cond_block(self) -> ir.IRBuilder:
        block = self.func.append_basic_block(f"cond{self._condblox}")
        builder = ir.IRBuilder(block)
        self.blocks.append(block)
        self.builders.append(builder)
        self._condblox+=1
        return builder
    def add_anon_block(self) -> ir.IRBuilder:
        block = self.func.append_basic_block(f"anon{self._anonblox}")
        builder = ir.IRBuilder(block)
        self.blocks.append(block)
        self.builders.append(builder)
        self._anonblox+=1
        return builder
    def add_loop_block(self) -> ir.IRBuilder:
        block = self.func.append_basic_block(f"loop{self._loopblox}")
        builder = ir.IRBuilder(block)
        self.blocks.append(block)
        self.builders.append(builder)
        self._loopblox+=1
        return builder
    def pop_block(self):
        if len(self.blocks)<=self._i:raise ValueError("cant pop parent block")
        self.blocks.pop()
        self.builders.pop()
    @staticmethod
    def inherit(v, func: ir.Function):
        return ScopeHandler(func, v.scopes)

class Function:
    __slots__ = {'arg_t', 'arg_names', 'fnptr', 'type', 'scopehnd', 'name'}
    def __init__(self, module, name, arg_t, arg_names, type, parent_scopehnd=None) -> None:
        self.arg_t = arg_t
        self.type = type
        self.name = name
        self.fnptr = ir.Function(module, ir.FunctionType(type, arg_t), name=name)
        self.arg_names = arg_names
        self.scopehnd = ScopeHandler.inherit(parent_scopehnd,self.fnptr) if parent_scopehnd is not None else ScopeHandler(self.fnptr)
        for i,arg in enumerate(self.fnptr.args):
            self.scopehnd.set(arg_names[i], arg)
    @property
    def builder(self) -> ir.IRBuilder:
        return self.scopehnd.builders[-1]
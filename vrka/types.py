from alg_types import alg, variant
from typing import Self
from dataclasses import dataclass
import llvmlite.ir as ir

@alg
class VrType:
    @dataclass
    class StructOpts:
        fields: dict[str, Self]
        defaults: dict[str, Self] #TODO: change to list of Constants
        def check_eq(self, other: Self) -> bool:
            for field, ofield in zip(self.fields.items(), other.fields.items()):
                if not field[1].check_eq(ofield[1]):
                    return False
            for default, odefault in zip(self.defaults.items(), other.defaults.items()):
                if not default[1].check_eq(odefault[1]):
                    return False
            return True
    @dataclass
    class FuncOpts:
        params: dict[str, Self]
        ret: Self
        def check_eq(self, other: Self) -> bool:
            for param, oparam in zip(self.params.items(), other.params.items()):
                if not param[1].check_eq(oparam[1]):
                    return False
            return self.ret.check_eq(other.ret)
    @variant
    def Integer(v: int, len: int): ...
    @variant
    def UnsignedInt(v: int, len: int): ...
    @variant
    def Float(v: float): ...
    @variant
    def Double(v: float): ...
    @variant
    def String(v: str): ...
    @variant
    def Void(): ...
    @variant
    def Pointer(v: Self): ...
    @variant
    def Optional(v: Self): ...
    @variant
    def List(v: Self): ...
    @variant
    def Struct(opts: StructOpts): ...
    @variant
    def Function(opts: FuncOpts): ...
    @variant
    def Unknown(vars: list[object]): ...

    def check_eq(self, other: Self) -> bool:
        if self.__class__ == VrType.Unknown and other.__class__ != VrType.Unknown:
            for var in self.vars:
                var.type = other
            return True
        if self.__class__ != VrType.Unknown and other.__class__ == VrType.Unknown:
            for var in other.vars:
                var.type = self
            return True
        if self.__class__ != other.__class__:
            return False
        if self.__class__ == VrType.Integer:
            return self.len == other.len
        if self.__class__ == VrType.UnsignedInt:
            return self.len == other.len
        if self.__class__ == VrType.Unknown and other.__class__ == VrType.Unknown:
            sum_list = self.vars + other.vars
            self.vars = sum_list
            other.vars = sum_list
            return True
        if self.__class__ == VrType.Pointer and other.__class__ == VrType.Pointer:
            return self.v.check_eq(other.v)
        if self.__class__ == VrType.Optional and other.__class__ == VrType.Optional:
            return self.v.check_eq(other.v)
        if self.__class__ == VrType.Struct and other.__class__ == VrType.Struct:
            return self.opts.check_eq(other.opts)
        if self.__class__ == VrType.Function and other.__class__ == VrType.Function:
            return self.opts.check_eq(other.opts)
        if self.__class__ == VrType.List and other.__class__ == VrType.List:
            return self.v.check_eq(other.v)
        return False
    

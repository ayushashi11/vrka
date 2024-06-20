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
            for field, ofield in zip(self.fields.values(), other.fields.values()):#zip(self.fields.items(), other.fields.items()):
                if not field.check_eq(ofield):
                    return False
            for default, odefault in zip(self.defaults.values(), other.defaults.values()):#zip(self.defaults.items(), other.defaults.items()):
                if not default.check_eq(odefault):
                    return False
            return True
    @dataclass
    class FuncOpts:
        params: list[Self]
        ret: Self
        def check_eq(self, other: Self) -> bool:
            for param, oparam in zip(self.params, other.params):#zip(self.params.items(), other.params.items()):
                if not param.check_eq(oparam):
                    return False
            return self.ret.check_eq(other.ret)
    @variant
    def Integer( len: int): ...
    @variant
    def UnsignedInt(len: int): ...
    @variant
    def Char(): ...
    @variant
    def Float(): ...
    @variant
    def Double(): ...
    @variant
    def String(): ...
    @variant
    def Bool(): ...
    @variant
    def Void(): ...
    @variant
    def Unit(): ...
    @variant
    def Pointer(to: Self): ...
    @variant
    def Optional(of: Self): ...
    @variant
    def List(of: Self): ...
    @variant
    def Struct(opts: StructOpts): ...
    @variant
    def Function(opts: FuncOpts): ...
    @variant
    def Unknown(vars: list[object]): ...

    def check_eq(self, other: Self) -> bool:
        if self.__class__ == VrType.Unknown and other.__class__ != VrType.Unknown:
            for var in self.vars:
                var.return_type = other
            return True
        if self.__class__ != VrType.Unknown and other.__class__ == VrType.Unknown:
            for var in other.vars:
                var.return_type = self
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
            return self.to.check_eq(other.to)
        if self.__class__ == VrType.Optional and other.__class__ == VrType.Optional:
            return self.of.check_eq(other.of)
        if self.__class__ == VrType.Struct and other.__class__ == VrType.Struct:
            return self.opts.check_eq(other.opts)
        if self.__class__ == VrType.Function and other.__class__ == VrType.Function:
            return self.opts.check_eq(other.opts)
        if self.__class__ == VrType.List and other.__class__ == VrType.List:
            return self.of.check_eq(other.of)
        if self.__class__ == other.__class__ and self.__class__ in [VrType.Char, VrType.Float, VrType.Double, VrType.String, VrType.Bool, VrType.Void, VrType.Unit]:
            return True
        return False
    
#opaque type for Unit
UnitType = ir.LiteralStructType([], packed=True)
Unit = ir.Constant(UnitType, [])
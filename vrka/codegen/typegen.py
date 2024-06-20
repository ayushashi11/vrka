from llvmlite import ir
from ..v_types import VrType, UnitType

class TypeGen:
    def __init__(self, module: ir.Module):
        self.module = module
    def visit(self, node):
        if not isinstance(node, VrType):
            method = getattr(self, f'{node.__class__.__name__.lower()}')
            return method(node)
        node_type = node.__class__.__name__.lower().split('vrtype.')[1]
        if node_type == 'list':
            node_type = 'array'
        method = getattr(self, f'{node_type}')
        return method(node)
    def integer(self, node):
        return ir.IntType(node.len)
    def float(self, node):
        return ir.FloatType()
    def unit(self, node):
        return UnitType
    def double(self, node):
        return ir.DoubleType()
    def bool(self, node):
        return ir.IntType(1)
    def string(self, node):
        return ir.IntType(8).as_pointer()
    def array(self, node):
        return ir.PointerType(self.visit(node.of))
    def unsignedint(self, node):
        return ir.IntType(node.len)
    def char(self, node):
        return ir.IntType(8)

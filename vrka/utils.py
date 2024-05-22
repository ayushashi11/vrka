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

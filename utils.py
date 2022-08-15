from llvmlite import ir
from typing import List
from langtypes import Types, Value
_formats=dict()
def generate_format(args: List[Value], module: ir.Module, builder, sep=" ", end="\n") -> ir.GlobalVariable:
    name = "fstr"
    fstr = ""
    for i,arg in enumerate(args):
        match arg.type:
            case x if x in Types.ints:
                fstr+="%d"+sep
                name+="i"
            case Types.double:
                fstr+="%.10f"+sep
                name+="d"
            case Types.floatt:
                fstr+="%f"+sep
                name+="f"
                args[i] = builder.fpext(arg, Types.double)
            case Types.stringptr|Types.string:
                fstr+="%s"+sep
                name+="s"
            case _:
                pass
    fstr += end+"\0"
    if fstr in _formats: return _formats[fstr]
    c_fmt = ir.Constant(ir.ArrayType(Types.i8, len(fstr)),
                        bytearray(fstr.encode("utf8")))
    global_fmt = ir.GlobalVariable(module, c_fmt.type, name=name)
    global_fmt.linkage = 'internal'
    global_fmt.global_constant = True
    global_fmt.initializer = c_fmt  # type: ignore
    _formats[fstr] = global_fmt
    return global_fmt

def test_equality(lhs, rhs) -> bool:
    match lhs, rhs:
        case Types.string, Types.stringptr:
            return True
        case Types.stringptr, Types.string:
            return True
        case x,y if x in Types.ints and y in Types.ints:
            return True
        case x, (Types.floatt|Types.double) if x in Types.ints:
            return True
        case (Types.floatt|Types.double), x if x in Types.ints:
            return True
        case (Types.floatt, Types.double)|(Types.double, Types.floatt):
            return True
        case x, y:
            return x==y

def test_equalities(lhs, rhs) -> bool:
    out = len(lhs)==len(rhs)
    for x,y in zip(lhs,rhs):
        out = out and test_equality(x,y)
    return out

def cast(val: Value, type: ir.Type, builder: ir.IRBuilder) -> Value:
    match val.type, type:
        case Types.string, Types.stringptr:
            return builder.bitcast(val, type)
        case x, (Types.floatt|Types.double) if x in Types.ints:
            return builder.sitofp(val,type)
        case (Types.floatt|Types.double), x if x in Types.ints:
            return builder.fptosi(val,type)
        case Types.floatt, Types.double:
            return builder.fpext(val,type)
        case Types.double, Types.floatt:
            return builder.fptrunc(val,type)
        case x,y if x==y:
            return val
        case x,y:
            raise ValueError(f"cant convert from {x} to {y}")
def cast_list(vals:List[Value], types: List[ir.Type], builder:ir.IRBuilder):
    for i, val in enumerate(vals):
        vals[i] = cast(val, types[i], builder)
    return vals
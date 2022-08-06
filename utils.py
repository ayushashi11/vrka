from llvmlite import ir
from typing import List
from langtypes import Types, Value
_formats=dict()
def generate_format(args: List[Value], module: ir.Module, fpext) -> Types.voidptr:
    name = "fstr"
    fstr = ""
    for i,arg in enumerate(args):
        match arg.type:
            case x if x in Types.ints:
                fstr+="%d "
                name+="i"
            case Types.floatt:
                fstr+="%f "
                name+="f"
                args[i] = fpext(arg, Types.double)
            case Types.double:
                fstr+="%lf "
                name+="d"
            case Types.stringptr:
                fstr+="%s "
                name+="s"
            case _:
                pass
    fstr += "\n\0"
    if fstr in _formats: return _formats[fstr]
    c_fmt = ir.Constant(ir.ArrayType(Types.i8, len(fstr)),
                        bytearray(fstr.encode("utf8")))
    global_fmt = ir.GlobalVariable(module, c_fmt.type, name=name)
    global_fmt.linkage = 'internal'
    global_fmt.global_constant = True
    global_fmt.initializer = c_fmt  # type: ignore
    _formats[fstr] = global_fmt
    return global_fmt
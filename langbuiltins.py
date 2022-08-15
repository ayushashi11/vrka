from functions import Function
from langtypes import Types
libtable = {
    "STRUTILS": "stdlib/strutils.o",
    "MATH": ""
}
clibs = {
    "STRUTILS": [],
    "MATH": ["-lm"]
}
func_builtins = {
    "STRUTILS": [
        lambda module: Function(module, "strrev", [Types.stringptr], ["st"], Types.stringptr) ,
        lambda module: Function(module, "strrepr", [Types.stringptr], ["st"], Types.stringptr)  
    ],
    "MATH": [
        lambda module: Function(module, "pow", [Types.double, Types.double], ["n", "p"], Types.double)
    ]
}

from lark import Lark, Token, ast_utils, Tree, v_args, Transformer
from lark.tree import Meta
from .v_ast import *
from .utils import Result
from .v_types import VrType
from pprint import pprint
import re,colorama
colorama.init()

@v_args(inline=True, meta=True)
class AstBuilder(Transformer):
    INTREGEX = r'i\d+'
    UINTREGEX = r'u\d+'
    def __init__(self, source:str, visit_tokens: bool = True) -> None:
        super().__init__(visit_tokens)
        self.source = source
        self.structs = {}
    def string_concat(self, _meta:Meta, *strings: Expr.Constant) -> Expr:
        string = strings[0]
        for st in strings[1:]:
            string.constant.value += st.constant.value
        return string
    def string(self,_meta:Meta, string:str) -> Expr:
        return Expr.Constant(VrType.String(),Constant(string, VrType.String()))
    def line_string(self, _meta:Meta, *chars: str):
        _dbl_qo_start, *chars, _dbl_qo_end = chars #type: ignore
        return "".join(chars)
    def string_char_simple(self, _meta:Meta, char: Token):
        return str(char)
    def string_char_escaped(self, _meta:Meta, char: Token) -> str:
        try: 
            return eval(f'"{char}"')
        except (UnicodeError, SyntaxError) as e:
            line_no = char.end_line or 0
            start_pos = char.start_pos or 0
            end_pos = char.end_pos or (start_pos+1)
            lines = source.splitlines()
            source_begin, line, source_end = "\n".join(lines[(line_no-2 if line_no>=2 else 0):line_no-1]), lines[line_no-1], "\n".join(lines[line_no:line_no+2])
            print(source_begin) if source_begin!='' else ''
            print(line[:start_pos]+colorama.Fore.LIGHTRED_EX+line[start_pos:end_pos]+colorama.Fore.RESET+line[end_pos:])
            print(start_pos*' '+colorama.Fore.LIGHTRED_EX+(end_pos-start_pos)*'^'+colorama.Fore.RESET)
            print(source_end) if source_end!='' else ''
            print("invalid unicode byte")
            exit(1)
    string_char_utf8 = string_char_escaped
    string_char_utf16 = string_char_escaped
    string_char_x = string_char_escaped
    def number(self, _meta: Meta,num:int|float|complex, typ: VrType|None) -> Expr: #type: ignore
        if isinstance(num, float):
            typ = typ or VrType.Float()
            return Expr.Constant(typ, Constant(num, typ))
        elif isinstance(num, int):
            typ = typ or VrType.Integer(32)
            return Expr.Constant(typ, Constant(num, typ))
    def FLOAT_NUMBER(self, num:Token):
        try:
            return float(num)
        except ValueError:
            line_no = num.end_line or 0
            start_pos = num.start_pos or 0
            end_pos = num.end_pos or (start_pos+1)
            lines = source.splitlines()
            source_begin, line, source_end = "\n".join(lines[(line_no-2 if line_no>=2 else 0):line_no-1]), lines[line_no-1], "\n".join(lines[line_no:line_no+2])
            print(source_begin) if source_begin!='' else ''
            print(line[:start_pos]+colorama.Fore.LIGHTRED_EX+line[start_pos:end_pos]+colorama.Fore.RESET+line[end_pos:])
            print(start_pos*' '+colorama.Fore.LIGHTRED_EX+(end_pos-start_pos)*'^'+colorama.Fore.RESET)
            print(source_end) if source_end!='' else ''
            print("invalid float literal")
            exit(1) 
        
    def DEC_NUMBER(self, num:Token):
        try:
            return int(num)
        except ValueError:
            line_no = num.end_line or 0
            start_pos = num.start_pos or 0
            end_pos = num.end_pos or (start_pos+1)
            lines = source.splitlines()
            source_begin, line, source_end = "\n".join(lines[(line_no-2 if line_no>=2 else 0):line_no-1]), lines[line_no-1], "\n".join(lines[line_no:line_no+2])
            print(source_begin) if source_begin!='' else ''
            print(line[:start_pos]+colorama.Fore.LIGHTRED_EX+line[start_pos:end_pos]+colorama.Fore.RESET+line[end_pos:])
            print(start_pos*' '+colorama.Fore.LIGHTRED_EX+(end_pos-start_pos)*'^'+colorama.Fore.RESET)
            print(source_end) if source_end!='' else ''
            print("invalid integer literal")
            exit(1) 
    def BIN_NUMBER(self, num:Token):
        try:
            return int(num[2:],2)
        except ValueError:
            line_no = num.end_line or 0
            start_pos = num.start_pos or 0
            end_pos = num.end_pos or (start_pos+1)
            lines = source.splitlines()
            source_begin, line, source_end = "\n".join(lines[(line_no-2 if line_no>=2 else 0):line_no-1]), lines[line_no-1], "\n".join(lines[line_no:line_no+2])
            print(source_begin) if source_begin!='' else ''
            print(line[:start_pos]+colorama.Fore.LIGHTRED_EX+line[start_pos:end_pos]+colorama.Fore.RESET+line[end_pos:])
            print(start_pos*' '+colorama.Fore.LIGHTRED_EX+(end_pos-start_pos)*'^'+colorama.Fore.RESET)
            print(source_end) if source_end!='' else ''
            print("invalid binary literal")
            exit(1) 
    def OCT_NUMBER(self, num:Token):
        try:
            return int(num[2:],8)
        except ValueError:
            line_no = num.end_line or 0
            start_pos = num.start_pos or 0
            end_pos = num.end_pos or (start_pos+1)
            lines = source.splitlines()
            source_begin, line, source_end = "\n".join(lines[(line_no-2 if line_no>=2 else 0):line_no-1]), lines[line_no-1], "\n".join(lines[line_no:line_no+2])
            print(source_begin) if source_begin!='' else ''
            print(line[:start_pos]+colorama.Fore.LIGHTRED_EX+line[start_pos:end_pos]+colorama.Fore.RESET+line[end_pos:])
            print(start_pos*' '+colorama.Fore.LIGHTRED_EX+(end_pos-start_pos)*'^'+colorama.Fore.RESET)
            print(source_end) if source_end!='' else ''
            print("invalid octal literal")
            exit(1) 
    def HEX_NUMBER(self, num: Token):
        try:
            return int(num[2:],16)
        except ValueError:
            line_no = num.end_line or 0
            start_pos = num.start_pos or 0
            end_pos = num.end_pos or (start_pos+1)
            lines = source.splitlines()
            source_begin, line, source_end = "\n".join(lines[(line_no-2 if line_no>=2 else 0):line_no-1]), lines[line_no-1], "\n".join(lines[line_no:line_no+2])
            print(source_begin) if source_begin!='' else ''
            print(line[:start_pos]+colorama.Fore.LIGHTRED_EX+line[start_pos:end_pos]+colorama.Fore.RESET+line[end_pos:])
            print(start_pos*' '+colorama.Fore.LIGHTRED_EX+(end_pos-start_pos)*'^'+colorama.Fore.RESET)
            print(source_end) if source_end!='' else ''
            print("invalid octal literal")
            exit(1)
    def name(self, _meta:Meta, n:str):
        return n

    def string_name(self, _meta: Meta, _at:Token, name:str):
        return name
    
    def type_name(self, _meta:Meta, name:Token):
        if re.fullmatch(self.INTREGEX,name):
            return VrType.Integer(int(name[1:]))
        elif re.fullmatch(self.UINTREGEX,name):
            return VrType.UnsignedInt(int(name[1:]))
        elif name == 'float':
            return VrType.Float()
        elif name == 'double':
            return VrType.Double()
        elif name == 'void':
            return VrType.Void()
        elif name == 'bool':
            return VrType.Bool()
        else:
            try:
                return self.structs[name]
            except KeyError:
                line_no = name.end_line or 0
                start_pos = name.start_pos or 0
                end_pos = name.end_pos or (start_pos+1)
                lines = source.splitlines()
                source_begin, line, source_end = "\n".join(lines[(line_no-2 if line_no>=2 else 0):line_no-1]), lines[line_no-1], "\n".join(lines[line_no:line_no+2])
                print(source_begin) if source_begin!='' else ''
                print(line[:start_pos]+colorama.Fore.LIGHTRED_EX+line[start_pos:end_pos]+colorama.Fore.RESET+line[end_pos:])
                print(start_pos*' '+colorama.Fore.LIGHTRED_EX+(end_pos-start_pos)*'^'+colorama.Fore.RESET)
                print(source_end) if source_end!='' else ''
                print(f"undefined type {name}")
                exit(1)
    def type_name_dotted(self, _meta:Meta, *names:Token):
        if len(names) == 1:
            return names[0]
    def generic_type(self, _meta:Meta, name:VrType, *args:VrType): #type: ignore
        return name
    def ref_type(self, _meta:Meta, name:VrType): #type: ignore
        return VrType.Pointer(name)
    def optional_type(self, _meta:Meta, name:VrType): #type: ignore
        return VrType.Optional(name)
    def list_type(self, _meta:Meta, name:VrType): #type: ignore
        return VrType.List(name)
    def fn_type(self, _meta:Meta, params: List[VrType], ret_type):
        params = [] if params is None else params
        ret_type = VrType.Void() if ret_type is None else ret_type
        return VrType.Function(VrType.FuncOpts(params, ret_type))
    def term(self, _meta: Meta, factor:Expr, *factors_ops: Expr|BinOp):
        for i in range(1,len(factors_ops),2):
            factor = Expr.BinExp(factor, factors_ops[i-1], factors_ops[i])
        return factor

if __name__ == '__main__':
    parser = Lark.open('vrka.lark', parser='lalr', start='expr')
    source = '''1*2'''
    tree = parser.parse(source)
    ab=AstBuilder(source)
    pprint(ab.transform(tree))
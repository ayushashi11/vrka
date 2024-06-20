from .v_ast import Expr, Context
from .v_types import VrType
from dataclasses import dataclass

@dataclass
class ScopeVar:
    name: str
    type: VrType
    value: Expr | None #type:ignore
    const: bool

@dataclass
class ScopeManager:
    scopes: list[Context]
    current: Context
    scope_vars: list[dict[str, ScopeVar]]
    current_scope: dict[str, ScopeVar]

    def __init__(self):
        self.scopes = [Context.Global()]
        self.current = Context.Global()
        self.scope_vars = [{}]
        self.current_scope = self.scope_vars[0]
    
    def push_scope(self, context: Context):
        self.scopes.append(context)
        self.scope_vars.append({})
        self.current = context
        self.current_scope = self.scope_vars[-1]
    
    def pop_scope(self):
        self.scopes.pop()
        ret = self.scope_vars.pop()
        self.current = self.scopes[-1]
        self.current_scope = self.scope_vars[-1]
        return ret
    
    def add_var(self, name: str, type: VrType, value: Expr | None = None, const = False): #type:ignore
        self.current_scope[name] = ScopeVar(name, type, value, const)

    def get_var(self, name: str) -> ScopeVar | None:
        for scope in reversed(self.scope_vars):
            if scope.get(name):
                return scope[name]
        return None

    def set_var(self, name: str, value: Expr):
        for scope in reversed(self.scope_vars):
            if scope.get(name):
                scope[name].value = value
                return
        raise Exception(f"Variable {name} not found in scopes")
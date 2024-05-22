from dataclasses import dataclass
from typing import List, Optional

@dataclass
class Expr:
    def resolve(self, builder, vars):
        raise NotImplementedError

@dataclass
class Stmt:
    def resolve(self, builder, vars):
        raise NotImplementedError

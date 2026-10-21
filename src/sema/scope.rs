use super::error::SemaError;
use super::symbol::{ResolvedVar, VarKind};
use crate::syntax::token::Span;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LocalInfo {
    pub slot: usize,
    pub kind: VarKind,
    pub span: Span,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct UpvalueDesc {
    pub name: String,
    pub kind: VarKind,
    pub parent_index: usize,
    pub is_direct_local: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct FunctionScope {
    pub name: String,
    pub locals: Vec<HashMap<String, LocalInfo>>,
    pub upvalues: Vec<UpvalueDesc>,
    pub next_slot: usize,
    pub scope_depth: usize,
    pub compiled_name: String,
}

impl FunctionScope {
    pub fn new(name: String, compiled_name: String) -> Self {
        Self {
            name,
            locals: vec![HashMap::new()],
            upvalues: Vec::new(),
            next_slot: 0,
            scope_depth: 0,
            compiled_name,
        }
    }

    pub fn enter_block(&mut self) {
        self.scope_depth += 1;
        self.locals.push(HashMap::new());
    }

    pub fn exit_block(&mut self) {
        self.scope_depth -= 1;
        self.locals.pop();
    }

    pub fn define_local(&mut self, name: String, kind: VarKind, span: Span) -> Result<usize, SemaError> {
        let current_scope = self.locals.last_mut().unwrap();
        if current_scope.contains_key(&name) {
            return Err(SemaError::duplicate(name, span));
        }
        let slot = self.next_slot;
        self.next_slot += 1;
        current_scope.insert(
            name,
            LocalInfo {
                slot,
                kind,
                span,
                depth: self.scope_depth,
            },
        );
        Ok(slot)
    }

    pub fn mark_mutated(&mut self, name: &str) {
        for scope in self.locals.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                if info.kind == VarKind::Constant {
                    // Will be caught by check_assignable, but if allowed mut, update to Mutable
                } else {
                    info.kind = VarKind::Mutable;
                }
                return;
            }
        }
    }

    pub fn find_local(&self, name: &str) -> Option<LocalInfo> {
        for scope in self.locals.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info.clone());
            }
        }
        None
    }

    pub fn add_upvalue(&mut self, name: String, kind: VarKind, parent_index: usize, is_direct_local: bool, span: Span) -> usize {
        for (i, upval) in self.upvalues.iter().enumerate() {
            if upval.parent_index == parent_index && upval.is_direct_local == is_direct_local {
                return i;
            }
        }
        let index = self.upvalues.len();
        self.upvalues.push(UpvalueDesc {
            name,
            kind,
            parent_index,
            is_direct_local,
            span,
        });
        index
    }
}

pub struct ScopeManager {
    pub function_stack: Vec<FunctionScope>,
    pub globals: HashMap<String, Span>,
    pub global_compiled_names: HashMap<String, String>,
    pub namespaces: HashMap<String, Vec<String>>, // namespace name -> list of child symbols / namespaces
    pub enum_variants: HashMap<String, (String, Span)>, // variant_name -> (enum_name, span)
}

impl ScopeManager {
    pub fn new(global_scope_name: String) -> Self {
        let mut namespaces = HashMap::new();
        // Built-in / native namespaces matching IonVM stdlib / FFI
        namespaces.insert("std".to_string(), vec!["io".to_string(), "fs".to_string(), "os".to_string(), "time".to_string(), "net".to_string(), "json".to_string(), "string".to_string()]);
        namespaces.insert("std.io".to_string(), vec!["print".to_string(), "println".to_string(), "read_line".to_string(), "read_stdin".to_string()]);
        namespaces.insert("std.net".to_string(), vec!["tcp_listen".to_string(), "tcp_accept".to_string(), "tcp_recv".to_string(), "tcp_send".to_string(), "tcp_close".to_string(), "tcp_connect".to_string()]);
        namespaces.insert("std.time".to_string(), vec!["now_ms".to_string(), "sleep_ms".to_string()]);
        namespaces.insert("std.fs".to_string(), vec!["read_file".to_string(), "write_file".to_string(), "append_file".to_string(), "file_exists".to_string(), "remove_file".to_string()]);
        namespaces.insert("std.string".to_string(), vec![
            "str_split".to_string(), "str_slice".to_string(), "str_trim".to_string(), "str_trim_start".to_string(),
            "str_trim_end".to_string(), "str_starts_with".to_string(), "str_ends_with".to_string(),
            "str_index_of".to_string(), "str_replace".to_string(), "str_char_at".to_string(),
            "str_repeat".to_string(), "str_to_int".to_string(), "str_len".to_string(), "to_string".to_string(),
        ]);
        namespaces.insert("std.json".to_string(), vec![
            "json_parse".to_string(), "json_stringify".to_string(), "json_get".to_string(),
        ]);
        namespaces.insert("std.syntax".to_string(), vec![
            "mesh_lex".to_string(), "mesh_layout".to_string(), "mesh_check".to_string(),
            "mesh_format".to_string(), "mesh_parse_ast".to_string(),
        ]);
        namespaces.insert("math".to_string(), vec!["sin".to_string(), "cos".to_string(), "sqrt".to_string(), "abs".to_string()]);

        let str_methods = vec![
            "replace".to_string(), "trim".to_string(), "trim_start".to_string(), "trim_end".to_string(),
            "split".to_string(), "slice".to_string(), "starts_with".to_string(), "ends_with".to_string(),
            "index_of".to_string(), "repeat".to_string(), "to_int".to_string(), "len".to_string(),
            "char_at".to_string(), "to_string".to_string(),
        ];
        namespaces.insert("str".to_string(), str_methods.clone());
        namespaces.insert("String".to_string(), str_methods);
        namespaces.insert("json".to_string(), vec!["parse".to_string(), "stringify".to_string(), "get".to_string()]);
        namespaces.insert("io".to_string(), vec!["println".to_string(), "print".to_string(), "read_line".to_string(), "read_stdin".to_string()]);

        Self {
            function_stack: vec![FunctionScope::new("global".to_string(), global_scope_name)],
            globals: HashMap::new(),
            global_compiled_names: HashMap::new(),
            namespaces,
            enum_variants: HashMap::new(),
        }
    }

    pub fn register_global(&mut self, name: String, span: Span) -> Result<(), SemaError> {
        if self.globals.contains_key(&name) {
            return Err(SemaError::duplicate(name, span));
        }
        self.globals.insert(name, span);
        Ok(())
    }

    pub fn register_global_compiled(
        &mut self,
        name: String,
        compiled_name: String,
        span: Span,
    ) -> Result<(), SemaError> {
        self.register_global(name.clone(), span)?;
        self.global_compiled_names.insert(name, compiled_name);
        Ok(())
    }

    pub fn register_enum_variant(&mut self, enum_name: String, variant_name: String, span: Span) {
        self.enum_variants.insert(variant_name, (enum_name, span));
    }

    pub fn register_namespace(&mut self, path: &str, child: String) {
        self.namespaces.entry(path.to_string()).or_default().push(child);
    }

    pub fn current_fn(&self) -> &FunctionScope {
        self.function_stack.last().unwrap()
    }

    pub fn current_fn_mut(&mut self) -> &mut FunctionScope {
        self.function_stack.last_mut().unwrap()
    }

    pub fn enter_function(&mut self, name: String, compiled_name: String) {
        self.function_stack.push(FunctionScope::new(name, compiled_name));
    }

    pub fn exit_function(&mut self) -> FunctionScope {
        assert!(self.function_stack.len() > 1, "Cannot exit global function scope");
        self.function_stack.pop().unwrap()
    }

    /// Resolves an identifier across local block scopes, enclosing functions (upvalues), and globals
    pub fn resolve(&mut self, name: &str, span: Span) -> Result<ResolvedVar, SemaError> {
        // 1. Try local scopes in current function
        if let Some(local) = self.current_fn().find_local(name) {
            return Ok(ResolvedVar::Local {
                slot: local.slot,
                kind: local.kind,
                name: name.to_string(),
                span,
            });
        }

        // 2. Try upvalue resolution across outer function scopes
        if self.function_stack.len() > 1 {
            if let Some(upvalue) = self.resolve_upvalue(self.function_stack.len() - 1, name, span) {
                return Ok(upvalue);
            }
        }

        // 3. Try global symbols
        if self.globals.contains_key(name) {
            return Ok(ResolvedVar::Global {
                name: self
                    .global_compiled_names
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.to_string()),
                span,
            });
        }

        // 3b. Try registered VM builtins
        if crate::builtins::Builtins::is_builtin(name) {
            return Ok(ResolvedVar::Global {
                name: name.to_string(),
                span,
            });
        }

        // 4. Try namespaces (e.g. `std`, `math`)
        if self.namespaces.contains_key(name) {
            return Ok(ResolvedVar::Namespace {
                path: vec![name.to_string()],
                span,
            });
        }

        // 5. Try enum variants with or without leading colon
        let raw_variant = name.strip_prefix(':').unwrap_or(name);
        if let Some((enum_name, _)) = self.enum_variants.get(raw_variant) {
            return Ok(ResolvedVar::EnumVariant {
                enum_name: Some(enum_name.clone()),
                variant_name: raw_variant.to_string(),
                span,
            });
        }

        Err(SemaError::undefined(name, span))
    }

    fn resolve_upvalue(&mut self, fn_idx: usize, name: &str, span: Span) -> Option<ResolvedVar> {
        if fn_idx <= 1 {
            return None;
        }

        let parent_idx = fn_idx - 1;

        // Check if parent has it as a local
        if let Some(parent_local) = self.function_stack[parent_idx].find_local(name) {
            let is_const = parent_local.kind == VarKind::Constant;
            let kind = if is_const {
                VarKind::Upvalue {
                    is_const: true,
                    parent_index: parent_local.slot,
                    is_direct_local: true,
                }
            } else {
                VarKind::Upvalue {
                    is_const: false,
                    parent_index: parent_local.slot,
                    is_direct_local: true,
                }
            };
            let upval_idx = self.function_stack[fn_idx].add_upvalue(
                name.to_string(),
                kind,
                parent_local.slot,
                true,
                span,
            );
            return Some(ResolvedVar::Upvalue {
                upvalue_index: upval_idx,
                kind,
                name: name.to_string(),
                span,
            });
        }

        // Otherwise check if parent has it as an upvalue (recursive capture)
        if let Some(parent_upval_var) = self.resolve_upvalue(parent_idx, name, span) {
            if let ResolvedVar::Upvalue { upvalue_index: parent_upval_idx, kind: parent_kind, .. } = parent_upval_var {
                let is_const = match parent_kind {
                    VarKind::Constant => true,
                    VarKind::Upvalue { is_const, .. } => is_const,
                    VarKind::Mutable => false,
                };
                let kind = VarKind::Upvalue {
                    is_const,
                    parent_index: parent_upval_idx,
                    is_direct_local: false,
                };
                let upval_idx = self.function_stack[fn_idx].add_upvalue(
                    name.to_string(),
                    kind,
                    parent_upval_idx,
                    false,
                    span,
                );
                return Some(ResolvedVar::Upvalue {
                    upvalue_index: upval_idx,
                    kind,
                    name: name.to_string(),
                    span,
                });
            }
        }

        None
    }

    /// Resolves a dotted path (e.g. `["std", "io", "println"]` or `["math", "sin"]`)
    pub fn resolve_dotted_path(&self, parts: &[String], span: Span) -> Result<ResolvedVar, SemaError> {
        if parts.is_empty() {
            return Err(SemaError::undefined("", span));
        }

        let full_path = parts.join(".");
        // Check if entire path is a known namespace
        if self.namespaces.contains_key(&full_path) {
            return Ok(ResolvedVar::Namespace {
                path: parts.to_vec(),
                span,
            });
        }

        // Check if prefix is a namespace and the leaf is a function/item
        if parts.len() > 1 {
            let prefix = parts[..parts.len() - 1].join(".");
            let leaf = &parts[parts.len() - 1];
            if let Some(children) = self.namespaces.get(&prefix) {
                if children.iter().any(|c| c == leaf) {
                    return Ok(ResolvedVar::QualifiedGlobal {
                        path: parts.to_vec(),
                        span,
                    });
                }
            }
        }

        // Check if global matches full dotted name
        if self.globals.contains_key(&full_path) {
            return Ok(ResolvedVar::QualifiedGlobal {
                path: parts.to_vec(),
                span,
            });
        }

        Err(SemaError::unknown_namespace(full_path, span))
    }
}

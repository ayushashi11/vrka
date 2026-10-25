use super::error::SemaError;
use super::loader::ModuleLoader;
use super::scope::{ScopeManager, UpvalueDesc};
use super::symbol::{ResolvedVar, VarKind};
use crate::syntax::cst::*;
use crate::syntax::token::Span;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ResolvedFunction {
    pub name: String,
    pub total_locals: usize,
    pub upvalues: Vec<UpvalueDesc>,
    pub compiled_name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ResolvedStruct {
    pub name: String,
    pub compiled_name: String,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<(String, String)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ResolvedEnum {
    pub name: String,
    pub compiled_name: String,
    pub variants: Vec<VariantDef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ResolvedProgram {
    pub var_resolutions: HashMap<Span, ResolvedVar>,
    pub functions: Vec<ResolvedFunction>,
    pub structs: Vec<ResolvedStruct>,
    pub enums: Vec<ResolvedEnum>,
    pub top_level_consts: HashMap<String, LitVal>,
    pub imported_modules: Vec<Module>,
}

pub struct Resolver {
    pub start_filename: String,
    pub scopes: ScopeManager,
    pub var_resolutions: HashMap<Span, ResolvedVar>,
    pub completed_functions: Vec<ResolvedFunction>,
    pub top_level_consts: HashMap<String, LitVal>,
    pub loader: ModuleLoader,
    pub imported_modules: Vec<Module>,
    resolving_method: bool,
}

impl Resolver {
    pub fn new(start_filename: String) -> Self {
        Self::with_loader(start_filename, ModuleLoader::new(None))
    }

    pub fn with_loader(start_filename: String, loader: ModuleLoader) -> Self {
        Self {
            scopes: ScopeManager::new(start_filename.clone()),
            start_filename,
            var_resolutions: HashMap::new(),
            completed_functions: Vec::new(),
            top_level_consts: HashMap::new(),
            loader,
            imported_modules: Vec::new(),
            resolving_method: false,
        }
    }

    pub fn resolve_module(&mut self, module: &Module) -> Result<ResolvedProgram, SemaError> {
        // ── Phase 1: Top-level Declaration Discovery ─────────────────────────
        for item in &module.items {
            match item {
                Item::Decl(Decl::Fn(f)) => {
                    let module_path = module.path.to_string_lossy().to_string();
                    let compiled_name = self.compiled_function_name(&module_path, &f.name);
                    self.scopes
                        .register_global_compiled(f.name.clone(), compiled_name, f.span)?;
                }
                Item::Decl(Decl::Struct(s)) => {
                    let module_path = module.path.to_string_lossy().to_string();
                    self.scopes.register_global_compiled(
                        s.name.clone(),
                        format!("{}.{}", module_path, s.name),
                        s.span,
                    )?;
                }
                Item::Decl(Decl::Enum(e)) => {
                    let module_path = module.path.to_string_lossy().to_string();
                    self.scopes.register_global_compiled(
                        e.name.clone(),
                        format!("{}.{}", module_path, e.name),
                        e.span,
                    )?;
                    for v in &e.variants {
                        self.scopes
                            .register_enum_variant(e.name.clone(), v.name.clone(), v.span);
                    }
                }
                Item::Decl(Decl::Protocol(p)) => {
                    self.scopes.register_global(p.name.clone(), p.span)?;
                }
                Item::Decl(Decl::Impl(_)) => {}
                Item::Decl(Decl::Import(imp)) => {
                    self.resolve_import(imp)?;
                }
                Item::Stmt(Stmt::Const {
                    name, init, span, ..
                }) => {
                    self.scopes.register_global(name.clone(), *span)?;
                    if let Expr::Literal(lit, _) = init {
                        self.top_level_consts.insert(name.clone(), lit.clone());
                    }
                }
                Item::Stmt(_) => {}
            }
        }

        self.validate_protocol_implementations(module)?;

        // ── Phase 2: Body Resolution ─────────────────────────────────────────
        for item in &module.items {
            match item {
                Item::Decl(Decl::Fn(f)) => {
                    self.resolve_fn_def(f, module.path.to_string_lossy().to_string(), false)?;
                }
                Item::Decl(Decl::Impl(impl_def)) => {
                    for method in &impl_def.methods {
                        self.resolve_fn_def(
                            method,
                            impl_compiled_parent(&module.path, impl_def),
                            true,
                        )?;
                    }
                }
                Item::Decl(_) => {}
                Item::Stmt(stmt) => {
                    self.resolve_stmt(stmt)?;
                }
            }
        }

        let has_main = module.items.iter().any(|item| match item {
            Item::Decl(Decl::Fn(f)) => f.name == "main",
            _ => false,
        });
        let has_top_level_stmts = module
            .items
            .iter()
            .any(|item| matches!(item, Item::Stmt(_)));
        if !has_main && has_top_level_stmts {
            let global_scope = &self.scopes.function_stack[0];
            self.completed_functions.push(ResolvedFunction {
                name: "main".to_string(),
                compiled_name: "main".to_string(),
                total_locals: global_scope.next_slot,
                upvalues: global_scope.upvalues.clone(),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    col: 1,
                },
            });
        }

        let mut structs = Vec::new();
        let mut enums = Vec::new();
        for struct_module in std::iter::once(module).chain(self.imported_modules.iter()) {
            let module_path = struct_module.path.to_string_lossy();
            for item in &struct_module.items {
                match item {
                    Item::Decl(Decl::Struct(def)) => {
                        let compiled_name = format!("{}.{}", module_path, def.name);
                        let mut methods = Vec::new();
                        for impl_item in &struct_module.items {
                            let Item::Decl(Decl::Impl(impl_def)) = impl_item else {
                                continue;
                            };
                            if impl_def.target_type.get_compiled_name() != def.name {
                                continue;
                            }
                            for method in &impl_def.methods {
                                let compiled_method = self
                                    .completed_functions
                                    .iter()
                                    .find(|function| function.span == method.span)
                                    .map(|function| function.compiled_name.clone())
                                    .unwrap_or_else(|| {
                                        format!("{}.{}.{}", module_path, def.name, method.name)
                                    });
                                methods.push((method.name.clone(), compiled_method));
                            }
                        }
                        structs.push(ResolvedStruct {
                            name: def.name.clone(),
                            compiled_name,
                            fields: def.fields.clone(),
                            methods,
                            span: def.span,
                        });
                    }
                    Item::Decl(Decl::Enum(def)) => enums.push(ResolvedEnum {
                        name: def.name.clone(),
                        compiled_name: format!("{}.{}", module_path, def.name),
                        variants: def.variants.clone(),
                        span: def.span,
                    }),
                    _ => {}
                }
            }
        }

        Ok(ResolvedProgram {
            var_resolutions: std::mem::take(&mut self.var_resolutions),
            functions: std::mem::take(&mut self.completed_functions),
            structs,
            enums,
            top_level_consts: std::mem::take(&mut self.top_level_consts),
            imported_modules: std::mem::take(&mut self.imported_modules),
        })
    }

    fn resolve_import(&mut self, imp: &ImportDef) -> Result<(), SemaError> {
        match &imp.target {
            ImportTarget::Dotted(parts) => {
                let full = parts.join(".");
                if let Some(symbols) = self.scopes.namespaces.get(&full).cloned() {
                    for sym in symbols {
                        let _ = self.scopes.register_global(sym, imp.span);
                    }
                }
            }
            ImportTarget::Path(path_str) | ImportTarget::Ident(path_str) => {
                let resolved_path = self.loader.resolve_path(path_str).ok_or_else(|| {
                    SemaError::import_error(
                        format!("Cannot resolve module '{}'", path_str),
                        imp.span,
                    )
                })?;

                let (imported_module, _canonical) = self
                    .loader
                    .load_file(&resolved_path)
                    .map_err(|e| SemaError::import_error(e, imp.span))?;

                // 1. Discover top-level declarations of imported module
                for item in &imported_module.items {
                    match item {
                        Item::Decl(Decl::Fn(f)) => {
                            if f.name != "main" {
                                let module_path =
                                    imported_module.path.to_string_lossy().to_string();
                                let _ = self.scopes.register_global_compiled(
                                    f.name.clone(),
                                    self.compiled_function_name(&module_path, &f.name),
                                    f.span,
                                );
                            }
                        }
                        Item::Decl(Decl::Struct(s)) => {
                            let module_path = imported_module.path.to_string_lossy().to_string();
                            let _ = self.scopes.register_global_compiled(
                                s.name.clone(),
                                format!("{}.{}", module_path, s.name),
                                s.span,
                            );
                        }
                        Item::Decl(Decl::Enum(e)) => {
                            let module_path = imported_module.path.to_string_lossy().to_string();
                            let _ = self.scopes.register_global_compiled(
                                e.name.clone(),
                                format!("{}.{}", module_path, e.name),
                                e.span,
                            );
                            for v in &e.variants {
                                self.scopes.register_enum_variant(
                                    e.name.clone(),
                                    v.name.clone(),
                                    v.span,
                                );
                            }
                        }
                        Item::Decl(Decl::Protocol(p)) => {
                            let _ = self.scopes.register_global(p.name.clone(), p.span);
                        }
                        Item::Decl(Decl::Import(nested_imp)) => {
                            self.resolve_import(nested_imp)?;
                        }
                        Item::Stmt(Stmt::Const {
                            name, init, span, ..
                        }) => {
                            let _ = self.scopes.register_global(name.clone(), *span);
                            if let Expr::Literal(lit, _) = init {
                                self.top_level_consts.insert(name.clone(), lit.clone());
                            }
                        }
                        _ => {}
                    }
                }

                // 2. Resolve bodies of imported module
                for item in &imported_module.items {
                    match item {
                        Item::Decl(Decl::Fn(f)) => {
                            self.resolve_fn_def(
                                f,
                                imported_module.path.to_string_lossy().to_string(),
                                false,
                            )?;
                        }
                        Item::Decl(Decl::Impl(impl_def)) => {
                            for method in &impl_def.methods {
                                self.resolve_fn_def(
                                    method,
                                    impl_compiled_parent(&imported_module.path, impl_def),
                                    true,
                                )?;
                            }
                        }
                        _ => {}
                    }
                }

                self.imported_modules.push(imported_module);
            }
        }
        Ok(())
    }

    fn resolve_fn_def(
        &mut self,
        f: &FnDef,
        parent: String,
        is_method: bool,
    ) -> Result<(), SemaError> {
        let compiled_name = self.compiled_function_name(&parent, &f.name);
        self.scopes
            .enter_function(f.name.clone(), compiled_name.clone());

        for param in &f.params {
            self.scopes.current_fn_mut().define_local(
                param.name.clone(),
                VarKind::Constant,
                param.span,
            )?;
        }

        let was_resolving_method = self.resolving_method;
        self.resolving_method = is_method;
        let body_result = self.resolve_block(&f.body);
        self.resolving_method = was_resolving_method;
        body_result?;

        let fn_scope = self.scopes.exit_function();
        self.completed_functions.push(ResolvedFunction {
            name: f.name.clone(),
            total_locals: fn_scope.next_slot,
            upvalues: fn_scope.upvalues,
            span: f.span,
            compiled_name,
        });

        Ok(())
    }

    fn compiled_function_name(&self, parent: &str, name: &str) -> String {
        if name == "main" && parent == self.start_filename {
            "main".to_string()
        } else {
            format!("{}.{}", parent, name)
        }
    }

    fn validate_protocol_implementations(&self, module: &Module) -> Result<(), SemaError> {
        let modules = std::iter::once(module).chain(self.imported_modules.iter());
        for impl_module in std::iter::once(module).chain(self.imported_modules.iter()) {
            for item in &impl_module.items {
                let Item::Decl(Decl::Impl(impl_def)) = item else {
                    continue;
                };
                let Some(protocol_type) = &impl_def.for_protocol else {
                    continue;
                };
                let protocol_name = protocol_type.get_compiled_name();
                let protocol = modules
                    .clone()
                    .flat_map(|m| m.items.iter())
                    .find_map(|item| match item {
                        Item::Decl(Decl::Protocol(protocol)) if protocol.name == protocol_name => {
                            Some(protocol)
                        }
                        _ => None,
                    });

                let (required_properties, required_methods) = if let Some(protocol) = protocol {
                    (
                        protocol
                            .properties
                            .iter()
                            .map(|property| {
                                (
                                    property.name.as_str(),
                                    property.type_ann.get_compiled_name(),
                                )
                            })
                            .collect::<Vec<_>>(),
                        protocol.methods.iter().collect::<Vec<_>>(),
                    )
                } else {
                    match protocol_name.as_str() {
                        "ToString" => (Vec::new(), Vec::new()),
                        "Format" => (Vec::new(), Vec::new()),
                        _ => {
                            return Err(SemaError::custom(
                                format!("Unknown protocol '{}'", protocol_name),
                                impl_def.span,
                            ));
                        }
                    }
                };

                let target_name = impl_def.target_type.get_compiled_name();
                let target_struct = std::iter::once(module)
                    .chain(self.imported_modules.iter())
                    .flat_map(|m| m.items.iter())
                    .find_map(|item| match item {
                        Item::Decl(Decl::Struct(def)) if def.name == target_name => Some(def),
                        _ => None,
                    });

                for (property_name, expected_type) in required_properties {
                    let actual_type = if let Some(target_struct) = target_struct {
                        target_struct
                            .fields
                            .iter()
                            .find(|field| field.name == property_name)
                            .map(|field| field.type_ann.get_compiled_name())
                    } else {
                        builtin_property_type(&target_name, property_name).map(str::to_string)
                    };
                    match actual_type {
                        Some(actual_type) if actual_type == expected_type => {}
                        Some(actual_type) => {
                            return Err(SemaError::custom(
                                format!(
                                    "Type '{}' does not satisfy protocol '{}': property '{}' has type '{}', expected '{}'",
                                    target_name,
                                    protocol_name,
                                    property_name,
                                    actual_type,
                                    expected_type
                                ),
                                impl_def.span,
                            ));
                        }
                        None => {
                            return Err(SemaError::custom(
                                format!(
                                    "Type '{}' does not satisfy protocol '{}': missing property '{}'",
                                    target_name, protocol_name, property_name
                                ),
                                impl_def.span,
                            ));
                        }
                    }
                }

                for required_method in required_methods {
                    let Some(method) = impl_def
                        .methods
                        .iter()
                        .find(|method| method.name == required_method.name)
                    else {
                        return Err(SemaError::custom(
                            format!(
                                "Type '{}' does not satisfy protocol '{}': missing method '{}'",
                                target_name, protocol_name, required_method.name
                            ),
                            impl_def.span,
                        ));
                    };
                    if !method_signatures_match(required_method, method) {
                        return Err(SemaError::custom(
                            format!(
                                "Type '{}' does not satisfy protocol '{}': method '{}' has an incompatible signature",
                                target_name, protocol_name, required_method.name
                            ),
                            method.span,
                        ));
                    }
                }

                if protocol_name == "ToString" {
                    let Some(method) = impl_def
                        .methods
                        .iter()
                        .find(|method| method.name == "to_string")
                    else {
                        return Err(SemaError::custom(
                            format!(
                                "Type '{}' does not satisfy protocol 'ToString': missing method 'to_string'",
                                target_name
                            ),
                            impl_def.span,
                        ));
                    };
                    if method
                        .params
                        .iter()
                        .filter(|param| param.name == "self")
                        .count()
                        > 1
                        || method.params.iter().any(|param| param.name != "self")
                        || method
                            .ret_type
                            .as_ref()
                            .map(TypeAnn::get_compiled_name)
                            .as_deref()
                            != Some("String")
                    {
                        return Err(SemaError::custom(
                            format!(
                                "Type '{}' does not satisfy protocol 'ToString': method 'to_string' must have signature fn() -> String",
                                target_name
                            ),
                            method.span,
                        ));
                    }
                }

                if protocol_name == "Format" {
                    let Some(method) = impl_def
                        .methods
                        .iter()
                        .find(|method| method.name == "format")
                    else {
                        return Err(SemaError::custom(
                            format!(
                                "Type '{}' does not satisfy protocol 'Format': missing method 'format'",
                                target_name
                            ),
                            impl_def.span,
                        ));
                    };
                    let params = method
                        .params
                        .iter()
                        .filter(|param| param.name != "self")
                        .map(|param| param.type_ann.as_ref().map(TypeAnn::get_compiled_name))
                        .collect::<Vec<_>>();
                    if params != [Some("String".to_string())]
                        || method
                            .ret_type
                            .as_ref()
                            .map(TypeAnn::get_compiled_name)
                            .as_deref()
                            != Some("String")
                    {
                        return Err(SemaError::custom(
                            format!(
                                "Type '{}' does not satisfy protocol 'Format': method 'format' must have signature fn(String) -> String",
                                target_name
                            ),
                            method.span,
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn resolve_block(&mut self, block: &Block) -> Result<(), SemaError> {
        self.scopes.current_fn_mut().enter_block();

        for stmt in &block.stmts {
            self.resolve_stmt(stmt)?;
        }

        if let Some(tail) = &block.tail_expr {
            self.resolve_expr(tail)?;
        }

        self.scopes.current_fn_mut().exit_block();
        Ok(())
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) -> Result<(), SemaError> {
        match stmt {
            Stmt::Let {
                name, init, span, ..
            } => {
                self.resolve_expr(init)?;
                let slot = self.scopes.current_fn_mut().define_local(
                    name.clone(),
                    VarKind::Mutable,
                    *span,
                )?;
                self.var_resolutions.insert(
                    *span,
                    ResolvedVar::Local {
                        slot,
                        kind: VarKind::Mutable,
                        name: name.clone(),
                        span: *span,
                    },
                );
            }
            Stmt::Const {
                name, init, span, ..
            } => {
                self.resolve_expr(init)?;
                let slot = self.scopes.current_fn_mut().define_local(
                    name.clone(),
                    VarKind::Constant,
                    *span,
                )?;
                self.var_resolutions.insert(
                    *span,
                    ResolvedVar::Local {
                        slot,
                        kind: VarKind::Constant,
                        name: name.clone(),
                        span: *span,
                    },
                );
            }
            Stmt::Assign { target, val, span } => {
                self.resolve_expr(val)?;
                self.check_and_resolve_assignment(target, *span)?;
            }
            Stmt::AugAssign {
                target, val, span, ..
            } => {
                self.resolve_expr(val)?;
                self.check_and_resolve_assignment(target, *span)?;
            }
            Stmt::While { cond, body, .. } => {
                self.resolve_expr(cond)?;
                self.resolve_block(body)?;
            }
            Stmt::Loop { body, .. } => {
                self.resolve_block(body)?;
            }
            Stmt::Return { val, .. } => {
                if let Some(v) = val {
                    self.resolve_expr(v)?;
                }
            }
            Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::Expr(e) => {
                self.resolve_expr(e)?;
            }
        }
        Ok(())
    }

    fn check_and_resolve_assignment(
        &mut self,
        target: &Expr,
        assign_span: Span,
    ) -> Result<(), SemaError> {
        match target {
            Expr::Ident(name, span) => {
                if name == "self" && self.resolving_method {
                    return Err(SemaError::assign_constant(name, assign_span));
                }
                let resolved = self.scopes.resolve(name, *span)?;
                match &resolved {
                    ResolvedVar::Local { kind, .. } => {
                        if *kind == VarKind::Constant {
                            return Err(SemaError::assign_constant(name, assign_span));
                        }
                    }
                    ResolvedVar::Upvalue { kind, .. } => {
                        if let VarKind::Upvalue { is_const, .. } = kind {
                            if *is_const {
                                return Err(SemaError::assign_constant(name, assign_span));
                            }
                        }
                    }
                    ResolvedVar::Global { .. } => {
                        return Err(SemaError::assign_constant(name, assign_span));
                    }
                    _ => {}
                }
                // Mark variable as mutated
                self.scopes.current_fn_mut().mark_mutated(name);
                self.var_resolutions.insert(*span, resolved);
                Ok(())
            }
            Expr::FieldAccess { base, .. } => self.resolve_expr(base),
            Expr::Index { base, index, .. } => {
                self.resolve_expr(base)?;
                self.resolve_expr(index)
            }
            other => Err(SemaError::invalid_assignment_target(
                format!("{:?}", other),
                assign_span,
            )),
        }
    }

    pub fn resolve_expr(&mut self, expr: &Expr) -> Result<(), SemaError> {
        match expr {
            Expr::Literal(_, _) => Ok(()),
            Expr::InterpolatedString(parts, _) => {
                for part in parts {
                    if let crate::syntax::cst::StringPart::Expr(e) = part {
                        self.resolve_expr(e)?;
                    }
                }
                Ok(())
            }
            Expr::Ident(name, span) => {
                if name == "self" && self.resolving_method {
                    self.var_resolutions
                        .insert(*span, ResolvedVar::This { span: *span });
                    return Ok(());
                }
                let resolved = self.scopes.resolve(name, *span)?;
                self.var_resolutions.insert(*span, resolved);
                Ok(())
            }
            Expr::Atom(name, span) => {
                // Enum variant like :Circle or :Rect
                let raw_name = name.strip_prefix(':').unwrap_or(name);
                let resolved = if let Some((enum_name, _)) = self.scopes.enum_variants.get(raw_name)
                {
                    ResolvedVar::EnumVariant {
                        enum_name: Some(enum_name.clone()),
                        variant_name: raw_name.to_string(),
                        span: *span,
                    }
                } else {
                    ResolvedVar::EnumVariant {
                        enum_name: None,
                        variant_name: raw_name.to_string(),
                        span: *span,
                    }
                };
                self.var_resolutions.insert(*span, resolved);
                Ok(())
            }
            Expr::Binary { left, right, .. } => {
                self.resolve_expr(left)?;
                self.resolve_expr(right)
            }
            Expr::Unary { operand, .. } => self.resolve_expr(operand),
            Expr::Call { callee, args, .. } => {
                self.resolve_expr(callee)?;
                for arg in args {
                    self.resolve_expr(arg)?;
                }
                Ok(())
            }
            Expr::FieldAccess { base, field, span } => {
                // Check if base is a dotted path (e.g. std.io.println or math.sin)
                if let Some(parts) = self.collect_dotted_path(base, field) {
                    if let Ok(resolved) = self.scopes.resolve_dotted_path(&parts, *span) {
                        self.var_resolutions.insert(*span, resolved);
                        return Ok(());
                    }
                }
                // Otherwise it's a dynamic/prototype object member access: resolve base
                self.resolve_expr(base)
            }
            Expr::Index { base, index, .. } => {
                self.resolve_expr(base)?;
                self.resolve_expr(index)
            }
            Expr::ArrayLit(items, _) | Expr::TupleLit(items, _) => {
                for item in items {
                    self.resolve_expr(item)?;
                }
                Ok(())
            }
            Expr::RecordLit(fields, _) => {
                for (_, item) in fields {
                    self.resolve_expr(item)?;
                }
                Ok(())
            }
            Expr::Block(block) => self.resolve_block(block),
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.resolve_expr(cond)?;
                self.resolve_block(then_branch)?;
                if let Some(eb) = else_branch {
                    self.resolve_block(eb)?;
                }
                Ok(())
            }
            Expr::Match { target, arms, .. } => {
                self.resolve_expr(target)?;
                for arm in arms {
                    self.scopes.current_fn_mut().enter_block();
                    self.resolve_pattern_bindings(&arm.pattern)?;
                    self.resolve_expr(&arm.body)?;
                    self.scopes.current_fn_mut().exit_block();
                }
                Ok(())
            }
            Expr::Spawn { callee, args, .. } => {
                self.resolve_expr(callee)?;
                for arg in args {
                    self.resolve_expr(arg)?;
                }
                Ok(())
            }
            Expr::Send {
                receiver, message, ..
            } => {
                self.resolve_expr(receiver)?;
                self.resolve_expr(message)
            }
            Expr::Receive(_) => Ok(()),
            Expr::Lambda {
                params, body, span, ..
            } => {
                let lambda_name = format!("lambda_{}_{}", span.line, span.col);
                let compiled_name =
                    format!("{}.{}", self.scopes.current_fn().compiled_name, lambda_name);
                self.scopes
                    .enter_function(lambda_name.clone(), compiled_name);

                for p in params {
                    self.scopes.current_fn_mut().define_local(
                        p.name.clone(),
                        VarKind::Constant,
                        p.span,
                    )?;
                }

                self.resolve_block(body)?;

                let lambda_scope = self.scopes.exit_function();
                self.completed_functions.push(ResolvedFunction {
                    name: lambda_name,
                    total_locals: lambda_scope.next_slot,
                    upvalues: lambda_scope.upvalues,
                    span: *span,
                    compiled_name: lambda_scope.compiled_name,
                });
                Ok(())
            }
        }
    }

    fn collect_dotted_path(&self, base: &Expr, leaf_field: &str) -> Option<Vec<String>> {
        match base {
            Expr::Ident(name, _) => Some(vec![name.clone(), leaf_field.to_string()]),
            Expr::FieldAccess {
                base: inner_base,
                field: inner_field,
                ..
            } => {
                let mut path = self.collect_dotted_path(inner_base, inner_field)?;
                path.push(leaf_field.to_string());
                Some(path)
            }
            _ => None,
        }
    }

    fn resolve_pattern_bindings(&mut self, pat: &PatternNode) -> Result<(), SemaError> {
        match pat {
            PatternNode::Bind(name, span) => {
                let slot = self.scopes.current_fn_mut().define_local(
                    name.clone(),
                    VarKind::Constant,
                    *span,
                )?;
                self.var_resolutions.insert(
                    *span,
                    ResolvedVar::Local {
                        slot,
                        kind: VarKind::Constant,
                        name: name.clone(),
                        span: *span,
                    },
                );
            }
            PatternNode::Tuple(pats, _) => {
                for p in pats {
                    self.resolve_pattern_bindings(p)?;
                }
            }
            PatternNode::Struct { fields, .. } => {
                for field_pat in fields {
                    match field_pat {
                        StructPatternNode::KeyValue { value, .. } => {
                            self.resolve_pattern_bindings(value)?;
                        }
                        StructPatternNode::Shorthand { key, span } => {
                            let slot = self.scopes.current_fn_mut().define_local(
                                key.clone(),
                                VarKind::Constant,
                                *span,
                            )?;
                            self.var_resolutions.insert(
                                *span,
                                ResolvedVar::Local {
                                    slot,
                                    kind: VarKind::Constant,
                                    name: key.clone(),
                                    span: *span,
                                },
                            );
                        }
                    }
                }
            }
            PatternNode::EnumVariant { args, .. } => {
                for arg in args {
                    self.resolve_pattern_bindings(arg)?;
                }
            }
            PatternNode::Array { segments, .. } => {
                for seg in segments {
                    match seg {
                        PatternSegment::Element(p) => self.resolve_pattern_bindings(p)?,
                        PatternSegment::BindList(name, span) => {
                            let slot = self.scopes.current_fn_mut().define_local(
                                name.clone(),
                                VarKind::Constant,
                                *span,
                            )?;
                            self.var_resolutions.insert(
                                *span,
                                ResolvedVar::Local {
                                    slot,
                                    kind: VarKind::Constant,
                                    name: name.clone(),
                                    span: *span,
                                },
                            );
                        }
                        PatternSegment::WildcardList(_) => {}
                    }
                }
            }
            PatternNode::Wildcard(_) | PatternNode::Literal(_, _) | PatternNode::Range { .. } => {}
        }
        Ok(())
    }
}

fn impl_compiled_parent(path: &std::path::Path, impl_def: &ImplDef) -> String {
    let module_path = path.to_string_lossy();
    let target_name = impl_def.target_type.get_compiled_name();
    match &impl_def.for_protocol {
        Some(proto_name)
            if proto_name.get_compiled_name() == "ToString"
                && matches!(target_name.as_str(), "Int" | "Float" | "Bool" | "String") =>
        {
            format!("__ionvm.prelude.{}.#ToString", target_name)
        }
        Some(proto_name) => format!(
            "{}.{}.#{}",
            module_path,
            target_name,
            proto_name.get_compiled_name()
        ),
        None => format!("{}.{}", module_path, target_name),
    }
}

fn method_signatures_match(required: &FnSig, actual: &FnDef) -> bool {
    let required_params = required
        .params
        .iter()
        .filter(|param| param.name != "self")
        .map(|param| param.type_ann.as_ref().map(TypeAnn::get_compiled_name))
        .collect::<Vec<_>>();
    let actual_params = actual
        .params
        .iter()
        .filter(|param| param.name != "self")
        .map(|param| param.type_ann.as_ref().map(TypeAnn::get_compiled_name))
        .collect::<Vec<_>>();
    required_params == actual_params
        && required
            .ret_type
            .as_ref()
            .map(TypeAnn::get_compiled_name)
            .unwrap_or_else(|| "Unit".to_string())
            == actual
                .ret_type
                .as_ref()
                .map(TypeAnn::get_compiled_name)
                .unwrap_or_else(|| "Unit".to_string())
}

fn builtin_property_type(type_name: &str, property_name: &str) -> Option<&'static str> {
    match (type_name, property_name) {
        ("String" | "Array" | "IntArray" | "FloatArray", "length") => Some("Int"),
        ("Float", "is_nan" | "is_infinite") => Some("Bool"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;
    use crate::{resolve, resolve_with_loader};

    #[test]
    fn test_resolve_forward_reference_recursion() {
        let src = r#"
        fn is_even(n: Int) -> Bool: {
            if n == 0: { return true; }
            return is_odd(n - 1);
        }

        fn is_odd(n: Int) -> Bool: {
            if n == 0: { return false; }
            return is_even(n - 1);
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        assert_eq!(resolved.functions.len(), 2);
    }

    #[test]
    fn test_protocol_conformance_accepts_matching_properties_and_methods() {
        let src = r#"
        protocol Printable:
            label: String
            fn render(self, width: Int) -> String

        struct Box:
            label: String

        impl Printable for Box:
            fn render(width: Int) -> String:
                return self.label
        "#;
        let module = parse(src).expect("parsed");
        resolve(&module).expect("matching implementation conforms");
    }

    #[test]
    fn test_protocol_conformance_rejects_missing_property() {
        let src = r#"
        protocol Sized:
            length: Int

        struct Box:
            value: Int

        impl Sized for Box: {}
        "#;
        let module = parse(src).expect("parsed");
        let error = resolve(&module).expect_err("missing property must fail conformance");
        assert!(error.to_string().contains("missing property 'length'"));
    }

    #[test]
    fn test_protocol_conformance_rejects_method_signature_mismatch() {
        let src = r#"
        protocol Printable:
            fn render(self, width: Int) -> String

        struct Box:
            label: String

        impl Printable for Box:
            fn render(width: Float) -> String:
                return self.label
        "#;
        let module = parse(src).expect("parsed");
        let error = resolve(&module).expect_err("mismatched method signature must fail");
        assert!(error.to_string().contains("incompatible signature"));
    }

    #[test]
    fn test_format_protocol_requires_the_format_method_contract() {
        let src = r#"
        struct Box:
            value: Int

        impl Format for Box: {}
        "#;
        let module = parse(src).expect("parsed");
        let error = resolve(&module).expect_err("Format implementation must define format");
        assert!(error.to_string().contains("missing method 'format'"));
    }

    #[test]
    fn test_resolve_upvalue_capture() {
        let src = r#"
        fn make_adder(x: Int): {
            return fn(y: Int): {
                return x + y;
            };
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        assert_eq!(resolved.functions.len(), 2);
        // Find the lambda function
        let lambda = resolved
            .functions
            .iter()
            .find(|f| f.name.starts_with("lambda"))
            .expect("lambda found");
        assert_eq!(lambda.upvalues.len(), 1);
        assert_eq!(lambda.upvalues[0].name, "x");
        assert!(lambda.upvalues[0].is_direct_local);
    }

    #[test]
    fn test_resolve_const_reassignment_error() {
        let src = r#"
        fn test(): {
            const c = 10;
            c = 20;
        }
        "#;
        let module = parse(src).expect("parsed");
        let res = resolve(&module);
        assert!(res.is_err());
        match res.unwrap_err() {
            SemaError::CannotAssignToConstant { name, .. } => assert_eq!(name, "c"),
            other => panic!("expected CannotAssignToConstant, got {:?}", other),
        }
    }

    #[test]
    fn test_resolve_dotted_names_and_enum_variants() {
        let src = r#"
        enum Shape: {
            Circle(Float),
            Rect(Float, Float),
        }

        fn draw(s: Shape): {
            match s: {
                :Circle(r) => {
                    std.io.println("circle");
                },
                :Rect(w, h) => {
                    std.io.println("rect");
                },
            }
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        assert_eq!(resolved.functions.len(), 1);
    }

    #[test]
    fn test_resolve_import_stdlib_namespace() {
        let src = r#"
        import std.net;
        fn test_net(): {
            let l = tcp_listen(8080);
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        assert_eq!(resolved.functions.len(), 1);
    }

    #[test]
    fn test_resolve_import_file_and_run() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!(
            "mesh_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let helper_path = dir.join("math_helper.mesh");
        let mut f = std::fs::File::create(&helper_path).unwrap();
        writeln!(f, "fn double_it(n: Int) -> Int: {{ return n * 2; }}").unwrap();

        let main_src = format!(
            r#"
            import "{}";
            fn main() -> Int: {{
                return double_it(21);
            }}
            "#,
            helper_path.display()
        );

        let module = parse(&main_src).expect("parsed");
        let loader = ModuleLoader::new(Some(dir.clone()));
        let resolved = resolve_with_loader(&module, loader).expect("resolved");
        let ssa = crate::mir::build_ssa_program(&resolved, &module);

        let mut inferrer = crate::types::TypeInferrer::new();
        inferrer.register_program_declarations(&resolved, &module);
        let type_maps = inferrer.infer_program(&ssa).expect("inferred");
        let bytecode = crate::codegen::CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = vmm::vm::IonVM::new(bytecode).expect("vm init");
        vm.run().expect("vm run");
        assert_eq!(
            vm.entrypoint_return(),
            Some(vmm::lang::types::Value::Int(42))
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_imported_function_name_collisions_use_compiled_names() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!(
            "mesh_collision_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let first_path = dir.join("first.mesh");
        let second_path = dir.join("second.mesh");
        writeln!(
            std::fs::File::create(&first_path).unwrap(),
            "fn shared() -> Int: {{ return 11; }}"
        )
        .unwrap();
        writeln!(
            std::fs::File::create(&second_path).unwrap(),
            "fn shared() -> Int: {{ return 22; }}"
        )
        .unwrap();

        let main_src = format!(
            r#"
            import "{}";
            import "{}";
            fn main() -> Int: {{
                return shared();
            }}
            "#,
            first_path.display(),
            second_path.display()
        );
        let module = parse(&main_src).expect("parsed");
        let resolved =
            resolve_with_loader(&module, ModuleLoader::new(Some(dir.clone()))).expect("resolved");
        let ssa = crate::mir::build_ssa_program(&resolved, &module);

        let first_compiled_name = std::fs::canonicalize(&first_path)
            .unwrap()
            .to_string_lossy()
            .to_string()
            + ".shared";
        let second_compiled_name = std::fs::canonicalize(&second_path)
            .unwrap()
            .to_string_lossy()
            .to_string()
            + ".shared";
        println!("First compiled name: {}", first_compiled_name);
        println!("Second compiled name: {}", second_compiled_name);
        println!(
            "SSA functions: {:?}",
            ssa.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
        assert!(
            ssa.functions
                .iter()
                .any(|function| function.name == first_compiled_name)
        );
        assert!(
            ssa.functions
                .iter()
                .any(|function| function.name == second_compiled_name)
        );
        assert!(ssa.functions.iter().any(|function| function.name == "main"));

        let bytecode = crate::codegen::CodeEmitter::emit_program(&ssa);
        let mut vm = vmm::vm::IonVM::new(bytecode).expect("vm init");
        vm.run().expect("vm run");
        assert_eq!(
            vm.entrypoint_return(),
            Some(vmm::lang::types::Value::Int(11))
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_import_mesh_path_env() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!(
            "mesh_path_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let lib_path = dir.join("calc.mesh");
        let mut f = std::fs::File::create(&lib_path).unwrap();
        writeln!(f, "fn triple_it(n: Int) -> Int: {{ return n * 3; }}").unwrap();

        // Use explicit search path injection to simulate MESH_PATH safely without process-wide env mutation
        let main_src = r#"
            import calc;
            fn main() -> Int: {
                return triple_it(10);
            }
        "#;

        let module = parse(main_src).expect("parsed");
        let loader = ModuleLoader::with_mesh_path(None, Some(dir.to_str().unwrap()));
        let resolved = resolve_with_loader(&module, loader).expect("resolved");
        let ssa = crate::mir::build_ssa_program(&resolved, &module);

        let mut inferrer = crate::types::TypeInferrer::new();
        inferrer.register_program_declarations(&resolved, &module);
        let type_maps = inferrer.infer_program(&ssa).expect("inferred");
        let bytecode = crate::codegen::CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = vmm::vm::IonVM::new(bytecode).expect("vm init");
        vm.run().expect("vm run");
        assert_eq!(
            vm.entrypoint_return(),
            Some(vmm::lang::types::Value::Int(30))
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_resolve_import_circular_error() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!(
            "mesh_circ_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let file_a = dir.join("mod_a.mesh");
        let file_b = dir.join("mod_b.mesh");

        let mut fa = std::fs::File::create(&file_a).unwrap();
        writeln!(
            fa,
            "import \"{}\";\nfn a(): {{ return 1; }}",
            file_b.display()
        )
        .unwrap();

        let mut fb = std::fs::File::create(&file_b).unwrap();
        writeln!(
            fb,
            "import \"{}\";\nfn b(): {{ return 2; }}",
            file_a.display()
        )
        .unwrap();

        let main_src = format!(
            "import \"{}\";\nfn main(): {{ return 0; }}",
            file_a.display()
        );
        let module = parse(&main_src).expect("parsed");
        let loader = ModuleLoader::new(Some(dir.clone()));
        let res = resolve_with_loader(&module, loader);
        assert!(res.is_err());
        let err_msg = format!("{}", res.unwrap_err());
        assert!(err_msg.contains("Circular import detected"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}

use super::ty::*;
use crate::mir::ssa::*;
use crate::syntax::cst::{BinOp, LitVal, PatternNode, PatternSegment, TypeAnn, UnaryOp};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum TypeError {
    #[error("Type mismatch: cannot unify '{0}' with '{1}'")]
    Mismatch(Type, Type),

    #[error("Occurs check failed: type variable {0} occurs in {1}")]
    OccursCheck(TypeVarId, Type),

    #[error("Field '{field}' does not exist on type '{ty}'")]
    MissingField { field: String, ty: Type },

    #[error("Cannot index into non-array type '{0}'")]
    NotIndexable(Type),

    #[error("Function '{name}' expected {expected} arguments, but received {found}")]
    ArityMismatch {
        name: String,
        expected: usize,
        found: usize,
    },

    #[error("Undefined type '{name}'")]
    UndefinedType { name: String },

    #[error("Cannot perform operation '{op}' on type '{ty}'")]
    UnsupportedOp { op: String, ty: Type },

    #[error("Type inference error: {0}")]
    Custom(String),
}

pub struct TypeEnv {
    pub subst: HashMap<TypeVarId, Type>,
    pub next_var: usize,
    pub next_row: usize,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            subst: HashMap::new(),
            next_var: 0,
            next_row: 0,
        }
    }

    pub fn fresh_var(&mut self) -> Type {
        let id = TypeVarId(self.next_var);
        self.next_var += 1;
        Type::Var(id)
    }

    pub fn fresh_row(&mut self) -> RowVarId {
        let id = RowVarId(self.next_row);
        self.next_row += 1;
        id
    }

    pub fn resolve(&self, ty: &Type) -> Type {
        match ty {
            Type::Var(v) => {
                if let Some(target) = self.subst.get(v) {
                    self.resolve(target)
                } else {
                    ty.clone()
                }
            }
            Type::Array(inner) => Type::Array(Box::new(self.resolve(inner))),
            Type::Tuple(items) => Type::Tuple(items.iter().map(|t| self.resolve(t)).collect()),
            Type::Fn { params, ret } => Type::Fn {
                params: params.iter().map(|p| self.resolve(p)).collect(),
                ret: Box::new(self.resolve(ret)),
            },
            Type::Record { fields, row } => {
                let resolved_fields = fields
                    .iter()
                    .map(|(k, v)| (k.clone(), self.resolve(v)))
                    .collect();
                Type::Record {
                    fields: resolved_fields,
                    row: *row,
                }
            }
            Type::Variant { cases, row } => {
                let resolved_cases = cases
                    .iter()
                    .map(|(tag, args)| (tag.clone(), args.iter().map(|a| self.resolve(a)).collect()))
                    .collect();
                Type::Variant {
                    cases: resolved_cases,
                    row: *row,
                }
            }
            other => other.clone(),
        }
    }

    pub fn unify(&mut self, t1: &Type, t2: &Type) -> Result<(), TypeError> {
        let t1 = self.resolve(t1);
        let t2 = self.resolve(t2);

        if t1 == t2 {
            return Ok(());
        }

        match (&t1, &t2) {
            (Type::Var(v1), _) => self.bind_var(*v1, &t2),
            (_, Type::Var(v2)) => self.bind_var(*v2, &t1),

            // Duck typing: IntArray is compatible with Array[Int]
            (Type::IntArray, Type::Array(elem)) | (Type::Array(elem), Type::IntArray) => {
                self.unify(elem, &Type::Int)
            }

            // Duck typing: FloatArray is compatible with Array[Float]
            (Type::FloatArray, Type::Array(elem)) | (Type::Array(elem), Type::FloatArray) => {
                self.unify(elem, &Type::Float)
            }

            (Type::Array(e1), Type::Array(e2)) => self.unify(e1, e2),

            (Type::Tuple(items1), Type::Tuple(items2)) if items1.len() == items2.len() => {
                for (a, b) in items1.iter().zip(items2.iter()) {
                    self.unify(a, b)?;
                }
                Ok(())
            }

            // Row Polymorphism for Records (Duck typing on fields)
            (Type::Record { fields: f1, row: r1 }, Type::Record { fields: f2, row: r2 }) => {
                // Unify common fields
                for (k, v1) in f1 {
                    if let Some(v2) = f2.get(k) {
                        self.unify(v1, v2)?;
                    }
                }
                // If f1 is closed (r1.is_none()), it cannot satisfy fields required by f2 that f1 doesn't have
                if r1.is_none() {
                    for k in f2.keys() {
                        if !f1.contains_key(k) {
                            return Err(TypeError::MissingField {
                                field: k.clone(),
                                ty: t1.clone(),
                            });
                        }
                    }
                }
                // If f2 is closed (r2.is_none()), it cannot satisfy fields required by f1 that f2 doesn't have
                if r2.is_none() {
                    for k in f1.keys() {
                        if !f2.contains_key(k) {
                            return Err(TypeError::MissingField {
                                field: k.clone(),
                                ty: t2.clone(),
                            });
                        }
                    }
                }
                Ok(())
            }

            // Row Polymorphism for Variants (Duck typing on enum tags)
            (Type::Variant { cases: c1, row: r1 }, Type::Variant { cases: c2, row: r2 }) => {
                for (tag, args1) in c1 {
                    if let Some(args2) = c2.get(tag) {
                        if args1.len() != args2.len() {
                            return Err(TypeError::Mismatch(t1, t2));
                        }
                        for (a, b) in args1.iter().zip(args2.iter()) {
                            self.unify(a, b)?;
                        }
                    }
                }
                if r1.is_none() && r2.is_none() && c1.len() != c2.len() {
                    return Err(TypeError::Mismatch(t1, t2));
                }
                Ok(())
            }

            (Type::Fn { params: p1, ret: r1 }, Type::Fn { params: p2, ret: r2 })
                if p1.len() == p2.len() =>
            {
                for (a, b) in p1.iter().zip(p2.iter()) {
                    self.unify(a, b)?;
                }
                self.unify(r1, r2)
            }

            _ => Err(TypeError::Mismatch(t1, t2)),
        }
    }

    fn bind_var(&mut self, var: TypeVarId, ty: &Type) -> Result<(), TypeError> {
        if self.occurs_in(var, ty) {
            return Err(TypeError::OccursCheck(var, ty.clone()));
        }
        self.subst.insert(var, ty.clone());
        Ok(())
    }

    fn occurs_in(&self, var: TypeVarId, ty: &Type) -> bool {
        match self.resolve(ty) {
            Type::Var(v) => v == var,
            Type::Array(inner) => self.occurs_in(var, &inner),
            Type::Tuple(items) => items.iter().any(|t| self.occurs_in(var, t)),
            Type::Fn { params, ret } => {
                params.iter().any(|p| self.occurs_in(var, p)) || self.occurs_in(var, &ret)
            }
            Type::Record { fields, .. } => fields.values().any(|t| self.occurs_in(var, t)),
            Type::Variant { cases, .. } => cases.values().flatten().any(|t| self.occurs_in(var, t)),
            _ => false,
        }
    }

    pub fn type_from_ann(&self, ann: &TypeAnn) -> Type {
        match ann {
            TypeAnn::Named { name, args, .. } => match name.as_str() {
                "Int" => Type::Int,
                "Float" => Type::Float,
                "Bool" => Type::Bool,
                "String" => Type::String,
                "Unit" => Type::Unit,
                "Nil" => Type::Nil,
                "IntArray" => Type::IntArray,
                "FloatArray" => Type::FloatArray,
                "Array" | "List" => {
                    if let Some(inner) = args.first() {
                        Type::Array(Box::new(self.type_from_ann(inner)))
                    } else {
                        Type::Array(Box::new(Type::Int))
                    }
                }
                _ => Type::Record {
                    fields: HashMap::new(),
                    row: None,
                },
            },
            TypeAnn::Tuple(items, _) => {
                Type::Tuple(items.iter().map(|i| self.type_from_ann(i)).collect())
            }
            TypeAnn::Fn { params, ret, .. } => Type::Fn {
                params: params.iter().map(|p| self.type_from_ann(p)).collect(),
                ret: Box::new(self.type_from_ann(ret)),
            },
        }
    }
}

pub struct TypeInferrer {
    pub env: TypeEnv,
    pub val_types: HashMap<ValueId, Type>,
    pub val_ops: HashMap<ValueId, SsaOp>,
    pub globals: HashMap<String, Type>,
}

impl TypeInferrer {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            val_types: HashMap::new(),
            val_ops: HashMap::new(),
            globals: HashMap::new(),
        }
    }

    pub fn resolve_internal_attrs_based_off_type(&mut self, base_ty: &Type, field: &str) -> Option<Type> {
        match base_ty {
            Type::String => match field {
                "trim" | "trim_start" | "trim_end" | "lstrip" | "rstrip" => {
                    Some(Type::Fn { params: vec![], ret: Box::new(Type::String) })
                }
                "replace" => {
                    Some(Type::Fn { params: vec![Type::String, Type::String], ret: Box::new(Type::String) })
                }
                "split" => {
                    Some(Type::Fn { params: vec![Type::String], ret: Box::new(Type::Array(Box::new(Type::String))) })
                }
                "slice" => {
                    Some(Type::Fn { params: vec![Type::Int, Type::Int], ret: Box::new(Type::String) })
                }
                "starts_with" | "startswith" | "ends_with" | "endswith" | "contains" => {
                    Some(Type::Fn { params: vec![Type::String], ret: Box::new(Type::Bool) })
                }
                "index_of" => {
                    Some(Type::Fn { params: vec![Type::String], ret: Box::new(Type::Int) })
                }
                "repeat" => {
                    Some(Type::Fn { params: vec![Type::Int], ret: Box::new(Type::String) })
                }
                "char_at" => {
                    Some(Type::Fn { params: vec![Type::Int], ret: Box::new(Type::String) })
                }
                "to_int" => {
                    Some(Type::Fn { params: vec![], ret: Box::new(Type::Int) })
                }
                "length" => {
                    Some(Type::Int)
                }
                _ => None,
            },
            Type::Array(inner) => match field {
                "length" => Some(Type::Int),
                "push" => Some(Type::Fn { params: vec![*inner.clone()], ret: Box::new(Type::Unit) }),
                "pop" => Some(Type::Fn { params: vec![], ret: inner.clone() }),
                _ => None,
            },
            Type::IntArray => match field {
                "length" => Some(Type::Int),
                "push" => Some(Type::Fn { params: vec![Type::Int], ret: Box::new(Type::Unit) }),
                "pop" => Some(Type::Fn { params: vec![], ret: Box::new(Type::Int) }),
                _ => None,
            },
            Type::FloatArray => match field {
                "length" => Some(Type::Int),
                "push" => Some(Type::Fn { params: vec![Type::Float], ret: Box::new(Type::Unit) }),
                "pop" => Some(Type::Fn { params: vec![], ret: Box::new(Type::Float) }),
                _ => None,
            },
            Type::Float => match field {
                "is_nan" | "is_infinite" => Some(Type::Bool),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn register_module_declarations(&mut self, module: &crate::syntax::cst::Module) {
        use crate::syntax::cst::{Decl, Item};
        for item in &module.items {
            if let Item::Decl(Decl::Struct(s)) = item {
                let mut fields = HashMap::new();
                let mut param_types = Vec::new();
                for f in &s.fields {
                    let ty = self.env.type_from_ann(&f.type_ann);
                    fields.insert(f.name.clone(), ty.clone());
                    param_types.push(ty);
                }
                let struct_record = Type::Record {
                    fields,
                    row: None, // closed record for concrete nominal struct
                };
                let constructor_fn = Type::Fn {
                    params: param_types,
                    ret: Box::new(struct_record),
                };
                self.globals.insert(s.name.clone(), constructor_fn);
            }
        }
    }

    pub fn register_program_declarations(&mut self, resolved: &crate::sema::ResolvedProgram, module: &crate::syntax::cst::Module) {
        self.register_module_declarations(module);
        for imp in &resolved.imported_modules {
            self.register_module_declarations(imp);
        }
    }

    pub fn infer_function(&mut self, func: &SsaFunction) -> Result<HashMap<ValueId, Type>, TypeError> {
        self.val_types.clear();

        // 1. Assign type variables to all SSA values
        for val in &func.values {
            let tv = self.env.fresh_var();
            self.val_types.insert(val.id, tv);
        }

        // 2. Bind parameter type annotations and connect with global function signature
        if let Some(Type::Fn { params: sig_params, ret: sig_ret }) = self.globals.get(&func.name).cloned() {
            for (i, (_, val_id, _)) in func.params.iter().enumerate() {
                if let Some(sig_param_ty) = sig_params.get(i) {
                    let val_ty = self.val_types[val_id].clone();
                    self.env.unify(&val_ty, sig_param_ty)?;
                }
            }
            if let Some(ret_ann) = &func.ret_type {
                let ret_ty = self.env.type_from_ann(ret_ann);
                self.env.unify(&sig_ret, &ret_ty)?;
            }
        } else {
            for (_, val_id, type_ann) in &func.params {
                if let Some(ann) = type_ann {
                    let ty = self.env.type_from_ann(ann);
                    let val_ty = self.val_types[val_id].clone();
                    self.env.unify(&val_ty, &ty)?;
                }
            }
        }

        // Bind types for known constants
        for (&val_id, lit) in &func.const_values {
            let lit_ty = match lit {
                LitVal::Int(_) => Type::Int,
                LitVal::Float(_) => Type::Float,
                LitVal::String(_) => Type::String,
                LitVal::Bool(_) => Type::Bool,
                LitVal::Unit => Type::Unit,
                LitVal::Nil => Type::Nil,
            };
            if let Some(val_ty) = self.val_types.get(&val_id).cloned() {
                self.env.unify(&val_ty, &lit_ty)?;
            }
        }

        // 3. Walk basic blocks and generate constraints
        for bb in &func.blocks {
            // Phis
            for phi in &bb.phis {
                let dst_ty = self.val_types[&phi.dst].clone();
                for (_, incoming_val) in &phi.incoming {
                    let inc_ty = self.val_types[incoming_val].clone();
                    self.env.unify(&dst_ty, &inc_ty)?;
                }
            }

            // Instructions
            for inst in &bb.instructions {
                self.infer_instruction(inst)?;
            }

            // Terminator
            match &bb.terminator {
                SsaTerminator::Return(Some(val), _) => {
                    let val_ty = self.val_types[val].clone();
                    if let Some(ann) = &func.ret_type {
                        let ret_ty = self.env.type_from_ann(ann);
                        self.env.unify(&val_ty, &ret_ty)?;
                    }
                }
                SsaTerminator::Branch { cond, .. } => {
                    let cond_ty = self.val_types[cond].clone();
                    self.env.unify(&cond_ty, &Type::Bool)?;
                }
                SsaTerminator::Match { target, arms, .. } => {
                    let target_ty = self.val_types[target].clone();
                    for arm in arms {
                        self.infer_pattern(&arm.pattern, &target_ty, &arm.bindings)?;
                    }
                }
                _ => {}
            }
        }

        // 4. Resolve final types and update global function signature
        if let Some(sig) = self.globals.get(&func.name).cloned() {
            let resolved_sig = self.env.resolve(&sig);
            self.globals.insert(func.name.clone(), resolved_sig);
        }

        let mut resolved = HashMap::new();
        for (v, t) in &self.val_types {
            resolved.insert(*v, self.env.resolve(t));
        }

        Ok(resolved)
    }

    pub fn infer_program(
        &mut self,
        ssa: &SsaProgram,
    ) -> Result<HashMap<String, HashMap<ValueId, Type>>, TypeError> {
        // Phase 1: create function signature variables for all functions
        for func in &ssa.functions {
            let param_types: Vec<Type> = func
                .params
                .iter()
                .map(|(_, _, ann)| {
                    if let Some(a) = ann {
                        self.env.type_from_ann(a)
                    } else {
                        self.env.fresh_var()
                    }
                })
                .collect();
            let ret_type = if let Some(ann) = &func.ret_type {
                self.env.type_from_ann(ann)
            } else {
                self.env.fresh_var()
            };
            self.globals.insert(
                func.name.clone(),
                Type::Fn {
                    params: param_types,
                    ret: Box::new(ret_type),
                },
            );
        }

        // Phase 2: infer each function body, propagating constraints across callers
        let mut result = HashMap::new();
        for func in &ssa.functions {
            let val_map = self.infer_function(func)?;
            result.insert(func.name.clone(), val_map);
        }

        Ok(result)
    }

    fn infer_instruction(&mut self, inst: &SsaInst) -> Result<(), TypeError> {
        if let Some(d) = inst.dst {
            self.val_ops.insert(d, inst.op.clone());
        }
        let dst_ty = inst.dst.map(|d| self.val_types[&d].clone());

        match &inst.op {
            SsaOp::Const(lit) => {
                if let Some(dst) = dst_ty {
                    let lit_ty = match lit {
                        LitVal::Int(_) => Type::Int,
                        LitVal::Float(_) => Type::Float,
                        LitVal::String(_) => Type::String,
                        LitVal::Bool(_) => Type::Bool,
                        LitVal::Unit => Type::Unit,
                        LitVal::Nil => Type::Nil,
                    };
                    self.env.unify(&dst, &lit_ty)?;
                }
            }
            SsaOp::Binary(op, l, r) => {
                if let Some(dst) = dst_ty {
                    let l_ty = self.val_types[l].clone();
                    let r_ty = self.val_types[r].clone();

                    match op {
                        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
                            self.env.unify(&l_ty, &r_ty)?;
                            self.env.unify(&dst, &l_ty)?;
                        }
                        BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                            self.env.unify(&l_ty, &r_ty)?;
                            self.env.unify(&dst, &Type::Bool)?;
                        }
                        BinOp::And | BinOp::Or => {
                            self.env.unify(&l_ty, &Type::Bool)?;
                            self.env.unify(&r_ty, &Type::Bool)?;
                            self.env.unify(&dst, &Type::Bool)?;
                        }
                    }
                }
            }
            SsaOp::Unary(op, operand) => {
                if let Some(dst) = dst_ty {
                    let op_ty = self.val_types[operand].clone();
                    match op {
                        UnaryOp::Neg => {
                            self.env.unify(&dst, &op_ty)?;
                        }
                        UnaryOp::Not => {
                            self.env.unify(&op_ty, &Type::Bool)?;
                            self.env.unify(&dst, &Type::Bool)?;
                        }
                    }
                }
            }
            SsaOp::ArrayLit(items) => {
                if let Some(dst) = dst_ty {
                    if items.is_empty() {
                        let elem_ty = self.env.fresh_var();
                        self.env.unify(&dst, &Type::Array(Box::new(elem_ty)))?;
                    } else {
                        let first_ty = self.env.resolve(&self.val_types[&items[0]]);
                        let all_same = items.iter().all(|item| {
                            let resolved = self.env.resolve(&self.val_types[item]);
                            resolved == first_ty
                        });

                        if all_same && first_ty == Type::Int {
                            self.env.unify(&dst, &Type::IntArray)?;
                        } else if all_same && first_ty == Type::Float {
                            self.env.unify(&dst, &Type::FloatArray)?;
                        } else if all_same {
                            self.env.unify(&dst, &Type::Array(Box::new(first_ty)))?;
                        } else {
                            // Mixed array (e.g. [1, 2, 3.0, 4]): remains a normal mixed array!
                            // Not a FloatArray, not an IntArray!
                            let elem_ty = self.env.fresh_var();
                            self.env.unify(&dst, &Type::Array(Box::new(elem_ty)))?;
                        }
                    }
                }
            }
            SsaOp::TupleLit(items) => {
                if let Some(dst) = dst_ty {
                    let item_types: Vec<Type> = items.iter().map(|i| self.val_types[i].clone()).collect();
                    self.env.unify(&dst, &Type::Tuple(item_types))?;
                }
            }
            SsaOp::Index { base, index } => {
                if let Some(dst) = dst_ty {
                    let idx_ty = self.val_types[index].clone();
                    self.env.unify(&idx_ty, &Type::Int)?;

                    let base_ty = self.val_types[base].clone();
                    let resolved_base = self.env.resolve(&base_ty);

                    match resolved_base {
                        Type::IntArray => {
                            self.env.unify(&dst, &Type::Int)?;
                        }
                        Type::FloatArray => {
                            self.env.unify(&dst, &Type::Float)?;
                        }
                        Type::Array(inner) => {
                            self.env.unify(&dst, &inner)?;
                        }
                        _ => {
                            // Duck typing: base is ArrayLike
                            let elem_ty = self.env.fresh_var();
                            self.env.unify(&base_ty, &Type::Array(Box::new(elem_ty.clone())))?;
                            self.env.unify(&dst, &elem_ty)?;
                        }
                    }
                }
            }
            SsaOp::FieldAccess { base, field } => {
                if let Some(dst) = dst_ty {
                    let base_ty = self.val_types[base].clone();
                    let resolved_base = self.env.resolve(&base_ty);
                    if let Some(attr_ty) = self.resolve_internal_attrs_based_off_type(&resolved_base, field) {
                        self.env.unify(&dst, &attr_ty)?;
                    } else {
                        let row = self.env.fresh_row();
                        let mut fields = HashMap::new();
                        fields.insert(field.clone(), dst);
                        // Duck typing: base has field
                        self.env.unify(&base_ty, &Type::Record { fields, row: Some(row) })?;
                    }
                }
            }
            SsaOp::StoreField { base, field, val } => {
                let base_ty = self.val_types[base].clone();
                let val_ty = self.val_types[val].clone();
                let row = self.env.fresh_row();
                let mut fields = HashMap::new();
                fields.insert(field.clone(), val_ty);
                self.env.unify(&base_ty, &Type::Record { fields, row: Some(row) })?;
            }
            SsaOp::BuildString(_) => {
                if let Some(dst) = dst_ty {
                    self.env.unify(&dst, &Type::String)?;
                }
            }
            SsaOp::Spawn { callee, args } => {
                let callee_ty = self.val_types[callee].clone();
                let arg_types: Vec<Type> = args.iter().map(|a| self.val_types[a].clone()).collect();
                let ret_ty = self.env.fresh_var();
                let fn_ty = Type::Fn {
                    params: arg_types,
                    ret: Box::new(ret_ty),
                };
                self.env.unify(&callee_ty, &fn_ty)?;
                if let Some(dst) = dst_ty {
                    self.env.unify(&dst, &Type::Int)?;
                }
            }
            SsaOp::Send { receiver, .. } => {
                let r_ty = self.val_types[receiver].clone();
                self.env.unify(&r_ty, &Type::Int)?;
                if let Some(dst) = dst_ty {
                    self.env.unify(&dst, &Type::Unit)?;
                }
            }
            SsaOp::Receive => {}
            SsaOp::Call { callee, args } => {
                let builtin = if let Some(SsaOp::GlobalRef(name)) = self.val_ops.get(callee) {
                    crate::builtins::Builtins::get(name)
                } else {
                    None
                };

                if let Some(builtin) = builtin {
                    if !builtin.is_variadic {
                        if args.len() != builtin.param_types.len() {
                            return Err(TypeError::ArityMismatch {
                                name: builtin.name.to_string(),
                                expected: builtin.param_types.len(),
                                found: args.len(),
                            });
                        }
                        for (arg, expected_ty) in args.iter().zip(builtin.param_types.iter()) {
                            let arg_ty = self.val_types[arg].clone();
                            self.env.unify(&arg_ty, expected_ty)?;
                        }
                    }
                    if let Some(dst) = dst_ty {
                        let ret_ty = if let Type::Var(_) = &builtin.ret_type {
                            self.env.fresh_var()
                        } else {
                            builtin.ret_type.clone()
                        };
                        self.env.unify(&dst, &ret_ty)?;
                    }
                } else if let Some(dst) = dst_ty {
                    let callee_ty = self.val_types[callee].clone();
                    let arg_types: Vec<Type> = args.iter().map(|a| self.val_types[a].clone()).collect();
                    let fn_ty = Type::Fn {
                        params: arg_types,
                        ret: Box::new(dst),
                    };
                    self.env.unify(&callee_ty, &fn_ty)?;
                }
            }
            SsaOp::Atom(tag) => {
                if let Some(dst) = dst_ty {
                    let row = self.env.fresh_row();
                    let mut cases = HashMap::new();
                    cases.insert(tag.clone(), vec![]);
                    self.env.unify(&dst, &Type::Variant { cases, row: Some(row) })?;
                }
            }
            SsaOp::GlobalRef(name) => {
                if let Some(dst) = dst_ty {
                    if let Some(builtin) = crate::builtins::Builtins::get(name) {
                        if !builtin.is_variadic {
                            let ret_ty = if let Type::Var(_) = &builtin.ret_type {
                                self.env.fresh_var()
                            } else {
                                builtin.ret_type.clone()
                            };
                            let fn_ty = Type::Fn {
                                params: builtin.param_types.clone(),
                                ret: Box::new(ret_ty),
                            };
                            self.env.unify(&dst, &fn_ty)?;
                        }
                    } else if let Some(global_ty) = self.globals.get(name).cloned() {
                        self.env.unify(&dst, &global_ty)?;
                    }
                }
            }
            SsaOp::QualifiedGlobal(path) => {
                if let Some(dst) = dst_ty {
                    let fn_name = crate::codegen::emitter::resolve_qualified_name(path);
                    if let Some(builtin) = crate::builtins::Builtins::get(&fn_name) {
                        if !builtin.is_variadic {
                            let ret_ty = if let Type::Var(_) = &builtin.ret_type {
                                self.env.fresh_var()
                            } else {
                                builtin.ret_type.clone()
                            };
                            let fn_ty = Type::Fn {
                                params: builtin.param_types.clone(),
                                ret: Box::new(ret_ty),
                            };
                            self.env.unify(&dst, &fn_ty)?;
                        }
                    } else if let Some(global_ty) = self.globals.get(&fn_name).cloned() {
                        self.env.unify(&dst, &global_ty)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn infer_pattern(
        &mut self,
        pat: &PatternNode,
        target_ty: &Type,
        bindings: &[(String, ValueId)],
    ) -> Result<(), TypeError> {
        match pat {
            PatternNode::Literal(lit, _) => {
                let lit_ty = match lit {
                    LitVal::Int(_) => Type::Int,
                    LitVal::Float(_) => Type::Float,
                    LitVal::String(_) => Type::String,
                    LitVal::Bool(_) => Type::Bool,
                    LitVal::Unit => Type::Unit,
                    LitVal::Nil => Type::Nil,
                };
                self.env.unify(target_ty, &lit_ty)?;
            }
            PatternNode::Bind(name, _) => {
                if let Some((_, val_id)) = bindings.iter().find(|(n, _)| n == name) {
                    let val_ty = self.val_types[val_id].clone();
                    self.env.unify(target_ty, &val_ty)?;
                }
            }
            PatternNode::Tuple(pats, _) => {
                let pat_types: Vec<Type> = (0..pats.len()).map(|_| self.env.fresh_var()).collect();
                self.env.unify(target_ty, &Type::Tuple(pat_types.clone()))?;
                for (p, t) in pats.iter().zip(pat_types.iter()) {
                    self.infer_pattern(p, t, bindings)?;
                }
            }
            PatternNode::EnumVariant { name, args, .. } => {
                let raw_name = name.strip_prefix(':').unwrap_or(name);
                let row = self.env.fresh_row();
                let arg_types: Vec<Type> = (0..args.len()).map(|_| self.env.fresh_var()).collect();
                let mut cases = HashMap::new();
                cases.insert(raw_name.to_string(), arg_types.clone());

                self.env.unify(target_ty, &Type::Variant { cases, row: Some(row) })?;
                for (arg_pat, t) in args.iter().zip(arg_types.iter()) {
                    self.infer_pattern(arg_pat, t, bindings)?;
                }
            }
            // ── IntRange and FloatRange Resolution in Patterns ────────────────
            PatternNode::Range { start, end, inclusive: _, .. } => {
                let is_float = match (start, end) {
                    (Some(s), _) => matches!(s.as_ref(), crate::syntax::cst::Expr::Literal(LitVal::Float(_), _)),
                    (_, Some(e)) => matches!(e.as_ref(), crate::syntax::cst::Expr::Literal(LitVal::Float(_), _)),
                    _ => false,
                };

                if is_float {
                    self.env.unify(target_ty, &Type::Float)?;
                } else {
                    self.env.unify(target_ty, &Type::Int)?;
                }
            }
            // ── IntArray, FloatArray, and Array Resolution in Patterns ────────
            PatternNode::Array { segments, .. } => {
                let elem_ty = self.env.fresh_var();

                // Check target type
                let resolved_target = self.env.resolve(target_ty);
                let is_int_arr = resolved_target == Type::IntArray;
                let is_float_arr = resolved_target == Type::FloatArray;

                if is_int_arr {
                    self.env.unify(&elem_ty, &Type::Int)?;
                } else if is_float_arr {
                    self.env.unify(&elem_ty, &Type::Float)?;
                } else {
                    self.env.unify(target_ty, &Type::Array(Box::new(elem_ty.clone())))?;
                }

                for seg in segments {
                    match seg {
                        PatternSegment::Element(p) => {
                            self.infer_pattern(p, &elem_ty, bindings)?;
                        }
                        PatternSegment::BindList(name, _) => {
                            if let Some((_, val_id)) = bindings.iter().find(|(n, _)| n == name) {
                                let slice_ty = self.val_types[val_id].clone();
                                if is_int_arr {
                                    self.env.unify(&slice_ty, &Type::IntArray)?;
                                } else if is_float_arr {
                                    self.env.unify(&slice_ty, &Type::FloatArray)?;
                                } else {
                                    self.env.unify(&slice_ty, target_ty)?;
                                }
                            }
                        }
                        PatternSegment::WildcardList(_) => {}
                    }
                }
            }
            PatternNode::Wildcard(_) => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::build_ssa_program;
    use crate::sema::resolve;
    use crate::syntax::parse;

    #[test]
    fn test_infer_int_array_specialization() {
        let src = r#"
        fn test(): {
            let arr = [10, 20, 30];
            return arr[0];
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = TypeInferrer::new();
        let types = inferrer.infer_function(&ssa.functions[0]).expect("inferred");

        // The array literal must resolve to Type::IntArray!
        let has_int_array = types.values().any(|t| *t == Type::IntArray);
        assert!(has_int_array, "Array [10, 20, 30] must infer as specialized Type::IntArray!");
    }

    #[test]
    fn test_infer_float_array_specialization() {
        let src = r#"
        fn test(): {
            let arr = [1.5, 2.5, 3.5];
            return arr[1];
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = TypeInferrer::new();
        let types = inferrer.infer_function(&ssa.functions[0]).expect("inferred");

        // The float array literal must resolve to Type::FloatArray!
        let has_float_array = types.values().any(|t| *t == Type::FloatArray);
        assert!(has_float_array, "Array [1.5, 2.5, 3.5] must infer as specialized Type::FloatArray!");
    }

    #[test]
    fn test_infer_duck_typing_row_polymorphic_record() {
        let src = r#"
        fn get_x(p): {
            return p.x;
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = TypeInferrer::new();
        let types = inferrer.infer_function(&ssa.functions[0]).expect("inferred");

        // Parameter p must have an open record type with field x!
        let p_val = ssa.functions[0].params[0].1;
        let p_ty = &types[&p_val];
        match p_ty {
            Type::Record { fields, row } => {
                assert!(fields.contains_key("x"));
                assert!(row.is_some(), "Row variable must be present for duck typing!");
            }
            other => panic!("Expected row polymorphic Record type, got {:?}", other),
        }
    }

    #[test]
    fn test_infer_range_and_array_decomposition_patterns() {
        let src = r#"
        fn check_num(n): {
            match n: {
                1..10 => 100,
                _ => 0,
            }
        }

        fn check_arr(arr: IntArray): {
            match arr: {
                [..a | b] => b,
                _ => 0,
            }
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);

        let mut inferrer1 = TypeInferrer::new();
        let types1 = inferrer1.infer_function(&ssa.functions[0]).expect("inferred");
        // Parameter n must infer to Type::Int because of 1..10 range!
        let n_val = ssa.functions[0].params[0].1;
        assert_eq!(types1[&n_val], Type::Int);

        let mut inferrer2 = TypeInferrer::new();
        let types2 = inferrer2.infer_function(&ssa.functions[1]).expect("inferred");
        // Parameter arr is IntArray
        let arr_val = ssa.functions[1].params[0].1;
        assert_eq!(types2[&arr_val], Type::IntArray);
    }

    #[test]
    fn test_qualified_point_passed_through_duck_typing_function() {
        let src = r#"
        struct Point: {
            x: Int,
            y: Int,
        }

        fn get_x(p): {
            return p.x;
        }

        fn caller(): {
            let pt = Point(10, 20);
            return get_x(pt);
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);

        let mut inferrer = TypeInferrer::new();
        inferrer.register_module_declarations(&module);

        // Infer get_x
        let get_x_fn = ssa
            .functions
            .iter()
            .find(|f| f.name.ends_with(".get_x"))
            .unwrap();
        let types_get_x = inferrer.infer_function(get_x_fn).expect("inferred get_x");
        let p_param = get_x_fn.params[0].1;
        match &types_get_x[&p_param] {
            Type::Record { fields, row } => {
                assert!(fields.contains_key("x"));
                assert!(row.is_some());
            }
            other => panic!("expected record, got {:?}", other),
        }

        // Register get_x in inferrer globals
        let p_ty = types_get_x[&p_param].clone();
        let ret_ty = types_get_x[&get_x_fn.blocks[0].instructions[0].dst.unwrap()].clone();
        inferrer.globals.insert(get_x_fn.name.clone(), Type::Fn {
            params: vec![p_ty],
            ret: Box::new(ret_ty),
        });

        // Infer caller passing concrete Point
        let caller_fn = ssa
            .functions
            .iter()
            .find(|f| f.name.ends_with(".caller"))
            .unwrap();
        let caller_types = inferrer.infer_function(caller_fn).expect("inferred caller with Point");
        assert!(caller_types.values().any(|t| *t == Type::Int));
    }

    #[test]
    fn test_missing_field_rejected_at_compile_time() {
        let src = r#"
        struct Color: {
            r: Int,
            g: Int,
            b: Int,
        }

        fn get_x(p): {
            return p.x;
        }

        fn caller(): {
            let col = Color(255, 0, 0);
            return get_x(col);
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);

        let mut inferrer = TypeInferrer::new();
        inferrer.register_module_declarations(&module);

        let get_x_fn = ssa
            .functions
            .iter()
            .find(|f| f.name.ends_with(".get_x"))
            .unwrap();
        let types_get_x = inferrer.infer_function(get_x_fn).expect("inferred get_x");
        let p_param = get_x_fn.params[0].1;
        let p_ty = types_get_x[&p_param].clone();
        let ret_ty = types_get_x[&get_x_fn.blocks[0].instructions[0].dst.unwrap()].clone();
        inferrer.globals.insert(get_x_fn.name.clone(), Type::Fn {
            params: vec![p_ty],
            ret: Box::new(ret_ty),
        });

        let caller_fn = ssa
            .functions
            .iter()
            .find(|f| f.name.ends_with(".caller"))
            .unwrap();
        let res = inferrer.infer_function(caller_fn);
        assert!(res.is_err(), "Passing Color (missing field x) must fail type checking!");
        match res.unwrap_err() {
            TypeError::MissingField { field, .. } => assert_eq!(field, "x"),
            other => panic!("expected MissingField, got {:?}", other),
        }
    }
}


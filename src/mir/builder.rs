use super::ssa::*;
use crate::sema::{ResolvedProgram, ResolvedVar};
use crate::syntax::cst::*;
use crate::syntax::token::Span;
use std::collections::{HashMap, HashSet};

pub struct LoopCtx {
    pub header_bb: BlockId,
    pub exit_bb: BlockId,
    pub break_bbs: Vec<BlockId>,
}

pub struct SsaBuilder<'a> {
    pub resolved: &'a ResolvedProgram,
    pub compiled_name: String,
    pub current_func: SsaFunction,
    pub current_bb: BlockId,
    pub var_defs: HashMap<BlockId, HashMap<String, ValueId>>,
    pub loop_stack: Vec<LoopCtx>,
    pub nested_functions: Vec<SsaFunction>,
    pub env_cache: HashMap<Vec<ValueId>, ValueId>,
}

impl<'a> SsaBuilder<'a> {
    pub fn new(
        resolved: &'a ResolvedProgram,
        name: String,
        ret_type: Option<TypeAnn>,
        span: Span,
    ) -> Self {
        let current_func = SsaFunction::new(name.clone(), ret_type, span);
        let entry_bb = current_func.entry_block;
        let mut var_defs = HashMap::new();
        var_defs.insert(entry_bb, HashMap::new());

        Self {
            resolved,
            compiled_name: name.clone(),
            current_func,
            current_bb: entry_bb,
            var_defs,
            loop_stack: Vec::new(),
            nested_functions: Vec::new(),
            env_cache: HashMap::new(),
        }
    }

    pub fn build_function_with_nested(
        resolved: &'a ResolvedProgram,
        func_def: &FnDef,
        out_functions: &mut Vec<SsaFunction>,
    ) -> SsaFunction {
        let compiled_name = resolved
            .functions
            .iter()
            .find(|function| function.name == func_def.name && function.span == func_def.span)
            .map(|function| function.compiled_name.clone())
            .unwrap_or_else(|| func_def.name.clone());
        Self::build_function_with_compiled_name(
            resolved,
            func_def,
            compiled_name,
            out_functions,
        )
    }

    fn build_function_with_compiled_name(
        resolved: &'a ResolvedProgram,
        func_def: &FnDef,
        compiled_name: String,
        out_functions: &mut Vec<SsaFunction>,
    ) -> SsaFunction {
        let mut builder = SsaBuilder::new(
            resolved,
            compiled_name,
            func_def.ret_type.clone(),
            func_def.span,
        );

        // Bind function parameters
        for param in &func_def.params {
            let val = builder.current_func.new_value(Some(param.name.clone()), param.span);
            builder.current_func.params.push((param.name.clone(), val, param.type_ann.clone()));
            builder.set_var(&param.name, val);
        }

        // Lower function body
        builder.lower_block(&func_def.body);

        // Ensure current block has a terminator
        if !builder.current_func.block(builder.current_bb).is_terminated() {
            builder.current_func.block_mut(builder.current_bb).terminator = SsaTerminator::Return(None, func_def.span);
        }

        out_functions.extend(builder.nested_functions);
        builder.current_func
    }

    pub fn build_function(
        resolved: &'a ResolvedProgram,
        func_def: &FnDef,
    ) -> SsaFunction {
        let mut extra = Vec::new();
        Self::build_function_with_nested(resolved, func_def, &mut extra)
    }

    fn set_var(&mut self, name: &str, val: ValueId) {
        self.var_defs
            .entry(self.current_bb)
            .or_default()
            .insert(name.to_string(), val);
    }

    fn get_var(&self, name: &str) -> Option<ValueId> {
        self.var_defs
            .get(&self.current_bb)
            .and_then(|defs| defs.get(name).copied())
    }

    fn new_block(&mut self, label: impl Into<String>) -> BlockId {
        let id = self.current_func.new_block(label);
        self.var_defs.insert(id, HashMap::new());
        id
    }

    fn emit_inst(&mut self, op: SsaOp, name: Option<String>, span: Span) -> ValueId {
        let dst = self.current_func.new_value(name, span);
        self.current_func.block_mut(self.current_bb).instructions.push(SsaInst {
            dst: Some(dst),
            op,
            span,
        });
        dst
    }

    fn emit_void_inst(&mut self, op: SsaOp, span: Span) {
        self.current_func.block_mut(self.current_bb).instructions.push(SsaInst {
            dst: None,
            op,
            span,
        });
    }

    pub fn lower_block(&mut self, block: &Block) -> Option<ValueId> {
        for stmt in &block.stmts {
            self.lower_stmt(stmt);
            if self.current_func.block(self.current_bb).is_terminated() {
                break;
            }
        }

        if let Some(tail) = &block.tail_expr {
            if !self.current_func.block(self.current_bb).is_terminated() {
                return Some(self.lower_expr(tail));
            }
        }

        None
    }

    pub fn lower_stmt(&mut self, stmt: &Stmt) {
        if self.current_func.block(self.current_bb).is_terminated() {
            return;
        }

        match stmt {
            Stmt::Let { name, init, .. } | Stmt::Const { name, init, .. } => {
                let val = self.lower_expr(init);
                self.set_var(name, val);
            }
            Stmt::Assign { target, val, span } => {
                let rhs_val = self.lower_expr(val);
                match target {
                    Expr::Ident(name, target_span) => {
                        if let Some(ResolvedVar::Upvalue { upvalue_index, .. }) = self.resolved.var_resolutions.get(target_span) {
                            self.emit_void_inst(SsaOp::StoreUpvalue { index: *upvalue_index, val: rhs_val }, *span);
                        }
                        self.set_var(name, rhs_val);
                    }
                    Expr::FieldAccess { base, field, .. } => {
                        let base_val = self.lower_expr(base);
                        self.emit_void_inst(
                            SsaOp::StoreField {
                                base: base_val,
                                field: field.clone(),
                                val: rhs_val,
                            },
                            *span,
                        );
                    }
                    Expr::Index { base, index, .. } => {
                        let base_val = self.lower_expr(base);
                        let idx_val = self.lower_expr(index);
                        self.emit_void_inst(
                            SsaOp::StoreIndex {
                                base: base_val,
                                index: idx_val,
                                val: rhs_val,
                            },
                            *span,
                        );
                    }
                    _ => {}
                }
            }
            Stmt::AugAssign { target, op, val, span } => {
                let rhs_val = self.lower_expr(val);
                match target {
                    Expr::Ident(name, target_span) => {
                        let old_val = self.lower_expr(&Expr::Ident(name.clone(), *target_span));
                        let new_val = self.emit_inst(SsaOp::Binary(*op, old_val, rhs_val), Some(name.clone()), *span);
                        if let Some(ResolvedVar::Upvalue { upvalue_index, .. }) = self.resolved.var_resolutions.get(target_span) {
                            self.emit_void_inst(SsaOp::StoreUpvalue { index: *upvalue_index, val: new_val }, *span);
                        }
                        self.set_var(name, new_val);
                    }
                    _ => {}
                }
            }
            Stmt::While { cond, body, span } => {
                let pre_loop_bb = self.current_bb;
                let header_bb = self.new_block("while_hdr");
                let body_bb = self.new_block("while_body");
                let exit_bb = self.new_block("while_exit");

                // Jump from pre-loop to header
                self.current_func.block_mut(pre_loop_bb).terminator = SsaTerminator::Goto(header_bb, *span);
                self.current_func.block_mut(header_bb).predecessors.push(pre_loop_bb);

                // Find variables potentially modified in while body
                let modified_vars = self.collect_assigned_vars_in_block(body);

                // Create phis in header for modified variables
                let mut phi_placeholders = HashMap::new();
                for var_name in &modified_vars {
                    if let Some(incoming_val) = self.var_defs[&pre_loop_bb].get(var_name).copied() {
                        let phi_dst = self.current_func.new_value(Some(var_name.clone()), *span);
                        phi_placeholders.insert(var_name.clone(), (phi_dst, incoming_val));
                    }
                }

                // In header_bb, set variable values to phi destinations
                let mut header_defs = self.var_defs[&pre_loop_bb].clone();
                for (var_name, (phi_dst, _)) in &phi_placeholders {
                    header_defs.insert(var_name.clone(), *phi_dst);
                }
                self.var_defs.insert(header_bb, header_defs);

                // Lower condition in header_bb
                self.current_bb = header_bb;
                let cond_val = self.lower_expr(cond);
                self.current_func.block_mut(header_bb).terminator = SsaTerminator::Branch {
                    cond: cond_val,
                    then_bb: body_bb,
                    else_bb: exit_bb,
                    span: *span,
                };
                self.current_func.block_mut(body_bb).predecessors.push(header_bb);
                self.current_func.block_mut(exit_bb).predecessors.push(header_bb);

                // Lower loop body in body_bb
                self.current_bb = body_bb;
                let body_init_defs = self.var_defs[&header_bb].clone();
                self.var_defs.insert(body_bb, body_init_defs);

                self.loop_stack.push(LoopCtx { header_bb, exit_bb, break_bbs: Vec::new() });
                self.lower_block(body);
                let _loop_ctx = self.loop_stack.pop().unwrap();

                let body_end_bb = self.current_bb;
                if !self.current_func.block(body_end_bb).is_terminated() {
                    self.current_func.block_mut(body_end_bb).terminator = SsaTerminator::Goto(header_bb, *span);
                    self.current_func.block_mut(header_bb).predecessors.push(body_end_bb);
                }

                // Complete the phis in header_bb with back-edge values
                for (var_name, (phi_dst, pre_val)) in phi_placeholders {
                    let back_val = self.var_defs[&body_end_bb].get(&var_name).copied().unwrap_or(pre_val);
                    self.current_func.block_mut(header_bb).phis.push(Phi {
                        dst: phi_dst,
                        incoming: vec![(pre_loop_bb, pre_val), (body_end_bb, back_val)],
                        span: *span,
                    });
                }

                // In exit_bb, inherit definitions from header_bb
                let exit_defs = self.var_defs[&header_bb].clone();
                self.var_defs.insert(exit_bb, exit_defs);
                self.current_bb = exit_bb;
            }
            Stmt::Loop { body, span } => {
                let pre_loop_bb = self.current_bb;
                let body_bb = self.new_block("loop_body");
                let exit_bb = self.new_block("loop_exit");

                self.current_func.block_mut(pre_loop_bb).terminator = SsaTerminator::Goto(body_bb, *span);
                self.current_func.block_mut(body_bb).predecessors.push(pre_loop_bb);

                self.current_bb = body_bb;
                let body_defs = self.var_defs[&pre_loop_bb].clone();
                self.var_defs.insert(body_bb, body_defs);

                self.loop_stack.push(LoopCtx { header_bb: body_bb, exit_bb, break_bbs: Vec::new() });
                self.lower_block(body);
                let _loop_ctx = self.loop_stack.pop().unwrap();

                if !self.current_func.block(self.current_bb).is_terminated() {
                    self.current_func.block_mut(self.current_bb).terminator = SsaTerminator::Goto(body_bb, *span);
                    self.current_func.block_mut(body_bb).predecessors.push(self.current_bb);
                }

                self.current_bb = exit_bb;
            }
            Stmt::Break(span) => {
                if let Some(ctx) = self.loop_stack.last_mut() {
                    let exit_bb = ctx.exit_bb;
                    let break_bb = self.current_bb;
                    ctx.break_bbs.push(break_bb);
                    self.current_func.block_mut(break_bb).terminator = SsaTerminator::Goto(exit_bb, *span);
                    self.current_func.block_mut(exit_bb).predecessors.push(break_bb);
                    self.current_bb = self.new_block("dead");
                }
            }
            Stmt::Continue(span) => {
                if let Some(ctx) = self.loop_stack.last() {
                    let header_bb = ctx.header_bb;
                    self.current_func.block_mut(self.current_bb).terminator = SsaTerminator::Goto(header_bb, *span);
                    self.current_func.block_mut(header_bb).predecessors.push(self.current_bb);
                    self.current_bb = self.new_block("dead");
                }
            }
            Stmt::Return { val, span } => {
                let ret_val = val.as_ref().map(|v| self.lower_expr(v));
                self.current_func.block_mut(self.current_bb).terminator = SsaTerminator::Return(ret_val, *span);
                self.current_bb = self.new_block("dead");
            }
            Stmt::Expr(e) => {
                self.lower_expr(e);
            }
        }
    }

    pub fn lower_expr(&mut self, expr: &Expr) -> ValueId {
        let span = expr.span();
        match expr {
            Expr::Literal(lit, _) => {
                let val = self.current_func.new_value(None, span);
                self.current_func.const_values.insert(val, lit.clone());
                val
            }
            Expr::InterpolatedString(parts, _) => {
                let mut part_vals = Vec::new();
                for part in parts {
                    match part {
                        crate::syntax::cst::StringPart::Literal(s) => {
                            part_vals.push(crate::mir::ssa::StringPartValue::Literal(s.clone()));
                        }
                        crate::syntax::cst::StringPart::Expr(e) => {
                            let v = self.lower_expr(e);
                            part_vals.push(crate::mir::ssa::StringPartValue::Value(v));
                        }
                    }
                }
                self.emit_inst(SsaOp::BuildString(part_vals), None, span)
            }
            Expr::Ident(name, _) => {
                if let Some(val) = self.get_var(name) {
                    val
                } else if let Some(lit) = self.resolved.top_level_consts.get(name) {
                    let val = self.current_func.new_value(Some(name.clone()), span);
                    self.current_func.const_values.insert(val, lit.clone());
                    val
                } else if let Some(res) = self.resolved.var_resolutions.get(&span) {
                    match res {
                        ResolvedVar::Upvalue { upvalue_index, .. } => {
                            self.emit_inst(SsaOp::Upvalue(*upvalue_index), Some(name.clone()), span)
                        }
                        ResolvedVar::Global { name, .. } => {
                            self.emit_inst(SsaOp::GlobalRef(name.clone()), Some(name.clone()), span)
                        }
                        ResolvedVar::This { .. } => {
                            self.emit_inst(SsaOp::This, Some(name.clone()), span)
                        }
                        ResolvedVar::EnumVariant { variant_name, .. } => {
                            self.emit_inst(SsaOp::Atom(variant_name.clone()), Some(variant_name.clone()), span)
                        }
                        _ => self.emit_inst(SsaOp::GlobalRef(name.clone()), Some(name.clone()), span),
                    }
                } else {
                    self.emit_inst(SsaOp::GlobalRef(name.clone()), Some(name.clone()), span)
                }
            }
            Expr::Atom(name, _) => {
                let raw_name = name.strip_prefix(':').unwrap_or(name);
                self.emit_inst(SsaOp::Atom(raw_name.to_string()), Some(raw_name.to_string()), span)
            }
            Expr::Binary { op, left, right, .. } => {
                let l_val = self.lower_expr(left);
                let r_val = self.lower_expr(right);
                if let (Some(l_lit), Some(r_lit)) = (
                    self.current_func.const_values.get(&l_val).cloned(),
                    self.current_func.const_values.get(&r_val).cloned(),
                ) {
                    if let Some(folded) = eval_binary_const(*op, &l_lit, &r_lit) {
                        let dst = self.current_func.new_value(None, span);
                        self.current_func.const_values.insert(dst, folded);
                        return dst;
                    }
                }
                self.emit_inst(SsaOp::Binary(*op, l_val, r_val), None, span)
            }
            Expr::Unary { op, operand, .. } => {
                let val = self.lower_expr(operand);
                if let Some(lit) = self.current_func.const_values.get(&val).cloned() {
                    if let Some(folded) = eval_unary_const(*op, &lit) {
                        let dst = self.current_func.new_value(None, span);
                        self.current_func.const_values.insert(dst, folded);
                        return dst;
                    }
                }
                self.emit_inst(SsaOp::Unary(*op, val), None, span)
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident(_, callee_span) = callee.as_ref() {
                    if let Some(ResolvedVar::Global { name, .. }) =
                        self.resolved.var_resolutions.get(callee_span)
                    {
                        if let Some(struct_def) = self.resolved.structs.iter().find(|s| s.compiled_name == *name) {
                            let arg_vals: Vec<ValueId> =
                                args.iter().map(|arg| self.lower_expr(arg)).collect();
                            return self.emit_inst(
                                SsaOp::NewObject {
                                    name: struct_def.name.clone(),
                                    prototype: struct_def.compiled_name.clone(),
                                    fields: struct_def
                                        .fields
                                        .iter()
                                        .map(|field| (field.name.clone(), field.type_ann.clone()))
                                        .collect(),
                                    args: arg_vals,
                                },
                                None,
                                span,
                            );
                        }
                    }
                }
                let callee_val = self.lower_expr(callee);
                let arg_vals: Vec<ValueId> = args.iter().map(|a| self.lower_expr(a)).collect();
                self.emit_inst(SsaOp::Call { callee: callee_val, args: arg_vals }, None, span)
            }
            Expr::FieldAccess { base, field, .. } => {
                if let Some(ResolvedVar::QualifiedGlobal { path, .. }) = self.resolved.var_resolutions.get(&span) {
                    self.emit_inst(SsaOp::QualifiedGlobal(path.clone()), Some(field.clone()), span)
                } else {
                    let base_val = self.lower_expr(base);
                    self.emit_inst(SsaOp::FieldAccess { base: base_val, field: field.clone() }, Some(field.clone()), span)
                }
            }
            Expr::Index { base, index, .. } => {
                let base_val = self.lower_expr(base);
                let idx_val = self.lower_expr(index);
                self.emit_inst(SsaOp::Index { base: base_val, index: idx_val }, None, span)
            }
            Expr::ArrayLit(items, _) => {
                let item_vals: Vec<ValueId> = items.iter().map(|i| self.lower_expr(i)).collect();
                self.emit_inst(SsaOp::ArrayLit(item_vals), None, span)
            }
            Expr::TupleLit(items, _) => {
                let item_vals: Vec<ValueId> = items.iter().map(|i| self.lower_expr(i)).collect();
                self.emit_inst(SsaOp::TupleLit(item_vals), None, span)
            }
            Expr::Block(block) => {
                self.lower_block(block).unwrap_or_else(|| {
                    let val = self.current_func.new_value(None, span);
                    self.current_func.const_values.insert(val, LitVal::Unit);
                    val
                })
            }
            Expr::If { cond, then_branch, else_branch, .. } => {
                let cond_val = self.lower_expr(cond);
                let pre_if_bb = self.current_bb;
                let then_bb = self.new_block("if_then");
                let else_bb = self.new_block("if_else");
                let merge_bb = self.new_block("if_merge");

                self.current_func.block_mut(pre_if_bb).terminator = SsaTerminator::Branch {
                    cond: cond_val,
                    then_bb,
                    else_bb,
                    span,
                };
                self.current_func.block_mut(then_bb).predecessors.push(pre_if_bb);
                self.current_func.block_mut(else_bb).predecessors.push(pre_if_bb);

                // Lower then branch
                self.current_bb = then_bb;
                let then_init_defs = self.var_defs[&pre_if_bb].clone();
                self.var_defs.insert(then_bb, then_init_defs);
                let then_result = self.lower_block(then_branch);
                let then_end_bb = self.current_bb;
                if !self.current_func.block(then_end_bb).is_terminated() {
                    self.current_func.block_mut(then_end_bb).terminator = SsaTerminator::Goto(merge_bb, span);
                    self.current_func.block_mut(merge_bb).predecessors.push(then_end_bb);
                }

                // Lower else branch
                self.current_bb = else_bb;
                let else_init_defs = self.var_defs[&pre_if_bb].clone();
                self.var_defs.insert(else_bb, else_init_defs);
                let else_result = if let Some(eb) = else_branch {
                    self.lower_block(eb)
                } else {
                    None
                };
                let else_end_bb = self.current_bb;
                if !self.current_func.block(else_end_bb).is_terminated() {
                    self.current_func.block_mut(else_end_bb).terminator = SsaTerminator::Goto(merge_bb, span);
                    self.current_func.block_mut(merge_bb).predecessors.push(else_end_bb);
                }

                // In merge_bb, insert phis for modified variables
                let mut merge_defs = self.var_defs[&pre_if_bb].clone();
                let all_vars: HashSet<String> = self.var_defs[&then_end_bb]
                    .keys()
                    .chain(self.var_defs[&else_end_bb].keys())
                    .cloned()
                    .collect();

                for var_name in all_vars {
                    let v_then = self.var_defs[&then_end_bb].get(&var_name).copied();
                    let v_else = self.var_defs[&else_end_bb].get(&var_name).copied();
                    match (v_then, v_else) {
                        (Some(vt), Some(ve)) if vt != ve => {
                            let phi_dst = self.current_func.new_value(Some(var_name.clone()), span);
                            self.current_func.block_mut(merge_bb).phis.push(Phi {
                                dst: phi_dst,
                                incoming: vec![(then_end_bb, vt), (else_end_bb, ve)],
                                span,
                            });
                            merge_defs.insert(var_name, phi_dst);
                        }
                        (Some(vt), _) => {
                            merge_defs.insert(var_name, vt);
                        }
                        (_, Some(ve)) => {
                            merge_defs.insert(var_name, ve);
                        }
                        _ => {}
                    }
                }
                self.var_defs.insert(merge_bb, merge_defs);
                self.current_bb = merge_bb;

                // If this is an expression with values from both branches, join them
                if let (Some(vt), Some(ve)) = (then_result, else_result) {
                    let res_val = self.current_func.new_value(Some("if_res".to_string()), span);
                    self.current_func.block_mut(merge_bb).phis.push(Phi {
                        dst: res_val,
                        incoming: vec![(then_end_bb, vt), (else_end_bb, ve)],
                        span,
                    });
                    res_val
                } else {
                    then_result.or(else_result).unwrap_or_else(|| {
                        let val = self.current_func.new_value(None, span);
                        self.current_func.const_values.insert(val, LitVal::Unit);
                        val
                    })
                }
            }
            Expr::Match { target, arms, .. } => {
                let target_val = self.lower_expr(target);
                let pre_match_bb = self.current_bb;
                let default_bb = self.new_block("match_default");
                let exit_bb = self.new_block("match_exit");

                let mut ssa_arms = Vec::new();
                let mut arm_results = Vec::new();

                for arm in arms {
                    let arm_bb = self.new_block("match_arm");
                    self.current_func.block_mut(arm_bb).predecessors.push(pre_match_bb);

                    let mut arm_defs = self.var_defs[&pre_match_bb].clone();
                    let mut bindings = Vec::new();

                    // Collect pattern bindings and create SSA values for them
                    self.collect_pattern_bindings_into_ssa(&arm.pattern, &mut bindings, &mut arm_defs);
                    self.var_defs.insert(arm_bb, arm_defs);

                    ssa_arms.push(SsaMatchArm {
                        pattern: arm.pattern.clone(),
                        target_bb: arm_bb,
                        bindings,
                    });

                    // Lower arm body in arm_bb
                    self.current_bb = arm_bb;
                    let body_res = self.lower_expr(&arm.body);
                    let arm_end_bb = self.current_bb;
                    if !self.current_func.block(arm_end_bb).is_terminated() {
                        self.current_func.block_mut(arm_end_bb).terminator = SsaTerminator::Goto(exit_bb, span);
                        self.current_func.block_mut(exit_bb).predecessors.push(arm_end_bb);
                        arm_results.push((arm_end_bb, body_res));
                    }
                }

                // In default_bb: unreachable or panic
                self.current_func.block_mut(default_bb).predecessors.push(pre_match_bb);
                self.current_func.block_mut(default_bb).terminator = SsaTerminator::Unreachable(span);

                // Terminate pre_match_bb with SsaTerminator::Match
                self.current_func.block_mut(pre_match_bb).terminator = SsaTerminator::Match {
                    target: target_val,
                    arms: ssa_arms,
                    default_bb,
                    span,
                };

                // In exit_bb, if arms produced values, join with a Phi
                self.current_bb = exit_bb;
                let exit_defs = self.var_defs[&pre_match_bb].clone();
                self.var_defs.insert(exit_bb, exit_defs);

                if !arm_results.is_empty() {
                    let res_val = self.current_func.new_value(Some("match_res".to_string()), span);
                    self.current_func.block_mut(exit_bb).phis.push(Phi {
                        dst: res_val,
                        incoming: arm_results,
                        span,
                    });
                    res_val
                } else {
                    let val = self.current_func.new_value(None, span);
                    self.current_func.const_values.insert(val, LitVal::Unit);
                    val
                }
            }
            Expr::Spawn { callee, args, .. } => {
                let callee_val = self.lower_expr(callee);
                let arg_vals: Vec<ValueId> = args.iter().map(|a| self.lower_expr(a)).collect();
                self.emit_inst(SsaOp::Spawn { callee: callee_val, args: arg_vals }, None, span)
            }
            Expr::Send { receiver, message, .. } => {
                let recv_val = self.lower_expr(receiver);
                let msg_val = self.lower_expr(message);
                self.emit_inst(SsaOp::Send { receiver: recv_val, message: msg_val }, None, span)
            }
            Expr::Receive(_) => {
                self.emit_inst(SsaOp::Receive, None, span)
            }
            Expr::Lambda { params, body, span, .. } => {
                // Lambda creates a nested function in SsaProgram
                let lambda_name = format!("lambda_{}_{}", span.line, span.col);
                let compiled_name = format!("{}.{}", self.compiled_name, lambda_name);
                let lambda_fn_def = FnDef {
                    name: lambda_name.clone(),
                    type_params: vec![],
                    params: params.clone(),
                    ret_type: None,
                    body: body.clone(),
                    span: *span,
                };
                let nested_fn = SsaBuilder::build_function_with_compiled_name(
                    self.resolved,
                    &lambda_fn_def,
                    compiled_name.clone(),
                    &mut self.nested_functions,
                );
                self.nested_functions.push(nested_fn);

                let upvals = self.resolved.functions.iter()
                    .find(|f| f.compiled_name == compiled_name)
                    .map(|f| &f.upvalues);
                let mut captured_values = Vec::new();
                if let Some(upvals) = upvals {
                    for u in upvals {
                        if let Some(v) = self.get_var(&u.name) {
                            captured_values.push(v);
                        } else {
                            let v = self.emit_inst(SsaOp::Upvalue(u.parent_index), Some(u.name.clone()), *span);
                            captured_values.push(v);
                        }
                    }
                }
                if captured_values.is_empty() {
                    self.emit_inst(
                        SsaOp::GlobalRef(compiled_name.clone()),
                        Some(compiled_name),
                        *span,
                    )
                } else {
                    let env_val = if let Some(&cached_env) = self.env_cache.get(&captured_values) {
                        cached_env
                    } else {
                        let ev = self.emit_inst(
                            SsaOp::MakeEnv(captured_values.clone()),
                            Some("env".to_string()),
                            *span,
                        );
                        self.env_cache.insert(captured_values, ev);
                        ev
                    };
                    self.emit_inst(
                        SsaOp::MakeClosure {
                            func: compiled_name.clone(),
                            env: Some(env_val),
                        },
                        Some(compiled_name),
                        *span,
                    )
                }
            }
        }
    }

    fn collect_pattern_bindings_into_ssa(
        &mut self,
        pat: &PatternNode,
        bindings: &mut Vec<(String, ValueId)>,
        defs: &mut HashMap<String, ValueId>,
    ) {
        let span = pat.span();
        match pat {
            PatternNode::Bind(name, _) => {
                let val = self.current_func.new_value(Some(name.clone()), span);
                bindings.push((name.clone(), val));
                defs.insert(name.clone(), val);
            }
            PatternNode::Tuple(pats, _) => {
                for p in pats {
                    self.collect_pattern_bindings_into_ssa(p, bindings, defs);
                }
            }
            PatternNode::EnumVariant { args, .. } => {
                for arg in args {
                    self.collect_pattern_bindings_into_ssa(arg, bindings, defs);
                }
            }
            PatternNode::Array { segments, .. } => {
                for seg in segments {
                    match seg {
                        PatternSegment::Element(p) => {
                            self.collect_pattern_bindings_into_ssa(p, bindings, defs);
                        }
                        PatternSegment::BindList(name, seg_span) => {
                            let val = self.current_func.new_value(Some(name.clone()), *seg_span);
                            bindings.push((name.clone(), val));
                            defs.insert(name.clone(), val);
                        }
                        PatternSegment::WildcardList(_) => {}
                    }
                }
            }
            PatternNode::Wildcard(_) | PatternNode::Literal(_, _) | PatternNode::Range { .. } => {}
        }
    }

    fn collect_assigned_vars_in_block(&self, block: &Block) -> HashSet<String> {
        let mut set = HashSet::new();
        for stmt in &block.stmts {
            match stmt {
                Stmt::Assign { target, .. } | Stmt::AugAssign { target, .. } => {
                    if let Expr::Ident(name, _) = target {
                        set.insert(name.clone());
                    }
                }
                Stmt::While { body, .. } | Stmt::Loop { body, .. } => {
                    set.extend(self.collect_assigned_vars_in_block(body));
                }
                Stmt::Expr(expr) => {
                    self.collect_assigned_vars_in_expr(expr, &mut set);
                }
                _ => {}
            }
        }
        if let Some(ref tail) = block.tail_expr {
            self.collect_assigned_vars_in_expr(tail, &mut set);
        }
        set
    }

    fn collect_assigned_vars_in_expr(&self, expr: &Expr, set: &mut HashSet<String>) {
        match expr {
            Expr::If { then_branch, else_branch, .. } => {
                set.extend(self.collect_assigned_vars_in_block(then_branch));
                if let Some(eb) = else_branch {
                    set.extend(self.collect_assigned_vars_in_block(eb));
                }
            }
            Expr::Match { arms, .. } => {
                for arm in arms {
                    self.collect_assigned_vars_in_expr(&arm.body, set);
                }
            }
            Expr::Block(b) => {
                set.extend(self.collect_assigned_vars_in_block(b));
            }
            _ => {}
        }
    }
}

pub fn build_ssa_program(resolved: &ResolvedProgram, module: &Module) -> SsaProgram {
    let mut functions = Vec::new();

    for item in &module.items {
        match item {
            Item::Decl(Decl::Fn(f)) => {
                let mut nested = Vec::new();
                let compiled_name =
                    find_compiled_function_name(resolved, &module.path, f, None);
                let ssa_fn = SsaBuilder::build_function_with_compiled_name(
                    resolved,
                    f,
                    compiled_name,
                    &mut nested,
                );
                functions.extend(nested);
                functions.push(ssa_fn);
            }
            Item::Decl(Decl::Impl(impl_def)) => {
                for method in &impl_def.methods {
                    let mut nested = Vec::new();
                    let compiled_name =
                        find_compiled_function_name(resolved, &module.path, method, Some(impl_def));
                    let ssa_fn = SsaBuilder::build_function_with_compiled_name(
                        resolved,
                        method,
                        compiled_name,
                        &mut nested,
                    );
                    functions.extend(nested);
                    functions.push(ssa_fn);
                }
            }
            _ => {}
        }
    }

    for imp in &resolved.imported_modules {
        for item in &imp.items {
            match item {
                Item::Decl(Decl::Fn(f)) => {
                    if f.name != "main" {
                        let mut nested = Vec::new();
                        let compiled_name =
                            find_compiled_function_name(resolved, &imp.path, f, None);
                        let ssa_fn = SsaBuilder::build_function_with_compiled_name(
                            resolved,
                            f,
                            compiled_name,
                            &mut nested,
                        );
                        functions.extend(nested);
                        functions.push(ssa_fn);
                    }
                }
                Item::Decl(Decl::Impl(impl_def)) => {
                    for method in &impl_def.methods {
                        let mut nested = Vec::new();
                        let compiled_name = find_compiled_function_name(
                            resolved,
                            &imp.path,
                            method,
                            Some(impl_def),
                        );
                        let ssa_fn = SsaBuilder::build_function_with_compiled_name(
                            resolved,
                            method,
                            compiled_name,
                            &mut nested,
                        );
                        functions.extend(nested);
                        functions.push(ssa_fn);
                    }
                }
                _ => {}
            }
        }
    }

    let top_level_stmts: Vec<Stmt> = module
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Stmt(s) => Some(s.clone()),
            _ => None,
        })
        .collect();

    let has_main = module.items.iter().any(|item| match item {
        Item::Decl(Decl::Fn(f)) => f.name == "main",
        _ => false,
    });

    if !has_main && !top_level_stmts.is_empty() {
        let span = top_level_stmts[0].span();
        let main_fn_def = FnDef {
            name: "main".to_string(),
            type_params: vec![],
            params: vec![],
            ret_type: None,
            body: Block {
                stmts: top_level_stmts,
                tail_expr: None,
                span,
            },
            span,
        };
        let mut nested = Vec::new();
        let ssa_fn = SsaBuilder::build_function_with_nested(resolved, &main_fn_def, &mut nested);
        functions.extend(nested);
        functions.push(ssa_fn);
    }

    SsaProgram {
        functions,
        structs: resolved.structs.clone(),
    }
}

fn find_compiled_function_name(
    resolved: &ResolvedProgram,
    module_path: &std::path::Path,
    function: &FnDef,
    impl_def: Option<&ImplDef>,
) -> String {
    let module_path = module_path.to_string_lossy();
    let parent = if let Some(impl_def) = impl_def {
        match &impl_def.for_protocol {
            Some(proto_name)
                if proto_name.get_compiled_name() == "ToString"
                    && matches!(
                        impl_def.target_type.get_compiled_name().as_str(),
                        "Int" | "Float" | "Bool" | "String"
                    ) =>
            {
                format!(
                    "__ionvm.prelude.{}.#ToString",
                    impl_def.target_type.get_compiled_name()
                )
            }
            Some(proto_name) => format!(
                "{}.{}.#{}",
                module_path,
                impl_def.target_type.get_compiled_name(),
                proto_name.get_compiled_name()
            ),
            None => format!(
                "{}.{}",
                module_path,
                impl_def.target_type.get_compiled_name()
            ),
        }
    } else {
        module_path.to_string()
    };
    resolved
        .functions
        .iter()
        .find(|resolved_function| {
            resolved_function.name == function.name
                && resolved_function.span == function.span
        })
        .map(|resolved_function| resolved_function.compiled_name.clone())
        .unwrap_or_else(|| {
            if function.name == "main" && impl_def.is_none() {
                "main".to_string()
            } else {
                format!("{}.{}", parent, function.name)
            }
        })
}

pub fn eval_unary_const(op: UnaryOp, lit: &LitVal) -> Option<LitVal> {
    match (op, lit) {
        (UnaryOp::Not, LitVal::Bool(b)) => Some(LitVal::Bool(!b)),
        (UnaryOp::Neg, LitVal::Int(i)) => Some(LitVal::Int(-i)),
        (UnaryOp::Neg, LitVal::Float(f)) => Some(LitVal::Float(-f)),
        _ => None,
    }
}

pub fn eval_binary_const(op: BinOp, left: &LitVal, right: &LitVal) -> Option<LitVal> {
    match (op, left, right) {
        // Int arithmetic
        (BinOp::Add, LitVal::Int(l), LitVal::Int(r)) => Some(LitVal::Int(l.wrapping_add(*r))),
        (BinOp::Sub, LitVal::Int(l), LitVal::Int(r)) => Some(LitVal::Int(l.wrapping_sub(*r))),
        (BinOp::Mul, LitVal::Int(l), LitVal::Int(r)) => Some(LitVal::Int(l.wrapping_mul(*r))),
        (BinOp::Div, LitVal::Int(l), LitVal::Int(r)) if *r != 0 => Some(LitVal::Int(l / r)),
        (BinOp::Rem, LitVal::Int(l), LitVal::Int(r)) if *r != 0 => Some(LitVal::Int(l % r)),

        // Float arithmetic
        (BinOp::Add, LitVal::Float(l), LitVal::Float(r)) => Some(LitVal::Float(l + r)),
        (BinOp::Sub, LitVal::Float(l), LitVal::Float(r)) => Some(LitVal::Float(l - r)),
        (BinOp::Mul, LitVal::Float(l), LitVal::Float(r)) => Some(LitVal::Float(l * r)),
        (BinOp::Div, LitVal::Float(l), LitVal::Float(r)) => Some(LitVal::Float(l / r)),

        // Comparisons
        (BinOp::Eq, LitVal::Int(l), LitVal::Int(r)) => Some(LitVal::Bool(l == r)),
        (BinOp::NotEq, LitVal::Int(l), LitVal::Int(r)) => Some(LitVal::Bool(l != r)),
        (BinOp::Lt, LitVal::Int(l), LitVal::Int(r)) => Some(LitVal::Bool(l < r)),
        (BinOp::LtEq, LitVal::Int(l), LitVal::Int(r)) => Some(LitVal::Bool(l <= r)),
        (BinOp::Gt, LitVal::Int(l), LitVal::Int(r)) => Some(LitVal::Bool(l > r)),
        (BinOp::GtEq, LitVal::Int(l), LitVal::Int(r)) => Some(LitVal::Bool(l >= r)),

        (BinOp::Eq, LitVal::Float(l), LitVal::Float(r)) => Some(LitVal::Bool(l == r)),
        (BinOp::NotEq, LitVal::Float(l), LitVal::Float(r)) => Some(LitVal::Bool(l != r)),
        (BinOp::Lt, LitVal::Float(l), LitVal::Float(r)) => Some(LitVal::Bool(l < r)),
        (BinOp::LtEq, LitVal::Float(l), LitVal::Float(r)) => Some(LitVal::Bool(l <= r)),
        (BinOp::Gt, LitVal::Float(l), LitVal::Float(r)) => Some(LitVal::Bool(l > r)),
        (BinOp::GtEq, LitVal::Float(l), LitVal::Float(r)) => Some(LitVal::Bool(l >= r)),

        (BinOp::Eq, LitVal::Bool(l), LitVal::Bool(r)) => Some(LitVal::Bool(l == r)),
        (BinOp::NotEq, LitVal::Bool(l), LitVal::Bool(r)) => Some(LitVal::Bool(l != r)),
        (BinOp::And, LitVal::Bool(l), LitVal::Bool(r)) => Some(LitVal::Bool(*l && *r)),
        (BinOp::Or, LitVal::Bool(l), LitVal::Bool(r)) => Some(LitVal::Bool(*l || *r)),

        (BinOp::Eq, LitVal::String(l), LitVal::String(r)) => Some(LitVal::Bool(l == r)),
        (BinOp::NotEq, LitVal::String(l), LitVal::String(r)) => Some(LitVal::Bool(l != r)),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sema::resolve;
    use crate::syntax::parse;

    #[test]
    fn test_ssa_builder_simple_function() {
        let src = r#"
        fn add(a: Int, b: Int) -> Int: {
            return a + b;
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        assert_eq!(ssa.functions.len(), 1);
        let f = &ssa.functions[0];
        assert_eq!(f.name, resolved.functions[0].compiled_name);
        assert_eq!(f.params.len(), 2);
        // Display LLVM-like IR
        let ir = format!("{}", f);
        assert!(ir.contains("add"));
        assert!(ir.contains("ret"));
    }

    #[test]
    fn test_ssa_builder_if_phi() {
        let src = r#"
        fn max(a: Int, b: Int) -> Int: {
            let res = 0;
            if a > b: {
                res = a;
            } else: {
                res = b;
            }
            return res;
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let f = &ssa.functions[0];
        let ir = format!("{}", f);
        // Verify phi node was generated at if_merge
        assert!(ir.contains("phi ["));
        assert!(ir.contains("ret"));
    }

    #[test]
    fn test_ssa_builder_while_loop_phi() {
        let src = r#"
        fn count_to(limit: Int) -> Int: {
            let i = 0;
            let sum = 0;
            while i < limit: {
                sum = sum + i;
                i = i + 1;
            }
            return sum;
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let f = &ssa.functions[0];
        let ir = format!("{}", f);
        // Header must have phi nodes for i and sum
        assert!(ir.contains("while_hdr"));
        assert!(ir.contains("phi ["));
    }

    #[test]
    fn test_ssa_builder_match_array_decomposition() {
        let src = r#"
        fn head_tail_sum(arr: Array[Int]) -> Int: {
            match arr: {
                [..a | b] => b,
                [head | ..tail] => head,
                _ => 0,
            }
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let f = &ssa.functions[0];
        let ir = format!("{}", f);
        assert!(ir.contains("match"));
        assert!(ir.contains("match_arm"));
        assert!(ir.contains("phi ["));
    }
}

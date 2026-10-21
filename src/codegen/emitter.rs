use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use vmm::bytecode::instruction::{Bytecode, Instruction, OperandType};
use vmm::bytecode::opcode::Opcode;
use vmm::lang::types::{Function, FunctionType, Object, Protection, Value};
use vmm::lang::matcher::{ArrayPattern, ListSegment, Pattern};

use crate::codegen::regalloc::RegAlloc;
use crate::mir::ssa::{
    BlockId, SsaFunction, SsaInst, SsaOp, SsaProgram, SsaTerminator, StringPartValue, ValueId,
};
use crate::syntax::cst::{BinOp, LitVal, PatternNode, UnaryOp};
use crate::types::ty::Type;

fn lit_to_value(lit: &LitVal) -> Value {
    match lit {
        LitVal::Int(i) => Value::Int(*i),
        LitVal::Float(f) => Value::Float(*f),
        LitVal::Bool(b) => Value::Bool(*b),
        LitVal::String(s) => Value::String(s.clone()),
        LitVal::Nil | LitVal::Unit => Value::Nil,
    }
}

pub fn resolve_qualified_name(path: &[String]) -> String {
    match (path.get(0).map(|s| s.as_str()), path.get(1).map(|s| s.as_str())) {
        (Some("str") | Some("String"), Some(m)) => match m {
            "len" => "str_len".to_string(),
            other => format!("str_{}", other),
        },
        (Some("json"), Some(m)) => format!("json_{}", m),
        (Some("io"), Some(m)) => m.to_string(),
        (Some("std"), Some("io")) if path.len() == 3 => path[2].clone(),
        (Some("std"), Some("net")) if path.len() == 3 => path[2].clone(),
        _ => path.join("."),
    }
}

pub struct CodeEmitter {
    pub functions: Vec<Function>,
    struct_fields: HashMap<String, Vec<String>>,
}

impl CodeEmitter {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            struct_fields: HashMap::new(),
        }
    }

    pub fn emit_program(program: &SsaProgram) -> Bytecode {
        Self::emit_program_with_types(program, None)
    }

    pub fn emit_program_with_types(
        program: &SsaProgram,
        type_maps: Option<&HashMap<String, HashMap<ValueId, Type>>>,
    ) -> Bytecode {
        let mut emitter = CodeEmitter::new();
        emitter.struct_fields = program
            .structs
            .iter()
            .map(|struct_def| {
                (
                    struct_def.name.clone(),
                    struct_def.fields.iter().map(|field| field.name.clone()).collect(),
                )
            })
            .collect();
        for func in &program.functions {
            let fn_types = type_maps.and_then(|m| m.get(&func.name));
            let compiled = emitter.emit_function_with_types(func, fn_types);
            emitter.functions.push(compiled);
        }

        // Register all built-in VM functions as Native functions
        for b in crate::builtins::Builtins::all() {
            emitter.functions.push(Function::new(
                b.name.to_string(),
                vec![],
                0,
                b.param_types.len(),
                FunctionType::Native,
            ));
        }

        let function_by_name: HashMap<String, usize> = emitter
            .functions
            .iter()
            .map(|function| (function.name.clone(), function.argument_count))
            .collect();
        let object_inits = program
            .structs
            .iter()
            .map(|struct_def| {
                let mut object = Object::new(None);
                for field in &struct_def.fields {
                    object.set_property(&field.name, Value::Nil, Protection::Writeable);
                }
                for (method_name, compiled_name) in &struct_def.methods {
                    if let Some(argument_count) = function_by_name.get(compiled_name) {
                        let method_ref = Function::new(
                            compiled_name.clone(),
                            Vec::new(),
                            0,
                            *argument_count,
                            FunctionType::Regular,
                        );
                        object.set_property(
                            method_name,
                            Value::Function(Arc::new(RwLock::new(method_ref))),
                            Protection::Writeable,
                        );
                    }
                }
                (
                    struct_def.compiled_name.clone(),
                    Arc::new(RwLock::new(object)),
                )
            })
            .collect();

        Bytecode {
            functions: emitter.functions,
            entry_point: "main".to_string(),
            native_declarations: vec![],
            object_inits,
        }
    }

    pub fn emit_function(&mut self, func: &SsaFunction) -> Function {
        self.emit_function_with_types(func, None)
    }

    pub fn emit_function_with_types(
        &mut self,
        func: &SsaFunction,
        types: Option<&HashMap<ValueId, Type>>,
    ) -> Function {
        let mut instructions: Vec<Instruction> = Vec::new();
        let mut block_offsets: HashMap<BlockId, usize> = HashMap::new();
        let mut jump_fixups: Vec<(usize, usize, BlockId)> = Vec::new(); // (inst_idx, operand_idx, target_bb)

        // Pre-scan all constants and global function references in the function
        let mut const_values: HashMap<ValueId, LitVal> = func.const_values.clone();
        let mut global_fn_names: HashMap<ValueId, String> = HashMap::new();
        let mut value_ops: HashMap<ValueId, SsaOp> = HashMap::new();
        for block in &func.blocks {
            for inst in &block.instructions {
                if let Some(dst) = inst.dst {
                    value_ops.insert(dst, inst.op.clone());
                }
                match &inst.op {
                    SsaOp::Const(lit) => {
                        if let Some(val_id) = inst.dst {
                            const_values.insert(val_id, lit.clone());
                        }
                    }
                    SsaOp::GlobalRef(name) => {
                        if let Some(val_id) = inst.dst {
                            global_fn_names.insert(val_id, name.clone());
                        }
                    }
                    SsaOp::QualifiedGlobal(path) => {
                        if let Some(val_id) = inst.dst {
                            global_fn_names.insert(val_id, resolve_qualified_name(path));
                        }
                    }
                    _ => {}
                }
            }
        }

        // Iterative CCR constant folding pass beforehand during compilation
        let mut changed = true;
        while changed {
            changed = false;
            for block in &func.blocks {
                for inst in &block.instructions {
                    if let Some(dst) = inst.dst {
                        if const_values.contains_key(&dst) {
                            continue;
                        }
                        match &inst.op {
                            SsaOp::Unary(op, val) => {
                                if let Some(lit) = const_values.get(val).cloned() {
                                    if let Some(folded) = crate::mir::builder::eval_unary_const(*op, &lit) {
                                        const_values.insert(dst, folded);
                                        changed = true;
                                    }
                                }
                            }
                            SsaOp::Binary(op, left, right) => {
                                if let (Some(l_lit), Some(r_lit)) = (
                                    const_values.get(left).cloned(),
                                    const_values.get(right).cloned(),
                                ) {
                                    if let Some(folded) = crate::mir::builder::eval_binary_const(*op, &l_lit, &r_lit) {
                                        const_values.insert(dst, folded);
                                        changed = true;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Compute needed registers: do NOT allocate registers for constants!
        let mut needed = std::collections::HashSet::new();
        for block in &func.blocks {
            for phi in &block.phis {
                needed.insert(phi.dst);
            }
            for inst in &block.instructions {
                if let Some(dst) = inst.dst {
                    if !const_values.contains_key(&dst) {
                        needed.insert(dst);
                    }
                }
            }
            if let SsaTerminator::Match { arms, .. } = &block.terminator {
                for arm in arms {
                    for (_, binding) in &arm.bindings {
                        needed.insert(*binding);
                    }
                }
            }
        }
        let regalloc = RegAlloc::allocate_with_needed(func, &needed);

        // Count additional scratch registers needed for env / temp ops
        let mut max_reg = regalloc.total_registers;

        // Ensure entry block is emitted first
        let mut ordered_blocks: Vec<&crate::mir::ssa::BasicBlock> = Vec::new();
        if let Some(entry) = func.blocks.iter().find(|b| b.id == func.entry_block) {
            ordered_blocks.push(entry);
        }
        for block in &func.blocks {
            if block.id != func.entry_block {
                ordered_blocks.push(block);
            }
        }

        // Pass 1: Emit instructions and record block offsets & jump fixups
        for (b_idx, block) in ordered_blocks.iter().enumerate() {
            let bb_start = instructions.len();
            block_offsets.insert(block.id, bb_start);

            // In entry block, copy arguments into parameter registers
            if b_idx == 0 {
                for (arg_idx, (_, val_id, _)) in func.params.iter().enumerate() {
                    let reg = regalloc.get(*val_id);
                    instructions.push(Instruction::new(
                        Opcode::LoadConst,
                        vec![OperandType::Argument(arg_idx), OperandType::Register(reg)],
                    ));
                }
            }

            // Emit instructions in block
            for inst in &block.instructions {
                self.emit_instruction(
                    inst,
                    &regalloc,
                    &const_values,
                    &global_fn_names,
                    &value_ops,
                    types,
                    &mut instructions,
                    &mut max_reg,
                );
            }

            // Emit terminator
            match &block.terminator {
                SsaTerminator::Return(val_opt, _) => {
                    if let Some(val) = val_opt {
                        if let Some(lit) = const_values.get(val) {
                            instructions.push(Instruction::new(
                                Opcode::Ret,
                                vec![OperandType::Constant(lit_to_value(lit))],
                            ));
                        } else {
                            let reg = regalloc.get(*val);
                            instructions.push(Instruction::new(
                                Opcode::Ret,
                                vec![OperandType::Register(reg)],
                            ));
                        }
                    } else {
                        instructions.push(Instruction::new(
                            Opcode::Ret,
                            vec![OperandType::Constant(Value::Nil)],
                        ));
                    }
                }
                SsaTerminator::Goto(target, _) => {
                    // Check if target has phis coming from this block
                    self.emit_phi_moves(func, block.id, *target, &regalloc, &const_values, &mut instructions);
                    let jump_idx = instructions.len();
                    instructions.push(Instruction::new(
                        Opcode::Jump,
                        vec![OperandType::Offset(0)],
                    ));
                    jump_fixups.push((jump_idx, 0, *target));
                }
                SsaTerminator::Branch { cond, then_bb, else_bb, .. } => {
                    let cond_op = if let Some(lit) = const_values.get(cond) {
                        OperandType::Constant(lit_to_value(lit))
                    } else {
                        OperandType::Register(regalloc.get(*cond))
                    };
                    if b_idx + 1 < ordered_blocks.len() && ordered_blocks[b_idx + 1].id == *then_bb {
                        // Inverted branch: JumpFalse to else_bb, fall-through to then_bb!
                        let else_jump_idx = instructions.len();
                        instructions.push(Instruction::new(
                            Opcode::JumpFalse,
                            vec![cond_op, OperandType::Offset(0)],
                        ));
                        jump_fixups.push((else_jump_idx, 1, *else_bb));
                    } else {
                        let then_jump_idx = instructions.len();
                        instructions.push(Instruction::new(
                            Opcode::JumpTrue,
                            vec![cond_op, OperandType::Offset(0)],
                        ));
                        jump_fixups.push((then_jump_idx, 1, *then_bb));

                        // Else fall-through or jump
                        self.emit_phi_moves(func, block.id, *else_bb, &regalloc, &const_values, &mut instructions);
                        let else_jump_idx = instructions.len();
                        instructions.push(Instruction::new(
                            Opcode::Jump,
                            vec![OperandType::Offset(0)],
                        ));
                        jump_fixups.push((else_jump_idx, 0, *else_bb));
                    }
                }
                SsaTerminator::Match { target, arms, default_bb, .. } => {
                    let target_op = if let Some(lit) = const_values.get(target) {
                        OperandType::Constant(lit_to_value(lit))
                    } else {
                        OperandType::Register(regalloc.get(*target))
                    };
                    let match_idx = instructions.len();

                    let mut operands = Vec::new();
                    operands.push(target_op);
                    operands.push(OperandType::Offset(arms.len() as i64));

                    for arm in arms {
                        let binding_regs = arm
                            .bindings
                            .iter()
                            .map(|(name, value)| (name.clone(), regalloc.get(*value)))
                            .collect();
                        let pat = self.lower_pattern(&arm.pattern, &regalloc, &binding_regs);
                        operands.push(OperandType::Pattern(pat));
                        operands.push(OperandType::Offset(0)); // placeholder
                    }
                    operands.push(OperandType::Offset(0)); // default placeholder

                    instructions.push(Instruction::new(Opcode::Match, operands));

                    for (arm_idx, arm) in arms.iter().enumerate() {
                        let op_idx = 2 + arm_idx * 2 + 1;
                        jump_fixups.push((match_idx, op_idx, arm.target_bb));
                    }
                    let def_op_idx = 2 + arms.len() * 2;
                    jump_fixups.push((match_idx, def_op_idx, *default_bb));
                }
                SsaTerminator::Unreachable(_) => {
                    instructions.push(Instruction::new(Opcode::Nop, vec![]));
                }
            }
        }

        // Pass 2: Resolve relative jump offsets
        for (inst_idx, operand_idx, target_bb) in jump_fixups {
            if let Some(&target_pc) = block_offsets.get(&target_bb) {
                let rel_offset = target_pc as i64 - inst_idx as i64;
                instructions[inst_idx].operands[operand_idx] = OperandType::Offset(rel_offset);
            }
        }

        // Detect if function uses closure upvalues
        let is_closure = instructions.iter().any(|i| {
            i.operands.iter().any(|op| matches!(op, OperandType::Captured(_)))
        });

        Function {
            name: func.name.clone(),
            instructions,
            register_count: max_reg.max(func.params.len()).max(4),
            argument_count: func.params.len(),
            return_type: None,
            bound_this: None,
            fn_type: if is_closure {
                FunctionType::Closure
            } else {
                FunctionType::Regular
            },
            call_counter: 0,
            jit_compiled_fn: None,
            closure_env: None,
        }
    }

    fn emit_phi_moves(
        &self,
        func: &SsaFunction,
        from_bb: BlockId,
        to_bb: BlockId,
        regalloc: &RegAlloc,
        const_values: &HashMap<ValueId, LitVal>,
        instructions: &mut Vec<Instruction>,
    ) {
        if let Some(target) = func.blocks.iter().find(|b| b.id == to_bb) {
            let mut pending_moves = Vec::new();
            let mut const_moves = Vec::new();
            for phi in &target.phis {
                for &(incoming_bb, val) in &phi.incoming {
                    if incoming_bb == from_bb {
                        let dst_reg = regalloc.get(phi.dst);
                        if let Some(lit) = const_values.get(&val) {
                            const_moves.push((lit_to_value(lit), dst_reg));
                        } else {
                            let src_reg = regalloc.get(val);
                            if src_reg != dst_reg {
                                pending_moves.push((src_reg, dst_reg));
                            }
                        }
                    }
                }
            }

            // Check if the last instruction is an arithmetic op producing a phi source
            let can_fuse = instructions.last().and_then(|inst| {
                if matches!(
                    inst.opcode,
                    Opcode::Add
                        | Opcode::Sub
                        | Opcode::Mul
                        | Opcode::Div
                        | Opcode::IAddRRR
                        | Opcode::IAddRCR
                        | Opcode::ISubRRR
                        | Opcode::ISubRCR
                        | Opcode::ISubCRR
                        | Opcode::IMulRRR
                        | Opcode::IMulRCR
                        | Opcode::IDivRCR
                        | Opcode::IDivCRR
                        | Opcode::IRemRCR
                        | Opcode::IRemCRR
                ) {
                    inst.operands.get(2).and_then(|op| match op {
                        OperandType::Register(r) => Some(*r),
                        _ => None,
                    })
                } else {
                    None
                }
            });

            let fuse_idx = can_fuse.and_then(|r| pending_moves.iter().position(|(src, _)| *src == r));

            if let Some(idx) = fuse_idx {
                let (_, dst_reg) = pending_moves.remove(idx);
                let mut latch_inst = instructions.pop().unwrap();
                latch_inst.operands[2] = OperandType::Register(dst_reg);

                // Emit all other phi moves before the latch increment
                for (src_reg, dst_reg) in pending_moves {
                    instructions.push(Instruction::new(
                        Opcode::LoadConst,
                        vec![OperandType::Register(src_reg), OperandType::Register(dst_reg)],
                    ));
                }
                for (val, dst_reg) in const_moves {
                    instructions.push(Instruction::new(
                        Opcode::LoadConst,
                        vec![OperandType::Constant(val), OperandType::Register(dst_reg)],
                    ));
                }

                // Push the fused latch increment right before the jump!
                instructions.push(latch_inst);
            } else {
                for (src_reg, dst_reg) in pending_moves {
                    instructions.push(Instruction::new(
                        Opcode::LoadConst,
                        vec![OperandType::Register(src_reg), OperandType::Register(dst_reg)],
                    ));
                }
                for (val, dst_reg) in const_moves {
                    instructions.push(Instruction::new(
                        Opcode::LoadConst,
                        vec![OperandType::Constant(val), OperandType::Register(dst_reg)],
                    ));
                }
            }
        }
    }

    fn emit_instruction(
        &self,
        inst: &SsaInst,
        regalloc: &RegAlloc,
        const_values: &HashMap<ValueId, LitVal>,
        global_fn_names: &HashMap<ValueId, String>,
        value_ops: &HashMap<ValueId, SsaOp>,
        types: Option<&HashMap<ValueId, Type>>,
        instructions: &mut Vec<Instruction>,
        max_reg: &mut usize,
    ) {
        if let Some(dst) = inst.dst {
            if const_values.contains_key(&dst) {
                return; // Folded constant or compile-time evaluated
            }
        }
        let dst_reg = inst.dst.map(|d| regalloc.get(d));

        match &inst.op {
            SsaOp::Const(_) => {
                // Constants do not generate virtual registers or load_const instructions directly!
            }
            SsaOp::Binary(op, left, right) => {
                let dst = dst_reg.expect("binary op requires dst");
                let l_const = const_values.get(left);
                let r_const = const_values.get(right);

                let is_int_val = |v: &ValueId| -> bool {
                    if let Some(lit) = const_values.get(v) {
                        return matches!(lit, LitVal::Int(_));
                    }
                    types.and_then(|t| t.get(v)).map(|ty| *ty == Type::Int).unwrap_or(false)
                };
                let is_float_val = |v: &ValueId| -> bool {
                    if let Some(lit) = const_values.get(v) {
                        return matches!(lit, LitVal::Float(_));
                    }
                    types.and_then(|t| t.get(v)).map(|ty| *ty == Type::Float).unwrap_or(false)
                };

                let is_int_op = is_int_val(left) || is_int_val(right) || inst.dst.map(|d| is_int_val(&d)).unwrap_or(false);
                let is_float_op = is_float_val(left) || is_float_val(right) || inst.dst.map(|d| is_float_val(&d)).unwrap_or(false);

                let (opcode, op0, op1) = match op {
                    BinOp::Add => {
                        if is_int_op {
                            if let Some(LitVal::Int(c)) = r_const {
                                (Opcode::IAddRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Int(*c)))
                            } else if let Some(LitVal::Int(c)) = l_const {
                                (Opcode::IAddRCR, OperandType::Register(regalloc.get(*right)), OperandType::Constant(Value::Int(*c)))
                            } else {
                                (Opcode::IAddRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else if is_float_op {
                            if let Some(LitVal::Float(c)) = r_const {
                                (Opcode::FAddRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Float(*c)))
                            } else if let Some(LitVal::Float(c)) = l_const {
                                (Opcode::FAddRCR, OperandType::Register(regalloc.get(*right)), OperandType::Constant(Value::Float(*c)))
                            } else {
                                (Opcode::FAddRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else {
                            let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                            let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                            (Opcode::Add, op0, op1)
                        }
                    }
                    BinOp::Sub => {
                        if is_int_op {
                            if let Some(LitVal::Int(c)) = r_const {
                                (Opcode::ISubRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Int(*c)))
                            } else if let Some(LitVal::Int(c)) = l_const {
                                (Opcode::ISubCRR, OperandType::Constant(Value::Int(*c)), OperandType::Register(regalloc.get(*right)))
                            } else {
                                (Opcode::ISubRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else if is_float_op {
                            if let Some(LitVal::Float(c)) = r_const {
                                (Opcode::FSubRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Float(*c)))
                            } else if let Some(LitVal::Float(c)) = l_const {
                                (Opcode::FSubCRR, OperandType::Constant(Value::Float(*c)), OperandType::Register(regalloc.get(*right)))
                            } else {
                                (Opcode::FSubRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else {
                            let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                            let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                            (Opcode::Sub, op0, op1)
                        }
                    }
                    BinOp::Mul => {
                        if is_int_op {
                            if let Some(LitVal::Int(c)) = r_const {
                                (Opcode::IMulRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Int(*c)))
                            } else if let Some(LitVal::Int(c)) = l_const {
                                (Opcode::IMulRCR, OperandType::Register(regalloc.get(*right)), OperandType::Constant(Value::Int(*c)))
                            } else {
                                (Opcode::IMulRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else if is_float_op {
                            if let Some(LitVal::Float(c)) = r_const {
                                (Opcode::FMulRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Float(*c)))
                            } else if let Some(LitVal::Float(c)) = l_const {
                                (Opcode::FMulRCR, OperandType::Register(regalloc.get(*right)), OperandType::Constant(Value::Float(*c)))
                            } else {
                                (Opcode::FMulRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else {
                            let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                            let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                            (Opcode::Mul, op0, op1)
                        }
                    }
                    BinOp::Div => {
                        if is_int_op {
                            if let Some(LitVal::Int(c)) = r_const {
                                (Opcode::IDivRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Int(*c)))
                            } else if let Some(LitVal::Int(c)) = l_const {
                                (Opcode::IDivCRR, OperandType::Constant(Value::Int(*c)), OperandType::Register(regalloc.get(*right)))
                            } else {
                                (Opcode::Div, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else if is_float_op {
                            if let Some(LitVal::Float(c)) = r_const {
                                (Opcode::FDivRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Float(*c)))
                            } else if let Some(LitVal::Float(c)) = l_const {
                                (Opcode::FDivCRR, OperandType::Constant(Value::Float(*c)), OperandType::Register(regalloc.get(*right)))
                            } else {
                                (Opcode::FDivRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else {
                            let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                            let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                            (Opcode::Div, op0, op1)
                        }
                    }
                    BinOp::Rem => {
                        if is_int_op {
                            if let Some(LitVal::Int(c)) = r_const {
                                (Opcode::IRemRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Int(*c)))
                            } else if let Some(LitVal::Int(c)) = l_const {
                                (Opcode::IRemCRR, OperandType::Constant(Value::Int(*c)), OperandType::Register(regalloc.get(*right)))
                            } else {
                                (Opcode::Rem, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else {
                            let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                            let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                            (Opcode::Rem, op0, op1)
                        }
                    }
                    BinOp::Gt => {
                        if is_int_op {
                            if let Some(LitVal::Int(c)) = r_const {
                                (Opcode::IGtRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Int(*c)))
                            } else if let Some(LitVal::Int(c)) = l_const {
                                (Opcode::IGtCRR, OperandType::Constant(Value::Int(*c)), OperandType::Register(regalloc.get(*right)))
                            } else {
                                (Opcode::IGtRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else if is_float_op {
                            if let Some(LitVal::Float(c)) = r_const {
                                (Opcode::FGtRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Float(*c)))
                            } else if let Some(LitVal::Float(c)) = l_const {
                                (Opcode::FLtRCR, OperandType::Register(regalloc.get(*right)), OperandType::Constant(Value::Float(*c)))
                            } else {
                                (Opcode::FGtRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else {
                            let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                            let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                            (Opcode::Gt, op0, op1)
                        }
                    }
                    BinOp::Lt => {
                        if is_int_op {
                            if let Some(LitVal::Int(c)) = r_const {
                                (Opcode::ILtRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Int(*c)))
                            } else if let Some(LitVal::Int(c)) = l_const {
                                (Opcode::ILtCRR, OperandType::Constant(Value::Int(*c)), OperandType::Register(regalloc.get(*right)))
                            } else {
                                (Opcode::ILtRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else if is_float_op {
                            if let Some(LitVal::Float(c)) = r_const {
                                (Opcode::FLtRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Float(*c)))
                            } else if let Some(LitVal::Float(c)) = l_const {
                                (Opcode::FGtRCR, OperandType::Register(regalloc.get(*right)), OperandType::Constant(Value::Float(*c)))
                            } else {
                                (Opcode::FLtRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else {
                            let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                            let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                            (Opcode::Lt, op0, op1)
                        }
                    }
                    BinOp::Eq => {
                        if is_int_op {
                            if let Some(LitVal::Int(c)) = r_const {
                                (Opcode::IEqRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Int(*c)))
                            } else if let Some(LitVal::Int(c)) = l_const {
                                (Opcode::IEqRCR, OperandType::Register(regalloc.get(*right)), OperandType::Constant(Value::Int(*c)))
                            } else {
                                (Opcode::IEqRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else if is_float_op {
                            if let Some(LitVal::Float(c)) = r_const {
                                (Opcode::FEqRCR, OperandType::Register(regalloc.get(*left)), OperandType::Constant(Value::Float(*c)))
                            } else if let Some(LitVal::Float(c)) = l_const {
                                (Opcode::FEqRCR, OperandType::Register(regalloc.get(*right)), OperandType::Constant(Value::Float(*c)))
                            } else {
                                (Opcode::FEqRRR, OperandType::Register(regalloc.get(*left)), OperandType::Register(regalloc.get(*right)))
                            }
                        } else {
                            let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                            let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                            (Opcode::Eq, op0, op1)
                        }
                    }
                    BinOp::NotEq => {
                        let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                        let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                        (Opcode::Neq, op0, op1)
                    }
                    BinOp::LtEq => {
                        let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                        let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                        (Opcode::Le, op0, op1)
                    }
                    BinOp::GtEq => {
                        let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                        let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                        (Opcode::Ge, op0, op1)
                    }
                    BinOp::And => {
                        let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                        let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                        (Opcode::And, op0, op1)
                    }
                    BinOp::Or => {
                        let op0 = if let Some(l) = l_const { OperandType::Constant(lit_to_value(l)) } else { OperandType::Register(regalloc.get(*left)) };
                        let op1 = if let Some(r) = r_const { OperandType::Constant(lit_to_value(r)) } else { OperandType::Register(regalloc.get(*right)) };
                        (Opcode::Or, op0, op1)
                    }
                };

                instructions.push(Instruction::new(
                    opcode,
                    vec![op0, op1, OperandType::Register(dst)],
                ));
            }
            SsaOp::Unary(op, val) => {
                let dst = dst_reg.expect("unary op requires dst");
                let v_op = if let Some(lit) = const_values.get(val) {
                    OperandType::Constant(lit_to_value(lit))
                } else {
                    OperandType::Register(regalloc.get(*val))
                };
                match op {
                    UnaryOp::Not => {
                        instructions.push(Instruction::new(
                            Opcode::Not,
                            vec![v_op, OperandType::Register(dst)],
                        ));
                    }
                    UnaryOp::Neg => {
                        instructions.push(Instruction::new(
                            Opcode::Sub,
                            vec![
                                OperandType::Constant(Value::Int(0)),
                                v_op,
                                OperandType::Register(dst),
                            ],
                        ));
                    }
                }
            }
            SsaOp::Call { callee, args } => {
                let primitive_format = if args.len() == 1 {
                    match value_ops.get(callee) {
                        Some(SsaOp::FieldAccess { base, field }) if field == "format" => {
                            types
                                .and_then(|types| types.get(base))
                                .filter(|ty| {
                                    matches!(ty, Type::Int | Type::Float | Type::Bool | Type::String)
                                })
                                .map(|_| (*base, args[0]))
                        }
                        _ => None,
                    }
                } else {
                    None
                };

                if let Some((receiver, spec)) = primitive_format {
                    let dst = dst_reg.expect("primitive format call requires dst");
                    let receiver_op = if let Some(lit) = const_values.get(&receiver) {
                        OperandType::Constant(lit_to_value(lit))
                    } else {
                        OperandType::Register(regalloc.get(receiver))
                    };
                    let spec_op = if let Some(lit) = const_values.get(&spec) {
                        OperandType::Constant(lit_to_value(lit))
                    } else {
                        OperandType::Register(regalloc.get(spec))
                    };
                    instructions.push(Instruction::new(
                        Opcode::FormatValue,
                        vec![receiver_op, spec_op, OperandType::Register(dst)],
                    ));
                } else if let Some(fn_name) = global_fn_names.get(callee) {
                    if fn_name == "len" && args.len() == 1 {
                        let dst = dst_reg.unwrap_or(0);
                        let arr_op = if let Some(lit) = const_values.get(&args[0]) {
                            OperandType::Constant(lit_to_value(lit))
                        } else {
                            OperandType::Register(regalloc.get(args[0]))
                        };
                        instructions.push(Instruction::new(
                            Opcode::ArrayLen,
                            vec![arr_op, OperandType::Register(dst)],
                        ));
                    } else {
                        let callee_op = OperandType::Function(fn_name.clone());
                        let dst = dst_reg.unwrap_or(0);
                        let mut operands = Vec::with_capacity(3 + args.len());
                        operands.push(callee_op);
                        operands.push(OperandType::Offset(args.len() as i64));
                        for a in args {
                            if let Some(lit) = const_values.get(a) {
                                operands.push(OperandType::Constant(lit_to_value(lit)));
                            } else {
                                operands.push(OperandType::Register(regalloc.get(*a)));
                            }
                        }
                        operands.push(OperandType::Register(dst));
                        instructions.push(Instruction::new(Opcode::Call, operands));
                    }
                } else {
                    let callee_op = OperandType::Register(regalloc.get(*callee));
                    let dst = dst_reg.unwrap_or(0);
                    let mut operands = Vec::with_capacity(3 + args.len());
                    operands.push(callee_op);
                    operands.push(OperandType::Offset(args.len() as i64));
                    for a in args {
                        if let Some(lit) = const_values.get(a) {
                            operands.push(OperandType::Constant(lit_to_value(lit)));
                        } else {
                            operands.push(OperandType::Register(regalloc.get(*a)));
                        }
                    }
                    operands.push(OperandType::Register(dst));
                    instructions.push(Instruction::new(Opcode::Call, operands));
                }
            }
            SsaOp::GlobalRef(name) => {
                let dst = dst_reg.expect("global ref requires dst");
                instructions.push(Instruction::new(
                    Opcode::LoadConst,
                    vec![OperandType::Function(name.clone()), OperandType::Register(dst)],
                ));
            }
            SsaOp::QualifiedGlobal(path) => {
                let dst = dst_reg.expect("qualified global requires dst");
                let full_name = resolve_qualified_name(path);
                instructions.push(Instruction::new(
                    Opcode::LoadConst,
                    vec![OperandType::Function(full_name), OperandType::Register(dst)],
                ));
            }
            SsaOp::Atom(name) => {
                let dst = dst_reg.expect("atom requires dst");
                instructions.push(Instruction::new(
                    Opcode::LoadConst,
                    vec![
                        OperandType::Constant(Value::String(format!(":{}", name))),
                        OperandType::Register(dst),
                    ],
                ));
            }
            SsaOp::Upvalue(idx) => {
                let dst = dst_reg.expect("upvalue requires dst");
                instructions.push(Instruction::new(
                    Opcode::LoadConst,
                    vec![OperandType::Captured(*idx), OperandType::Register(dst)],
                ));
            }
            SsaOp::StoreUpvalue { index, val } => {
                let v_op = if let Some(lit) = const_values.get(val) {
                    OperandType::Constant(lit_to_value(lit))
                } else {
                    OperandType::Register(regalloc.get(*val))
                };
                instructions.push(Instruction::new(
                    Opcode::LoadConst,
                    vec![v_op, OperandType::Captured(*index)],
                ));
            }
            SsaOp::MakeEnv(captured_values) => {
                let dst = dst_reg.expect("make_env requires dst");
                let mut captured_regs = Vec::with_capacity(captured_values.len());
                for v in captured_values {
                    if let Some(r) = regalloc.get_opt(*v) {
                        captured_regs.push(r);
                    } else if let Some(lit) = const_values.get(v) {
                        let r = *max_reg;
                        *max_reg += 1;
                        instructions.push(Instruction::new(
                            Opcode::LoadConst,
                            vec![
                                OperandType::Constant(lit_to_value(lit)),
                                OperandType::Register(r),
                            ],
                        ));
                        captured_regs.push(r);
                    } else {
                        captured_regs.push(regalloc.get(*v));
                    }
                }
                instructions.push(Instruction::new(
                    Opcode::MakeEnv,
                    vec![
                        OperandType::Register(dst),
                        OperandType::RegisterList(captured_regs),
                    ],
                ));
            }
            SsaOp::MakeClosure { func, env } => {
                let dst = dst_reg.expect("make_closure requires dst");
                let env_reg = if let Some(e) = env {
                    regalloc.get(*e)
                } else {
                    let r = *max_reg;
                    *max_reg += 1;
                    instructions.push(Instruction::new(
                        Opcode::MakeEnv,
                        vec![
                            OperandType::Register(r),
                            OperandType::RegisterList(vec![]),
                        ],
                    ));
                    r
                };

                instructions.push(Instruction::new(
                    Opcode::MakeClosure,
                    vec![
                        OperandType::Function(func.clone()),
                        OperandType::Register(env_reg),
                        OperandType::Register(dst),
                    ],
                ));
            }
            SsaOp::FieldAccess { base, field } => {
                let dst = dst_reg.expect("field access requires dst");
                let base_op = if let Some(lit) = const_values.get(base) {
                    OperandType::Constant(lit_to_value(lit))
                } else {
                    OperandType::Register(regalloc.get(*base))
                };
                instructions.push(Instruction::new(
                    Opcode::GetProp,
                    vec![
                        base_op,
                        OperandType::Property(field.clone()),
                        OperandType::Register(dst),
                    ],
                ));
            }
            SsaOp::StoreField { base, field, val } => {
                let b_reg = regalloc.get(*base);
                let val_op = if let Some(lit) = const_values.get(val) {
                    OperandType::Constant(lit_to_value(lit))
                } else {
                    OperandType::Register(regalloc.get(*val))
                };
                instructions.push(Instruction::new(
                    Opcode::SetProp,
                    vec![
                        OperandType::Register(b_reg),
                        OperandType::Property(field.clone()),
                        val_op,
                    ],
                ));
            }
            SsaOp::Index { base, index } => {
                let dst = dst_reg.expect("index requires dst");
                let b_reg = regalloc.get(*base);
                let idx_op = if let Some(lit) = const_values.get(index) {
                    OperandType::Constant(lit_to_value(lit))
                } else {
                    OperandType::Register(regalloc.get(*index))
                };
                let is_int_arr = types
                    .and_then(|t| t.get(base))
                    .map(|t| match t {
                        Type::IntArray => true,
                        Type::Array(elem) => **elem == Type::Int,
                        _ => false,
                    })
                    .unwrap_or(false);
                let opcode = if is_int_arr {
                    Opcode::IArrayGet
                } else {
                    Opcode::GetIndex
                };
                instructions.push(Instruction::new(
                    opcode,
                    vec![
                        OperandType::Register(b_reg),
                        idx_op,
                        OperandType::Register(dst),
                    ],
                ));
            }
            SsaOp::StoreIndex { base, index, val } => {
                let b_reg = regalloc.get(*base);
                let idx_op = if let Some(lit) = const_values.get(index) {
                    OperandType::Constant(lit_to_value(lit))
                } else {
                    OperandType::Register(regalloc.get(*index))
                };
                let v_op = if let Some(lit) = const_values.get(val) {
                    OperandType::Constant(lit_to_value(lit))
                } else {
                    OperandType::Register(regalloc.get(*val))
                };
                let is_int_arr = types
                    .and_then(|t| t.get(base))
                    .map(|t| match t {
                        Type::IntArray => true,
                        Type::Array(elem) => **elem == Type::Int,
                        _ => false,
                    })
                    .unwrap_or(false);
                let opcode = if is_int_arr {
                    Opcode::IArraySet
                } else {
                    Opcode::ArraySet
                };
                instructions.push(Instruction::new(
                    opcode,
                    vec![
                        OperandType::Register(b_reg),
                        idx_op,
                        v_op,
                    ],
                ));
            }
            SsaOp::ArrayLit(elems) => {
                let dst = dst_reg.expect("array lit requires dst");
                let is_int_arr = types
                    .and_then(|t| inst.dst.and_then(|d| t.get(&d)))
                    .map(|t| *t == Type::IntArray)
                    .unwrap_or(false);
                let is_float_arr = types
                    .and_then(|t| inst.dst.and_then(|d| t.get(&d)))
                    .map(|t| *t == Type::FloatArray)
                    .unwrap_or(false);

                if is_float_arr {
                    let all_float_consts = !elems.is_empty()
                        && elems.iter().all(|e| matches!(const_values.get(e), Some(LitVal::Float(_))));
                    if all_float_consts {
                        let floats: Vec<f64> = elems
                            .iter()
                            .map(|e| match const_values.get(e).unwrap() {
                                LitVal::Float(f) => *f,
                                _ => unreachable!(),
                            })
                            .collect();
                        instructions.push(Instruction::new(
                            Opcode::LoadConst,
                            vec![
                                OperandType::Constant(Value::FloatArray(Arc::new(RwLock::new(floats)))),
                                OperandType::Register(dst),
                            ],
                        ));
                    } else {
                        let scratch = *max_reg;
                        *max_reg += 1;
                        instructions.push(Instruction::new(
                            Opcode::LoadConst,
                            vec![
                                OperandType::Constant(Value::Int(elems.len() as i64)),
                                OperandType::Register(scratch),
                            ],
                        ));
                        instructions.push(Instruction::new(
                            Opcode::NewFloatArray,
                            vec![OperandType::Register(scratch), OperandType::Register(dst)],
                        ));
                        for (i, elem) in elems.iter().enumerate() {
                            let val_op = if let Some(lit) = const_values.get(elem) {
                                OperandType::Constant(lit_to_value(lit))
                            } else {
                                OperandType::Register(regalloc.get(*elem))
                            };
                            instructions.push(Instruction::new(
                                Opcode::FArraySet,
                                vec![
                                    OperandType::Register(dst),
                                    OperandType::Constant(Value::Int(i as i64)),
                                    val_op,
                                ],
                            ));
                        }
                    }
                } else if is_int_arr {
                    let all_int_consts = !elems.is_empty()
                        && elems.iter().all(|e| matches!(const_values.get(e), Some(LitVal::Int(_))));
                    if all_int_consts {
                        let ints: Vec<i64> = elems
                            .iter()
                            .map(|e| match const_values.get(e).unwrap() {
                                LitVal::Int(i) => *i,
                                _ => unreachable!(),
                            })
                            .collect();
                        instructions.push(Instruction::new(
                            Opcode::LoadConst,
                            vec![
                                OperandType::Constant(Value::IntArray(Arc::new(RwLock::new(ints)))),
                                OperandType::Register(dst),
                            ],
                        ));
                    } else {
                        let scratch = *max_reg;
                        *max_reg += 1;
                        instructions.push(Instruction::new(
                            Opcode::LoadConst,
                            vec![
                                OperandType::Constant(Value::Int(elems.len() as i64)),
                                OperandType::Register(scratch),
                            ],
                        ));
                        instructions.push(Instruction::new(
                            Opcode::NewIntArray,
                            vec![OperandType::Register(scratch), OperandType::Register(dst)],
                        ));
                        for (i, elem) in elems.iter().enumerate() {
                            let val_op = if let Some(lit) = const_values.get(elem) {
                                OperandType::Constant(lit_to_value(lit))
                            } else {
                                OperandType::Register(regalloc.get(*elem))
                            };
                            instructions.push(Instruction::new(
                                Opcode::IArraySet,
                                vec![
                                    OperandType::Register(dst),
                                    OperandType::Constant(Value::Int(i as i64)),
                                    val_op,
                                ],
                            ));
                        }
                    }
                } else {
                    // Normal mixed array (e.g. [1, 2, 3.0, 4] or generic Array): stays a normal mixed array!
                    let mut reg_list = Vec::with_capacity(elems.len());
                    for e in elems {
                        if let Some(r) = regalloc.get_opt(*e) {
                            reg_list.push(r);
                        } else if let Some(lit) = const_values.get(e) {
                            let r = *max_reg;
                            *max_reg += 1;
                            instructions.push(Instruction::new(
                                Opcode::LoadConst,
                                vec![
                                    OperandType::Constant(lit_to_value(lit)),
                                    OperandType::Register(r),
                                ],
                            ));
                            reg_list.push(r);
                        } else {
                            reg_list.push(regalloc.get(*e));
                        }
                    }
                    instructions.push(Instruction::new(
                        Opcode::MakeEnv,
                        vec![OperandType::Register(dst), OperandType::RegisterList(reg_list)],
                    ));
                }
            }
            SsaOp::TupleLit(elems) => {
                let dst = dst_reg.expect("tuple lit requires dst");
                let mut reg_list = Vec::with_capacity(elems.len());
                for e in elems {
                    if let Some(r) = regalloc.get_opt(*e) {
                        reg_list.push(r);
                    } else if let Some(lit) = const_values.get(e) {
                        let r = *max_reg;
                        *max_reg += 1;
                        instructions.push(Instruction::new(
                            Opcode::LoadConst,
                            vec![
                                OperandType::Constant(lit_to_value(lit)),
                                OperandType::Register(r),
                            ],
                        ));
                        reg_list.push(r);
                    } else {
                        reg_list.push(regalloc.get(*e));
                    }
                }
                instructions.push(Instruction::new(
                    Opcode::MakeEnv,
                    vec![OperandType::Register(dst), OperandType::RegisterList(reg_list)],
                ));
            }
            SsaOp::NewObject { prototype, fields, args, .. } => {
                let dst = dst_reg.expect("new object requires dst");
                instructions.push(Instruction::new(
                    Opcode::NewObject,
                    vec![
                        OperandType::Register(dst),
                        OperandType::Property(prototype.clone()),
                    ],
                ));
                for ((field, _), arg) in fields.iter().zip(args) {
                    let value = if let Some(lit) = const_values.get(arg) {
                        OperandType::Constant(lit_to_value(lit))
                    } else {
                        OperandType::Register(regalloc.get(*arg))
                    };
                    instructions.push(Instruction::new(
                        Opcode::SetProp,
                        vec![
                            OperandType::Register(dst),
                            OperandType::Property(field.clone()),
                            value,
                        ],
                    ));
                }
            }
            SsaOp::Spawn { callee, args } => {
                let dst = dst_reg.expect("spawn requires dst");
                let callee_op = if let Some(fn_name) = global_fn_names.get(callee) {
                    OperandType::Function(fn_name.clone())
                } else {
                    OperandType::Register(regalloc.get(*callee))
                };
                let mut operands = vec![
                    callee_op,
                    OperandType::Offset(args.len() as i64),
                ];
                for a in args {
                    if let Some(lit) = const_values.get(a) {
                        operands.push(OperandType::Constant(lit_to_value(lit)));
                    } else {
                        operands.push(OperandType::Register(regalloc.get(*a)));
                    }
                }
                operands.push(OperandType::Register(dst));
                instructions.push(Instruction::new(Opcode::Spawn, operands));
            }
            SsaOp::Send { receiver, message } => {
                let r_op = if let Some(lit) = const_values.get(receiver) {
                    OperandType::Constant(lit_to_value(lit))
                } else {
                    OperandType::Register(regalloc.get(*receiver))
                };
                let m_op = if let Some(lit) = const_values.get(message) {
                    OperandType::Constant(lit_to_value(lit))
                } else {
                    OperandType::Register(regalloc.get(*message))
                };
                instructions.push(Instruction::new(
                    Opcode::Send,
                    vec![r_op, m_op],
                ));
            }
            SsaOp::Receive => {
                let dst = dst_reg.expect("receive requires dst");
                instructions.push(Instruction::new(
                    Opcode::Receive,
                    vec![OperandType::Register(dst)],
                ));
            }
            SsaOp::This => {
                let dst = dst_reg.expect("this requires dst");
                instructions.push(Instruction::new(
                    Opcode::LoadConst,
                    vec![OperandType::This, OperandType::Register(dst)],
                ));
            }
            SsaOp::BuildString(parts) => {
                let dst = dst_reg.expect("build_string requires dst");
                let mut operands = Vec::with_capacity(parts.len() + 1);
                for part in parts {
                    match part {
                        StringPartValue::Literal(s) => {
                            operands.push(OperandType::Constant(Value::String(s.clone())));
                        }
                        StringPartValue::Value(v) => {
                            if let Some(lit) = const_values.get(v) {
                                match lit {
                                    LitVal::String(s) => operands.push(OperandType::Constant(Value::String(s.clone()))),
                                    LitVal::Int(i) => operands.push(OperandType::Constant(Value::Int(*i))),
                                    LitVal::Float(f) => operands.push(OperandType::Constant(Value::Float(*f))),
                                    LitVal::Bool(b) => operands.push(OperandType::Constant(Value::Bool(*b))),
                                    LitVal::Unit | LitVal::Nil => operands.push(OperandType::Constant(Value::Nil)),
                                }
                            } else {
                                operands.push(OperandType::Register(regalloc.get(*v)));
                            }
                        }
                    }
                }
                operands.push(OperandType::Register(dst));
                instructions.push(Instruction::new(Opcode::BuildString, operands));
            }
            SsaOp::FormatValue { val, spec } => {
                let dst = dst_reg.expect("FormatValue requires dst register");
                let val_op = if let Some(lit) = const_values.get(val) {
                    OperandType::Constant(lit_to_value(lit))
                } else {
                    OperandType::Register(regalloc.get(*val))
                };
                let mut operands = vec![val_op];
                if let Some(spec_id) = spec {
                    let spec_op = if let Some(lit) = const_values.get(spec_id) {
                        OperandType::Constant(lit_to_value(lit))
                    } else {
                        OperandType::Register(regalloc.get(*spec_id))
                    };
                    operands.push(spec_op);
                }
                operands.push(OperandType::Register(dst));
                instructions.push(Instruction::new(Opcode::FormatValue, operands));
            }
        }
    }

    fn lower_pattern(
        &self,
        pat: &PatternNode,
        regalloc: &RegAlloc,
        binding_regs: &HashMap<String, usize>,
    ) -> Pattern {
        match pat {
            PatternNode::Wildcard(_) => Pattern::Wildcard,
            PatternNode::Bind(name, _) => Pattern::Bind(
                *binding_regs
                    .get(name)
                    .expect("pattern binding must have an allocated register"),
            ),
            PatternNode::Literal(lit, _) => {
                let val = match lit {
                    LitVal::Int(i) => Value::Int(*i),
                    LitVal::Float(f) => Value::Float(*f),
                    LitVal::Bool(b) => Value::Bool(*b),
                    LitVal::String(s) => Value::String(s.clone()),
                    LitVal::Nil | LitVal::Unit => Value::Nil,
                };
                Pattern::Literal(val)
            }
            PatternNode::Tuple(elems, _) => {
                Pattern::Tuple(
                    elems
                        .iter()
                        .map(|elem| self.lower_pattern(elem, regalloc, binding_regs))
                        .collect(),
                )
            }
            PatternNode::Range { start, end, inclusive, .. } => {
                let start_val = start.as_ref().and_then(|e| match e.as_ref() {
                    crate::syntax::cst::Expr::Literal(LitVal::Int(i), _) => Some(*i),
                    _ => None,
                });
                let end_val = end.as_ref().and_then(|e| match e.as_ref() {
                    crate::syntax::cst::Expr::Literal(LitVal::Int(i), _) => Some(*i),
                    _ => None,
                });
                Pattern::IntRange(start_val, end_val, *inclusive)
            }
            PatternNode::Array { segments, .. } => {
                let segs = segments
                    .iter()
                    .map(|s| match s {
                        crate::syntax::cst::PatternSegment::Element(p) => {
                            ListSegment::Element(self.lower_pattern(p, regalloc, binding_regs))
                        }
                        crate::syntax::cst::PatternSegment::BindList(name, _) => {
                            ListSegment::BindList(
                                *binding_regs
                                    .get(name)
                                    .expect("list pattern binding must have an allocated register"),
                            )
                        }
                        crate::syntax::cst::PatternSegment::WildcardList(_) => {
                            ListSegment::WildcardList
                        }
                    })
                    .collect();
                Pattern::Array(ArrayPattern { segments: segs })
            }
            PatternNode::EnumVariant { name, args, .. } => {
                if let Some(fields) = self.struct_fields.get(name) {
                    if fields.len() != args.len() {
                        return Pattern::Literal(Value::Bool(false));
                    }
                    return Pattern::Object(
                        fields
                            .iter()
                            .zip(args)
                            .map(|(field, arg)| {
                                (
                                    field.clone(),
                                    self.lower_pattern(arg, regalloc, binding_regs),
                                )
                            })
                            .collect(),
                    );
                }
                let clean_name = name.strip_prefix(':').unwrap_or(name);
                Pattern::Literal(Value::String(clean_name.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::parse;
    use crate::sema::resolve;
    use crate::mir::build_ssa_program;
    use vmm::vm::IonVM;

    #[test]
    fn test_struct_constructor_uses_shared_prototype_and_compiled_methods() {
        let src = r#"
        struct Point: {
            x: Int,
            y: Int,
        }

        impl Point: {
            fn describe() -> String: {
                return "Point prototype method ${self.x}";
            }
        }

        impl ToString for Point: {
            fn to_string() -> String: {
                return "Point ${self.x}";
            }
        }

        fn main() -> String: {
            let point = Point(40, 2);
            return point.describe();
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_program_declarations(&resolved, &module);
        let types = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&types));

        assert_eq!(resolved.structs.len(), 1);
        assert_eq!(resolved.structs[0].compiled_name, "<memory>.Point");
        {
            let (prototype_name, prototype) = bytecode
                .object_inits
                .iter()
                .find(|(name, _)| name == "<memory>.Point")
                .expect("Point load-time prototype emitted");
            assert_eq!(prototype_name, "<memory>.Point");
            let prototype = prototype.read().unwrap();
            assert_eq!(prototype.get_property("x"), Some(Value::Nil));
            match prototype.get_property("describe") {
                Some(Value::Function(function)) => {
                    let function = function.read().unwrap();
                    assert_eq!(function.name, "<memory>.Point.describe");
                    assert!(function.instructions.is_empty());
                }
                other => panic!("expected origin method on prototype, got {other:?}"),
            }
            match prototype.get_property("to_string") {
                Some(Value::Function(function)) => {
                    let function = function.read().unwrap();
                    assert_eq!(function.name, "<memory>.Point.#ToString.to_string");
                    assert!(function.instructions.is_empty());
                }
                other => panic!("expected protocol method on prototype, got {other:?}"),
            }
        }

        let mut encoded = Vec::new();
        vmm::bytecode::serde::write_bytecode(&mut encoded, &bytecode)
            .expect("serialize struct bytecode");
        let bytecode = vmm::bytecode::serde::read_bytecode(&mut encoded.as_slice())
            .expect("deserialize struct bytecode");
        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(
            vm.entrypoint_return(),
            Some(Value::String("Point prototype method 40".to_string()))
        );
    }

    #[test]
    fn test_struct_constructor_pattern_destructures_fields() {
        let src = r#"
        struct Point:
            x: Int
            y: Int

        fn main() -> String:
            let point = Point(40, 2)
            return match point:
                Point(x, y) => "Point x=${x}, y=${y}"
                _ => "not a point"
        "#;
        let bytecode = crate::compile(src).expect("compile struct pattern");
        let mut vm = IonVM::new(bytecode).expect("initialize VM");
        vm.run().expect("run VM");
        assert_eq!(
            vm.entrypoint_return(),
            Some(Value::String("Point x=40, y=2".to_string()))
        );
    }

    #[test]
    fn test_primitive_format_method_uses_vm_formatter() {
        let src = r#"
        fn main() -> String:
            return 255.format("08x")
        "#;
        let bytecode = crate::compile(src).expect("compile primitive format call");
        let mut vm = IonVM::new(bytecode).expect("initialize VM");
        vm.run().expect("run VM");
        assert_eq!(
            vm.entrypoint_return(),
            Some(Value::String("000000ff".to_string()))
        );
    }

    #[test]
    fn test_struct_to_string_protocol_method_is_bound_to_receiver() {
        let src = r#"
        struct Point:
            x: Int

        impl ToString for Point:
            fn to_string() -> String:
                return "Point x=${self.x}"

        fn main() -> String:
            let point = Point(42)
            return point.to_string()
        "#;
        let bytecode = crate::compile(src).expect("compile ToString implementation");
        let mut vm = IonVM::new(bytecode).expect("initialize VM");
        vm.run().expect("run VM");
        assert_eq!(
            vm.entrypoint_return(),
            Some(Value::String("Point x=42".to_string()))
        );
    }

    #[test]
    fn test_jit_string_return_value_is_copied() {
        let src = r#"
        fn message() -> String: {
            return "hello from JIT";
        }

        fn main() -> String: {
            return message();
        }
        "#;
        let bytecode = crate::compile(src).expect("compile string-returning function");
        let mut vm = IonVM::new(bytecode).expect("initialize VM");
        let message = vm
            .scheduler
            .bytecode
            .functions
            .iter()
            .find(|function| function.name.ends_with(".message"))
            .expect("message function exists");
        assert!(message.jit_compiled_fn.is_some(), "message should use JIT");
        vm.run().expect("run VM");
        assert_eq!(
            vm.entrypoint_return(),
            Some(Value::String("hello from JIT".to_string()))
        );
    }

    #[test]
    fn test_struct_constructor_validates_field_count_and_types() {
        let missing_field = r#"
        struct Point: { x: Int }
        fn main() -> Int: {
            let point = Point();
            return 0;
        }
        "#;
        assert!(crate::compile(missing_field).is_err());

        let wrong_field_type = r#"
        struct Point: { x: Int }
        fn main() -> Int: {
            let point = Point("not an int");
            return 0;
        }
        "#;
        assert!(crate::compile(wrong_field_type).is_err());
    }

    #[test]
    fn test_codegen_and_run_simple_add() {
        let src = r#"
        fn main() -> Int: {
            let a = 40;
            let b = 2;
            return a + b;
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let bytecode = CodeEmitter::emit_program(&ssa);
        let mut out = Vec::new();
        let _ = vmm::bytecode::disasm::print_disassembly(&mut out, &bytecode);
        println!("{}", String::from_utf8_lossy(&out));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(vm.entrypoint_return(), Some(Value::Int(42)));
    }

    #[test]
    fn test_codegen_and_run_function_returning_closure() {
        let src = r#"
        fn make_multiplier(factor: Int) -> fn(Int) -> Int: {
            return fn(x: Int) -> Int: {
                return x * factor;
            };
        }

        fn main() -> Int: {
            let double = make_multiplier(2);
            let triple = make_multiplier(3);
            let r1 = double(10);
            let r2 = triple(10);
            return r1 + r2;
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let bytecode = CodeEmitter::emit_program(&ssa);

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(vm.entrypoint_return(), Some(Value::Int(50)));
    }

    #[test]
    fn test_codegen_and_run_closure_capturing_value() {
        let src = r#"
        fn main() -> Int: {
            let x = 40;
            let add_x = fn(y: Int) -> Int: {
                return x + y;
            };
            return add_x(2);
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let bytecode = CodeEmitter::emit_program(&ssa);
        let mut out = Vec::new();
        let _ = vmm::bytecode::disasm::print_disassembly(&mut out, &bytecode);
        println!("{}", String::from_utf8_lossy(&out));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(vm.entrypoint_return(), Some(Value::Int(42)));
    }

    #[test]
    fn test_codegen_and_run_mixed_array() {
        let src = r#"
        fn main() -> Float: {
            let a = [1, 2, 3.0, 4];
            return a[2];
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(vm.entrypoint_return(), Some(Value::Float(3.0)));
    }

    #[test]
    fn test_codegen_and_run_const_statements() {
        let src = r#"
        const G = 100;
        fn main() -> Int: {
            const a = 42;
            return G + a;
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(vm.entrypoint_return(), Some(Value::Int(142)));
    }

    #[test]
    fn test_codegen_and_run_simd_float_array_vector_add() {
        let src = r#"
        fn main(): {
            let a = [1.0, 2.0, 3.0, 4.0];
            let b = [10.0, 20.0, 30.0, 40.0];
            let c = a + b;
            return c;
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        if let Some(Value::FloatArray(arr)) = vm.entrypoint_return() {
            let r = arr.read().unwrap();
            assert_eq!(&*r, &[11.0, 22.0, 33.0, 44.0]);
        } else {
            panic!("Expected FloatArray return, got {:?}", vm.entrypoint_return());
        }
    }

    #[test]
    fn test_codegen_and_run_simd_loop_auto_vectorization() {
        let src = r#"
        fn main(): {
            let limit = 10;
            let arr = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            let i = 0;
            while i < limit: {
                arr[i] = i * 2;
                i = i + 1;
            };
            return arr[4];
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(vm.entrypoint_return(), Some(Value::Int(8)));
    }

    #[test]
    fn test_codegen_and_run_indented_function_syntax() {
        let src = r#"
        fn main():
            const a = 42
            let b = a * 3
            return b - a
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(vm.entrypoint_return(), Some(Value::Int(84)));
    }

    #[test]
    fn test_codegen_and_run_indented_closures_in_array() {
        let src = r#"
        fn main():
            let funcs = [
                fn(x: Int) -> Int:
                    let y = x + 10
                    return y,
                fn(x: Int) -> Int:
                    let z = x * 3
                    return z,
            ]
            let f0 = funcs[0]
            let f1 = funcs[1]
            return f0(5) + f1(5)
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(vm.entrypoint_return(), Some(Value::Int(30)));
    }

    #[test]
    fn test_codegen_shared_closure_environment_mutation() {
        let src = r#"
        fn make_closures(x: Int):
            let m = x
            return [
                fn(y: Int):
                    return m + y,
                fn(y: Int):
                    m = m + y
                    return m
            ]

        fn main() -> Int:
            let closures = make_closures(5)
            let a = closures[0](2)
            let b = closures[1](2)
            let c = closures[0](2)
            return a + b + c
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        // a = 7, b = 7, c = 9 => 7 + 7 + 9 = 23
        assert_eq!(vm.entrypoint_return(), Some(Value::Int(23)));
    }

    #[test]
    fn test_codegen_and_run_actor_spawn_send_receive() {
        let src = r#"
        fn worker(main_pid: Int):
            let msg = receive()
            send(main_pid, msg * 2)

        fn main() -> Int:
            let worker_pid = spawn worker(0)
            send(worker_pid, 21)
            let reply = receive()
            return reply
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(vm.entrypoint_return(), Some(Value::Int(42)));
    }

    #[test]
    fn test_codegen_and_run_string_and_json_builtins() {
        let src = r#"
        fn main() -> String:
            let s = "  hello:world:mesh  "
            let trimmed = str_trim(s)
            let parts = str_split(trimmed, ":")
            let joined = str_replace(trimmed, ":", " ")
            return joined
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        assert_eq!(vm.entrypoint_return(), Some(Value::String("hello world mesh".to_string())));
    }

    #[test]
    fn test_codegen_and_run_json_stringify() {
        let src = r#"
        fn main() -> String:
            let payload = json_parse("{\"name\":\"mesh\",\"version\":\"0.1.0\"}")
            let s = json_stringify(payload)
            return s
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);
        let mut inferrer = crate::TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("types inferred");
        let bytecode = CodeEmitter::emit_program_with_types(&ssa, Some(&type_maps));

        let mut vm = IonVM::new(bytecode).expect("init vm");
        vm.run().expect("run vm");
        let ret = vm.entrypoint_return().unwrap();
        match ret {
            Value::String(s) => {
                assert!(s.contains("\"name\":\"mesh\""));
                assert!(s.contains("\"version\":\"0.1.0\""));
            }
            _ => panic!("Expected String"),
        }
    }
}

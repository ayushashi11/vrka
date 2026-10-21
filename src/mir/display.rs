use super::ssa::*;
use std::fmt;

impl fmt::Display for SsaOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SsaOp::Const(lit) => match lit {
                crate::syntax::cst::LitVal::Int(i) => write!(f, "const.int {}", i),
                crate::syntax::cst::LitVal::Float(flt) => write!(f, "const.float {}", flt),
                crate::syntax::cst::LitVal::String(s) => write!(f, "const.str \"{}\"", s),
                crate::syntax::cst::LitVal::Bool(b) => write!(f, "const.bool {}", b),
                crate::syntax::cst::LitVal::Unit => write!(f, "const.unit"),
                crate::syntax::cst::LitVal::Nil => write!(f, "const.nil"),
            },
            SsaOp::BuildString(parts) => {
                write!(f, "build_string [")?;
                for (i, p) in parts.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match p {
                        StringPartValue::Literal(s) => write!(f, "\"{}\"", s)?,
                        StringPartValue::Value(v) => write!(f, "{}", v)?,
                    }
                }
                write!(f, "]")
            }
            SsaOp::Binary(op, l, r) => write!(f, "{:?} {}, {}", op, l, r),
            SsaOp::Unary(op, opnd) => write!(f, "{:?} {}", op, opnd),
            SsaOp::Call { callee, args } => {
                write!(f, "call {}(", callee)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }
            SsaOp::FieldAccess { base, field } => write!(f, "{}.{}", base, field),
            SsaOp::Index { base, index } => write!(f, "{}[{}]", base, index),
            SsaOp::StoreField { base, field, val } => write!(f, "store {}.{} = {}", base, field, val),
            SsaOp::StoreIndex { base, index, val } => write!(f, "store {}[{}] = {}", base, index, val),
            SsaOp::ArrayLit(elems) => {
                write!(f, "array [")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, "]")
            }
            SsaOp::TupleLit(elems) => {
                write!(f, "tuple (")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, ")")
            }
            SsaOp::Spawn { callee, args } => {
                write!(f, "spawn {}(", callee)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }
            SsaOp::Send { receiver, message } => write!(f, "send {}, {}", receiver, message),
            SsaOp::Receive => write!(f, "receive"),
            SsaOp::GlobalRef(name) => write!(f, "global @{}", name),
            SsaOp::QualifiedGlobal(path) => write!(f, "global @{}", path.join(".")),
            SsaOp::Atom(tag) => write!(f, ":{}", tag),
            SsaOp::Upvalue(idx) => write!(f, "upval({})", idx),
            SsaOp::StoreUpvalue { index, val } => write!(f, "store upval({}) = {}", index, val),
            SsaOp::MakeEnv(captured_values) => {
                write!(f, "make_env [")?;
                for (i, v) in captured_values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            SsaOp::MakeClosure { func, env } => {
                if let Some(e) = env {
                    write!(f, "make_closure @{}, {}", func, e)
                } else {
                    write!(f, "make_closure @{}", func)
                }
            },
            SsaOp::FormatValue { val, spec } => write!(f, "format_value {}, {}", val, spec.map_or_else(|| String::from("<no spec>"), |v| format!("{}", v))),
        }
    }
}

impl fmt::Display for SsaTerminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SsaTerminator::Return(val, _) => {
                if let Some(v) = val {
                    write!(f, "ret {}", v)
                } else {
                    write!(f, "ret void")
                }
            }
            SsaTerminator::Goto(bb, _) => write!(f, "br {}", bb),
            SsaTerminator::Branch { cond, then_bb, else_bb, .. } => {
                write!(f, "br {}, {}, {}", cond, then_bb, else_bb)
            }
            SsaTerminator::Match { target, arms, default_bb, .. } => {
                writeln!(f, "match {} [", target)?;
                for arm in arms {
                    writeln!(f, "    {:?} => {},", arm.pattern, arm.target_bb)?;
                }
                write!(f, "    default => {}", default_bb)?;
                write!(f, "\n  ]")
            }
            SsaTerminator::Unreachable(_) => write!(f, "unreachable"),
        }
    }
}

impl fmt::Display for SsaFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "fn {}(", self.name)?;
        for (i, (name, val, _)) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", name, val)?;
        }
        writeln!(f, "):")?;

        for bb in &self.blocks {
            writeln!(f, "{}:", bb.label)?;
            for phi in &bb.phis {
                write!(f, "  {} = phi [", phi.dst)?;
                for (i, (b, v)) in phi.incoming.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}, {}", v, b)?;
                }
                writeln!(f, "]")?;
            }
            for inst in &bb.instructions {
                if let Some(dst) = inst.dst {
                    writeln!(f, "  {} = {}", dst, inst.op)?;
                } else {
                    writeln!(f, "  {}", inst.op)?;
                }
            }
            writeln!(f, "  {}", bb.terminator)?;
        }
        Ok(())
    }
}

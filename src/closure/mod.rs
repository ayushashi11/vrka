use crate::mir::ssa::SsaProgram;
use crate::sema::{ResolvedProgram, VarKind};
use crate::types::Type;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub enum CaptureKind {
    /// Read-only variable captured directly by value (copied into closure env)
    ByValue,
    /// Mutable variable that can be re-assigned; captured via a shared heap box/cell
    ByCell,
}

#[derive(Debug, Clone)]
pub struct ClosureCapture {
    pub name: String,
    pub kind: CaptureKind,
    pub parent_index: usize,
    pub is_direct_local: bool,
    pub ty: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct ClosureEnv {
    pub fn_name: String,
    pub captures: Vec<ClosureCapture>,
}

#[derive(Debug, Clone)]
pub struct ClosureAnalysis {
    pub closures: Vec<ClosureEnv>,
}

impl ClosureAnalysis {
    pub fn build(
        resolved: &ResolvedProgram,
        _ssa: &SsaProgram,
        type_maps: &HashMap<String, HashMap<crate::mir::ssa::ValueId, Type>>,
    ) -> Self {
        let mut closures = Vec::new();

        for func in &resolved.functions {
            if func.upvalues.is_empty() {
                continue;
            }

            let mut captures = Vec::new();
            let func_types = type_maps.get(&func.name);

            for upval in &func.upvalues {
                let kind = match upval.kind {
                    VarKind::Constant => CaptureKind::ByValue,
                    VarKind::Upvalue { is_const, .. } => {
                        if is_const {
                            CaptureKind::ByValue
                        } else {
                            CaptureKind::ByCell
                        }
                    }
                    VarKind::Mutable => CaptureKind::ByCell,
                };

                let ty = func_types
                    .and_then(|types| types.values().find(|t| !matches!(t, Type::Var(_))).cloned())
                    .or_else(|| {
                        type_maps.values().find_map(|types| {
                            types.iter().find_map(|(_, t)| {
                                if matches!(
                                    t,
                                    Type::Int
                                        | Type::Float
                                        | Type::Bool
                                        | Type::String
                                        | Type::IntArray
                                        | Type::FloatArray
                                ) {
                                    Some(t.clone())
                                } else {
                                    None
                                }
                            })
                        })
                    });

                captures.push(ClosureCapture {
                    name: upval.name.clone(),
                    kind,
                    parent_index: upval.parent_index,
                    is_direct_local: upval.is_direct_local,
                    ty,
                });
            }

            closures.push(ClosureEnv {
                fn_name: func.name.clone(),
                captures,
            });
        }

        Self { closures }
    }
}

impl fmt::Display for ClosureAnalysis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.closures.is_empty() {
            writeln!(f, "No closures with captured environments.")?;
            return Ok(());
        }

        for closure in &self.closures {
            writeln!(f, "Closure Environment for '{}':", closure.fn_name)?;
            for (i, cap) in closure.captures.iter().enumerate() {
                let kind_str = match cap.kind {
                    CaptureKind::ByValue => "by-value (immutable copy)",
                    CaptureKind::ByCell => "by-cell (mutable ref box)",
                };
                let origin_str = if cap.is_direct_local {
                    format!("parent local slot #{}", cap.parent_index)
                } else {
                    format!("parent upvalue slot #{}", cap.parent_index)
                };
                let type_str = cap
                    .ty
                    .as_ref()
                    .map(|t| format!(": {}", t))
                    .unwrap_or_default();

                writeln!(
                    f,
                    "  [{}] '{}{}' - {} from {}",
                    i, cap.name, type_str, kind_str, origin_str
                )?;
            }
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
    use crate::types::TypeInferrer;

    #[test]
    fn test_function_returning_two_closures_capturing_one_value() {
        let src = r#"
        fn make_pair(x: Int): {
            let f1 = fn(): { return x + 1; };
            let f2 = fn(): { return x + 2; };
            return [f1, f2];
        }
        "#;
        let module = parse(src).expect("parsed");
        let resolved = resolve(&module).expect("resolved");
        let ssa = build_ssa_program(&resolved, &module);

        let mut inferrer = TypeInferrer::new();
        inferrer.register_module_declarations(&module);
        let type_maps = inferrer.infer_program(&ssa).expect("inferred types");

        let analysis = ClosureAnalysis::build(&resolved, &ssa, &type_maps);
        assert_eq!(analysis.closures.len(), 2, "Must detect exactly 2 closures");

        for c in &analysis.closures {
            assert_eq!(
                c.captures.len(),
                1,
                "Each closure must capture exactly 1 value"
            );
            assert_eq!(c.captures[0].name, "x");
            assert!(c.captures[0].is_direct_local);
            assert!(matches!(c.captures[0].kind, CaptureKind::ByValue));
            assert_eq!(c.captures[0].ty, Some(Type::Int));
        }
    }
}

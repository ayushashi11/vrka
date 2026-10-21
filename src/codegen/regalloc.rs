use crate::mir::ssa::{SsaFunction, ValueId};
use std::collections::{HashMap, HashSet};

pub struct RegAlloc {
    pub map: HashMap<ValueId, usize>,
    pub total_registers: usize,
}

impl RegAlloc {
    pub fn allocate(func: &SsaFunction) -> Self {
        let all: HashSet<ValueId> = func.values.iter().map(|v| v.id).collect();
        Self::allocate_with_needed(func, &all)
    }

    pub fn allocate_with_needed(func: &SsaFunction, needed: &HashSet<ValueId>) -> Self {
        let mut map = HashMap::new();
        let mut next_reg = 0;

        // Allocate registers for function parameters first
        for (_, val_id, _) in &func.params {
            map.insert(*val_id, next_reg);
            next_reg += 1;
        }

        // Allocate registers only for needed SSA values
        for val in &func.values {
            if needed.contains(&val.id) && !map.contains_key(&val.id) {
                map.insert(val.id, next_reg);
                next_reg += 1;
            }
        }

        Self {
            map,
            total_registers: next_reg.max(func.params.len()),
        }
    }

    pub fn get(&self, val: ValueId) -> usize {
        *self.map.get(&val).unwrap_or(&0)
    }

    pub fn get_opt(&self, val: ValueId) -> Option<usize> {
        self.map.get(&val).copied()
    }
}

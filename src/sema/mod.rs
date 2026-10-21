pub mod error;
pub mod loader;
pub mod resolve;
pub mod scope;
pub mod symbol;

pub use error::SemaError;
pub use loader::ModuleLoader;
pub use resolve::{ResolvedFunction, ResolvedProgram, Resolver};
pub use scope::{FunctionScope, LocalInfo, ScopeManager, UpvalueDesc};
pub use symbol::{ResolvedVar, SymbolId, VarKind};

pub fn resolve(module: &crate::syntax::cst::Module) -> Result<ResolvedProgram, SemaError> {
    let mut resolver = Resolver::new(module.path.to_string_lossy().to_string());
    resolver.resolve_module(module)
}

pub fn resolve_with_loader(
    module: &crate::syntax::cst::Module,
    loader: ModuleLoader,
) -> Result<ResolvedProgram, SemaError> {
    let mut resolver = Resolver::with_loader(module.path.to_string_lossy().to_string(), loader);
    resolver.resolve_module(module)
}

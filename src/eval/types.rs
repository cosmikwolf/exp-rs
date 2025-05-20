extern crate alloc;
use crate::Real;

#[cfg(not(test))]
use alloc::rc::Rc;
use alloc::string::{String, ToString};
#[cfg(test)]
use std::rc::Rc;

pub struct OwnedNativeFunction {
    pub arity: usize,
    pub implementation: Rc<dyn Fn(&[Real]) -> Real>,
    pub name: String, // Fully owned String instead of Cow
    pub description: Option<String>,
}

// Convert from NativeFunction<'a> to OwnedNativeFunction
impl<'a> From<&crate::types::NativeFunction<'a>> for OwnedNativeFunction {
    fn from(nf: &crate::types::NativeFunction<'a>) -> Self {
        OwnedNativeFunction {
            arity: nf.arity,
            implementation: nf.implementation.clone(),
            name: nf.name.to_string(), // Convert Cow to String
            description: nf.description.clone(),
        }
    }
}

pub enum FunctionCacheEntry {
    Native(OwnedNativeFunction),
    Expression(crate::types::ExpressionFunction),
    User(crate::context::UserFunction),
}

impl Clone for FunctionCacheEntry {
    fn clone(&self) -> Self {
        match self {
            FunctionCacheEntry::Native(nf) => FunctionCacheEntry::Native(OwnedNativeFunction {
                arity: nf.arity,
                implementation: nf.implementation.clone(),
                name: nf.name.clone(),
                description: nf.description.clone(),
            }),
            FunctionCacheEntry::Expression(ef) => FunctionCacheEntry::Expression(ef.clone()),
            FunctionCacheEntry::User(uf) => FunctionCacheEntry::User(uf.clone()),
        }
    }
}

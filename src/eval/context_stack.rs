//! Context stack management for iterative evaluation
//!
//! This module provides a non-recursive way to manage evaluation contexts
//! during iterative AST evaluation.

use crate::context::EvalContext;
use crate::types::{HString};
use crate::error::ExprError;
use crate::Real;
use alloc::vec::Vec;
use alloc::rc::Rc;
use heapless::FnvIndexMap;

/// Maximum number of contexts we can track
const MAX_CONTEXTS: usize = 128;

/// Manages evaluation contexts without recursion
pub struct ContextStack {
    /// Stack of contexts, indexed by ID
    contexts: Vec<Option<ContextWrapper>>,
    /// Next available context ID
    next_id: usize,
    /// Maps context IDs to their parent IDs for variable lookup
    parent_map: FnvIndexMap<usize, Option<usize>, MAX_CONTEXTS>,
}

/// Wrapper for context with additional metadata
struct ContextWrapper {
    /// The actual context
    context: Rc<EvalContext>,
    /// Whether this context owns its data (vs being a reference)
    is_owned: bool,
}

impl ContextStack {
    /// Create a new context stack
    pub fn new() -> Self {
        Self {
            contexts: Vec::with_capacity(8),
            next_id: 0,
            parent_map: FnvIndexMap::new(),
        }
    }
    
    /// Clear the stack while preserving capacity
    pub fn clear(&mut self) {
        self.contexts.clear();
        self.next_id = 0;
        self.parent_map.clear();
    }
    
    /// Push a context onto the stack, returning its ID
    pub fn push_context(&mut self, ctx: Option<Rc<EvalContext>>) -> Result<usize, ExprError> {
        let id = self.next_id;
        
        // Check capacity
        if id >= MAX_CONTEXTS {
            return Err(ExprError::CapacityExceeded("context stack"));
        }
        
        self.next_id += 1;
        
        // Ensure vector has space
        if self.contexts.len() <= id {
            self.contexts.resize_with(id + 1, || None);
        }
        
        // Store context
        if let Some(ctx) = ctx {
            self.contexts[id] = Some(ContextWrapper {
                context: ctx,
                is_owned: false,
            });
        } else {
            // Create default context
            self.contexts[id] = Some(ContextWrapper {
                context: Rc::new(EvalContext::default()),
                is_owned: true,
            });
        }
        
        // No parent by default
        self.parent_map.insert(id, None)
            .map_err(|_| ExprError::CapacityExceeded("parent map"))?;
        
        Ok(id)
    }
    
    /// Push a new context with a specified parent
    pub fn push_context_with_parent(
        &mut self, 
        ctx: EvalContext, 
        parent_id: usize
    ) -> Result<usize, ExprError> {
        let id = self.next_id;
        
        // Check capacity
        if id >= MAX_CONTEXTS {
            return Err(ExprError::CapacityExceeded("context stack"));
        }
        
        self.next_id += 1;
        
        // Ensure vector has space
        if self.contexts.len() <= id {
            self.contexts.resize_with(id + 1, || None);
        }
        
        // Store context
        self.contexts[id] = Some(ContextWrapper {
            context: Rc::new(ctx),
            is_owned: true,
        });
        
        // Set parent relationship
        self.parent_map.insert(id, Some(parent_id))
            .map_err(|_| ExprError::CapacityExceeded("parent map"))?;
        
        Ok(id)
    }
    
    /// Get a context by ID
    pub fn get_context(&self, id: usize) -> Option<&Rc<EvalContext>> {
        self.contexts.get(id)
            .and_then(|opt| opt.as_ref())
            .map(|wrapper| &wrapper.context)
    }
    
    /// Look up a variable, checking parent contexts if needed
    pub fn lookup_variable(&self, ctx_id: usize, name: &HString) -> Option<Real> {
        let mut current_id = Some(ctx_id);
        
        while let Some(id) = current_id {
            if let Some(ctx) = self.get_context(id) {
                // Parameters are stored in the context, we need to handle this differently
                // For now, skip parameter checking as it's not exposed in EvalContext
                
                // Then check regular variables
                if let Some(&value) = ctx.variables.get(name) {
                    return Some(value);
                }
                
                // Check constants
                if let Some(&value) = ctx.constants.get(name) {
                    return Some(value);
                }
            }
            
            // Move to parent context
            current_id = self.parent_map.get(&id).and_then(|&parent| parent);
        }
        
        None
    }
    
    /// Get the parent ID of a context
    pub fn get_parent_id(&self, ctx_id: usize) -> Option<usize> {
        self.parent_map.get(&ctx_id).and_then(|&parent| parent)
    }
}
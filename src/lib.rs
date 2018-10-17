
extern crate cranelift_codegen;
extern crate cranelift_wasm;
extern crate cranelift_entity;
extern crate target_lexicon;

pub mod backend;
mod wasm_compile;

use backend::Backend;

// use wasm::{Compilation, Module, DataInitializer};

pub struct Nucleus<B: Backend> {
    backend: B,
    // wasm_modules: Vec<(Compilation, Module, DataInitializer)>,
}

impl<B: Backend> Nucleus<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            // wasm_modules: Vec::new(),
        }
    }
}
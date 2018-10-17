use cranelift_codegen::cursor::FuncCursor;
use cranelift_codegen::ir;
use cranelift_codegen::ir::immediates::{Imm64, Offset32};
use cranelift_codegen::ir::types::*;
use cranelift_codegen::ir::{
    AbiParam, ArgumentPurpose, ExtFuncData, ExternalName, Function, InstBuilder, Signature,
};
use cranelift_codegen::isa;
use cranelift_codegen::settings;
use cranelift_entity::EntityRef;
use cranelift_wasm::{
    self, translate_module, FuncIndex, Global, GlobalIndex, GlobalVariable, Memory, MemoryIndex,
    SignatureIndex, Table, TableIndex, WasmResult,
};
use module::{DataInitializer, Export, LazyContents, Module, TableElements};
use target_lexicon::Triple;

/// Compute a `ir::ExternalName` for a given wasm function index.
pub fn get_func_name(func_index: FuncIndex) -> ir::ExternalName {
    debug_assert!(FuncIndex::new(func_index.index() as u32 as usize) == func_index);
    ir::ExternalName::user(0, func_index.index() as u32)
}

pub struct ModuleEnvironment<'data, 'module> {
    isa: &'module isa::TargetIsa,
    module: &'module mut Module,
    lazy: LazyContents<'data>,
}

impl<'data, 'module> ModuleEnvironment<'data, 'module> {
    pub fn new(isa: &'module isa::TargetIsa, module: &'module mut Module) -> Self {
        Self {
            isa,
            module,
            lazy: LazyContents::new(),
        }
    }

    fn func_env(&self) -> FuncEnvironment {
        FuncEnvironment::new(self.isa, self.module)
    }

    fn pointer_type(&self) -> ir::Type {
        use cranelift_wasm::FuncEnvironment;
        self.func_env().pointer_type()
    }

    pub fn translate(mut self, data: &'data [u8]) -> WasmResult<ModuleTranslation<'data, 'module>> {
        translate_module(data, &mut self)?;

        Ok(ModuleTranslation {
            isa: self.isa,
            module: self.module,
            lazy: self.lazy,
        })
    }
}

pub struct FuncEnvironment<'mod_env> {
    isa: &'mod_env isa::TargetIsa,
    module: &'mod_env Module,

    vmctx_base: Option<ir::GlobalValue>,
    globals_base: Option<ir::GlobalValue>,
    memory_list_base: Option<ir::GlobalValue>,
    memory_bases: Vec<Option<ir::GlobalValue>>,

    current_memory_extfunc: Option<ir::FuncRef>,
    grow_memory_extfunc: Option<ir::FuncRef>,
}

impl<'mod_env> FuncEnvironment<'mod_env> {
    pub fn new(isa: &isa::TargetIsa, module: &Module) -> Self {
        Self {
            isa,
            module,
            vmctx_base: None,
            globals_base: None,
            memory_list_base: None,
            memory_bases: vec![None; module.memories.len()],
            current_memory_extfunc: None,
            grow_memory_extfunc: None,
        }
    }

    /// Transform the call argument list in preparation for making a call.
    /// This pushes the VMContext into the args list.
    fn get_real_call_args(func: &Function, call_args: &[ir::Value]) -> Vec<ir::Value> {
        let mut real_call_args = Vec::with_capacity(call_args.len() + 1);
        real_call_args.extend_from_slice(call_args);
        real_call_args.push(func.special_param(ArgumentPurpose::VMContext).unwrap());
        real_call_args
    }

    fn pointer_bytes(&self) -> usize {
        usize::from(self.isa.pointer_bytes())
    }

    fn vmctx_global(&self, func: &mut ir::Function) -> ir::GlobalValue {
        self.vmctx_base.unwrap_or_else(|| {
            let vmctx = func.create_global_value(ir::GlobalValueData::VMContext);
            self.vmctx_base = Some(vmctx);
            vmctx
        })
    }
}

impl<'data, 'module> cranelift_wasm::ModuleEnvironment<'data> for ModuleEnvironment<'data, 'module> {

    fn get_func_name(&self, func_index: FuncIndex) -> ir::ExternalName {
        get_func_name(func_index)
    }

    fn flags(&self) -> &settings::Flags {
        self.isa.flags()
    }

    fn declare_signature(&mut self, sig: &ir::Signature) {
        let mut sig = sig.clone();
        sig.params.push(AbiParam::special(
            self.pointer_type(),
            ArgumentPurpose::VMContext,
        ));
        self.module.signatures.push(sig);
    }

    fn get_signature(&self, sig_index: SignatureIndex) -> &ir::Signature {
        &self.module.signatures[sig_index]
    }

    fn declare_func_import(&mut self, sig_index: SignatureIndex, module: &str, field: &str) {
        debug_assert_eq!(
            self.module.functions.len(),
            self.module.imported_funcs.len(),
            "Imported functions must be declared first"
        );

        self.module.functions.push(sig_index);
        self.module
            .imported_funcs
            .push((module.to_string(), field.to_string()));
    }

    fn get_num_func_imports(&self) -> usize {
        self.module.imported_funcs.len()
    }

    fn declare_func_type(&mut self, sig_index: SignatureIndex) {
        self.module.functions.push(sig_index);
    }

    fn get_func_type(&self, func_index: FuncIndex) -> SignatureIndex {
        self.module.functions[func_index]
    }

    fn declare_global(&mut self, global: Global) {
        self.module.globals.push(global);
    }

    fn get_global(&self, global_index: GlobalIndex) -> &Global {
        &self.module.globals[global_index]
    }

    fn declare_table(&mut self, table: Table) {
        self.module.table.push(table);
    }

    fn declare_table_elements(&mut self, table_index: TableIndex, base: Option<GlobalIndex>, offset: usize, elements: Vec<FuncIndex>) {
        debug_assert!(base.is_none(), "global-value offsets not supported yet");
        self.module.table_elements.push(TableElements {
            table_index,
            base,
            offset,
            elements,
        });
    }

    fn declare_memory(&mut self, memory: Memory) {
        self.module.memories.push(memory);
    }

    fn declare_data_initialization(&mut self, memory_index: MemoryIndex, base: Option<GlobalIndex>, offset: usize, data: &'data [u8]) {
        debug_assert!(base.is_none(), "global-value offsets not supported yet");
        self.lazy.data_initializers.push(DataInitializer {
            memory_index,
            base,
            offset,
            data,
        });
    }

    fn declare_func_export(&mut self, func_index: FuncIndex, name: &str) {
        self.module
            .exports
            .insert(name.to_string(), Export::Function(func_index));
    }

    fn declare_table_export(&mut self, table_index: TableIndex, name: &str) {
        self.module
            .exports
            .insert(name.to_string(), Export::Table(table_index));
    }

    fn declare_memory_export(&mut self, memory_index: MemoryIndex, name: &str) {
        self.module
            .exports
            .insert(name.to_string(), Export::Memory(memory_index));
    }

    fn declare_global_export(&mut self, global_index: GlobalIndex, name: &str) {
        self.module
            .exports
            .insert(name.to_string(), Export::Global(global_index));
    }

    fn declare_start_func(&mut self, func_index: FuncIndex) {
        debug_assert!(self.module.start_func.is_none());
        self.module.start_func = Some(func_index);
    }

    fn define_function_body(&mut self, body_bytes: &'data [u8]) -> WasmResult<()> {
        self.lazy.func_body_inputs.push(body_bytes);
        Ok(())
    }
}

impl<'mod_env> cranelift_wasm::FuncEnvironment for FuncEnvironment<'mod_env> {
    fn flags(&self) -> &settings::Flags {
        &self.isa.flags()
    }

    fn triple(&self) -> &Triple {
        self.isa.triple()
    }

    fn make_global(&mut self, func: &mut ir::Function, index: GlobalIndex) -> GlobalVariable {
        let globals_base = self.globals_base.unwrap_or_else(|| {
            let vmctx = self.vmctx_global(func);
            let new_base = func.create_global_value(ir::GlobalValueData::Load {
                base: vmctx,
                offset: 0.into(),
                global_type: self.pointer_type(),
            });
            self.globals_base = Some(new_base);
            new_base
        });

        let offset = index * self.pointer_bytes();

        let gv = func.create_global_value(ir::GlobalValueData::Load {
            base: globals_base,
            offset: (offset as i32).into(),
            global_type: self.pointer_type(),
        });

        GlobalVariable::Memory {
            gv,
            ty: self.module.globals[index].ty,
        }
    }

    fn make_heap(&mut self, func: &mut ir::Function, index: MemoryIndex) -> ir::Heap {
        let heap_addr = *self.memory_bases[index].get_or_insert_with(|| {
            let memory_list_base = *self.memory_list_base.get_or_insert_with(|| {
                let vmctx = self.vmctx_global(func);
                func.create_global_value(ir::GlobalValueData::Load {
                    base: vmctx,
                    offset: (self.pointer_bytes() as i32).into(),
                    global_type: self.pointer_type(),
                })
            });
            let offset = index * self.pointer_bytes();
            func.create_global_value(ir::GlobalValueData::Load {
                base: memory_list_base,
                offset: (offset as i32).into(),
                global_type: self.pointer_type(),
            })
        });

        func.create_heap(ir::HeapData {
            base: heap_addr,
            min_size: 0.into(),
            guard_size: 0x8000_0000.into(),
            style: ir::HeapStyle::Static {
                bound: 0x1_0000_0000.into(),
            },
            index_type: I32,
        })
    }

    fn make_table(&mut self, func: &mut ir::Function, index: TableIndex) -> ir::Table {
        
    }
}
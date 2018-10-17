use cranelift_codegen::binemit;
use cranelift_codegen::ir;
use cranelift_codegen::ir::ExternalName;
use cranelift_codegen::isa;
use cranelift_codegen::Context;
use cranelift_entity::{EntityRef, PrimaryMap};
use cranelift_wasm::{DefinedFuncIndex, FuncIndex, FuncTranslator};
use environ::{get_func_name, ModuleTranslation};

mod environ;

pub struct Compilation {
    funcs: PrimaryMap<DefinedFuncIndex, Vec<u8>>,
}

impl Compilation {
    pub fn new(funcs: PrimaryMap<DefinedFuncIndex, Vec<u8>>) -> Self {
        Compilation {
            funcs,
        }
    }
}

pub struct RelocSink {
    func_relocs: Vec<Relocation>,
}

impl binemit::RelocSink for RelocSink {
    fn reloc_ebb(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _ebb_offset: binemit::CodeOffset,
    ) {
        // This should use the `offsets` field of `ir::Function`.
        unimplemented!();
    }

    fn reloc_external(
        &mut self,
        offset: binemit::CodeOffset,
        reloc: binemit::Reloc,
        name: &ExternalName,
        addend: binemit::Addend,
    ) {
        let reloc_target = match *name {
            ExternalName::User {
                namespace,
                index,
            } => {
                debug_assert!(namespace == 0);
                RelocationTarget::UserFunc(FuncIndex::new(index as _))
            },
            ExternalName::testcase("grow_memory") => {
                RelocationTarget::GrowMemory
            },
            ExternalName::testcase("current_memory") => {
                RelocationTarget::CurrentMemory
            },
            _ => panic!("unknown external name"),
        };

        self.func_relocs.push(Relocation {
            reloc,
            reloc_target,
            offset,
            addend,
        });
    }

    fn reloc_jt(
        &mut self,
        _offset: binemit::CodeOffset,
        _reloc: binemit::Reloc,
        _jt: ir::JumpTable,
    ) {
        unimplemented!();
    }
}

impl RelocSink {
    fn new() -> Self {
        Self {
            func_relocs: Vec::new(),
        }
    }
}

pub struct Relocation {
    /// The relocation code.
    pub reloc: binemit::Reloc,
    /// The relocation target
    pub reloc_target: RelocationTarget,
    /// The offset where to apply the relocation.
    pub offset: binemit::CodeOffset,
    /// The addend to add to the relocation value.
    pub addend: binemit::Addend,
}

pub enum RelocationTarget {
    /// The user function index
    UserFunc(FuncIndex),
    /// memory.grow
    GrowMemory,
    /// memory.current
    CurrentMemory,
}

pub type Relocations = PrimaryMap<DefinedFuncIndex, Vec<Relocation>>;

pub fn compile_module<'data, 'module>(translation: &ModuleTranslation<'data, 'module>, isa: &isa::TargetIsa) -> Result<(Compilation, Relocations), String> {
    let mut functions = PrimaryMap::new();
    let mut relocations = PrimaryMap::new();

    let mut context = Context::new();
    let mut trans = FuncTranslator::new();

    for (i, input) in translation.lazy.func_body_inputs.iter() {

    }
}

pub use self::mutex::{Mutex, MutexGuard, RawMutex};

mod mutex;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SandboxImplementation {
    UncommitedPages,
    BoundsChecks,
}

pub trait Backend: Send + Sync {
    type ThreadToken;
    type Info: SystemInfo;
    type RawMutex: RawMutex;

    fn create_thread(&self, func_ptr: *const (), arg: *const ()) -> Self::ThreadToken;
    fn destroy_thread(&self, token: Self::ThreadToken);

    fn allocate_executable_memory(&self, size: usize) -> *mut u8;
    fn deallocate_executable_memory(&self, ptr: *mut u8, size: usize);

    fn system_info(&self) -> &Self::Info;
}

pub trait SystemInfo {
    fn hardware_thread_num(&self) -> usize;

    fn sandbox_implementation(&self) -> SandboxImplementation;
}
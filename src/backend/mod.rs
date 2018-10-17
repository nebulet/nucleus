
pub use self::mutex::{Mutex, MutexGuard, RawMutex};

mod mutex;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SandboxImplementation {
    UncommitedPages,
    BoundsChecks,
}

bitflags! {
    pub struct MemPerms: u8 {
        const NONE  = 0x0;
        const READ  = 0x1;
        const WRITE = 0x2;
        const EXEC  = 0x4;
    }
}

pub trait Backend: Send + Sync {
    type ThreadToken;
    type Info: SystemInfo;
    type RawMutex: RawMutex;

    fn create_thread(&self, func_ptr: *const (), arg: *const ()) -> Self::ThreadToken;
    fn destroy_thread(&self, token: Self::ThreadToken);

    fn alloc(&self, size: usize, perms: MemPerms) -> *mut u8;
    fn dealloc(&self, ptr: *mut u8, size: usize);

    fn system_info(&self) -> &Self::Info;
}

pub trait SystemInfo {
    fn hardware_thread_num(&self) -> usize;

    fn sandbox_implementation(&self) -> SandboxImplementation;
}
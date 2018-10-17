
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

pub trait RawMutex {
    const INIT: Self;

    unsafe fn lock(&self);
    unsafe fn try_lock(&self) -> bool;
    unsafe fn unlock(&self);
}

pub struct MutexGuard<'a, R: 'a + RawMutex, T: 'a + ?Sized> {
    data: &'a mut T,
    raw: &'a R,
}

pub struct Mutex<R: RawMutex, T: ?Sized> {
    raw: R,
    inner: UnsafeCell<T>,
}

impl<R: RawMutex, T> Mutex<R, T> {
    pub fn new(inner: T) -> Self {
        Self {
            raw: R::INIT,
            inner: UnsafeCell::new(inner),
        }
    }

    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }
}

impl<R: RawMutex, T: ?Sized> Mutex<R, T> {
    pub fn lock(&self) -> MutexGuard<R, T> {
        unsafe { self.raw.lock(); }

        MutexGuard {
            data: unsafe { &mut *self.inner.get() },
            raw: &self.raw,
        }
    }

    pub fn try_lock(&self) -> Option<MutexGuard<R, T>> {
        let was_locked = unsafe { self.raw.try_lock() };

        if was_locked {
            Some(MutexGuard {
                data: unsafe { &mut *self.inner.get() },
                raw: &self.raw,
            })
        } else {
            None
        }
    }
}

impl<'a, R: 'a + RawMutex, T: 'a + ?Sized> Deref for MutexGuard<'a, R, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, R: 'a + RawMutex, T: 'a + ?Sized> DerefMut for MutexGuard<'a, R, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, R: 'a + RawMutex, T: 'a + ?Sized> Drop for MutexGuard<'a, R, T> {
    fn drop(&mut self) {
        unsafe { self.raw.unlock(); }
    }
}

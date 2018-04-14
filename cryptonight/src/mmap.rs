// copyright 2017 Kaz Wesley

use libc::{self, c_void, MAP_ANONYMOUS, MAP_HUGETLB, MAP_PRIVATE, PROT_READ, PROT_WRITE};
use std::mem::size_of;
use std::ops::{Deref, DerefMut};
use std::ptr::{self, Unique};

pub struct Mmap<T>(Unique<T>);

impl<T> Mmap<T> {
    pub fn new_huge() -> Option<Self> {
        unsafe {
            let pmap = libc::mmap(
                ptr::null_mut(),
                size_of::<T>(),
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS | MAP_HUGETLB,
                -1,
                0,
            ) as *mut T;
            if pmap as *mut libc::c_void == libc::MAP_FAILED {
                return None;
            }
            Some(Mmap(Unique::new(pmap)?))
        }
    }
}

impl<T> Default for Mmap<T> {
    fn default() -> Self {
        Mmap::new_huge().expect("hugepage mmap")
    }
}

impl<T> Drop for Mmap<T> {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.0.as_ptr() as *mut T as *mut c_void, size_of::<T>());
        }
    }
}

impl<T> Deref for Mmap<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { self.0.as_ref() }
    }
}

impl<T> DerefMut for Mmap<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.0.as_mut() }
    }
}

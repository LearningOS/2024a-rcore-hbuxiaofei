use lazy_static::*;
use crate::sync::UPSafeCell;
use crate::config::MAX_SYSCALL_NUM;

pub struct SyscallCount {
    inner: UPSafeCell<SyscallCountInner>,
}

pub struct SyscallCountInner {
    count: [u32; MAX_SYSCALL_NUM],
}

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref SYSCALL_COUNT: SyscallCount = {
        let count = [0; MAX_SYSCALL_NUM];
        SyscallCount {
            inner: unsafe {
                UPSafeCell::new(SyscallCountInner {
                    count,
                })
            },
        }
    };
}

impl SyscallCount {
    fn count_inc(&self, syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        inner.count[syscall_id] += 1;
    }

    fn count_get(&self, syscall_id: usize) -> u32{
        let inner = self.inner.exclusive_access();
        inner.count[syscall_id]
    }
}

pub fn count_inc(syscall_id: usize) {
    SYSCALL_COUNT.count_inc(syscall_id);
}

pub fn count_get(syscall_id: usize) -> u32 {
    SYSCALL_COUNT.count_get(syscall_id)
}

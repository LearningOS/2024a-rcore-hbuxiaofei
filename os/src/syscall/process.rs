//! Process management syscalls

use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    mm::{translated_byte_buffer, MapPermission, VirtAddr},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_current_status,
        get_current_syscall_times, get_current_time, map_check_ok, map_current,
        suspend_current_and_run_next, unmap_current, TaskStatus,
    },
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// Copy memory from kernel space to user space
fn copy_to_user(dst: *mut u8, src: *const u8, len: usize) {
    let s: &[u8] = unsafe { core::slice::from_raw_parts(src, len) };
    let buffers = translated_byte_buffer(current_user_token(), dst, len);
    let mut start = 0;
    for buffer in buffers {
        let end = start + buffer.len();
        buffer.copy_from_slice(&s[start..end]);
        start = end;
    }
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let tv = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let len = core::mem::size_of::<TimeVal>();
    let s: &[u8] = unsafe { core::slice::from_raw_parts(&tv as *const TimeVal as *const u8, len) };
    copy_to_user(ts as *mut u8, s.as_ptr(), len);
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let mut syscall_times = [0; MAX_SYSCALL_NUM];
    for i in 0..syscall_times.len() {
        syscall_times[i] = get_current_syscall_times(i);
    }
    let info = TaskInfo {
        status: get_current_status(),
        syscall_times,
        time: get_current_time(),
    };
    let len = core::mem::size_of::<TaskInfo>();
    let s: &[u8] =
        unsafe { core::slice::from_raw_parts(&info as *const TaskInfo as *const u8, len) };
    copy_to_user(ti as *mut u8, s.as_ptr(), len);
    0
}

/// Align up to PAGE_SIZE
fn align_up_pagesize(x: usize) -> usize {
    (x + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    let start_va = VirtAddr(start);
    if !start_va.aligned() {
        return -1;
    }
    if port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }

    let end_va = VirtAddr(start + align_up_pagesize(len));

    if !map_check_ok(start_va, end_va) {
        return -1;
    }

    let mut permission = MapPermission::U;
    if port & (0x01 << 0) != 0 {
        permission |= MapPermission::R;
    }
    if port & (0x01 << 1) != 0 {
        permission |= MapPermission::W;
    }
    if port & (0x01 << 2) != 0 {
        permission |= MapPermission::X;
    }

    map_current(start_va, end_va, permission);

    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let start_va = VirtAddr(start);
    if !start_va.aligned() {
        return -1;
    }
    let end_va = VirtAddr(start + align_up_pagesize(len));
    unmap_current(start_va, end_va);
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

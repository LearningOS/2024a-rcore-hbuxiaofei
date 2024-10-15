//! Process management syscalls
//!
use alloc::sync::Arc;

use crate::{
    config::MAX_SYSCALL_NUM,
    fs::{open_file, OpenFlags},
    config::PAGE_SIZE,
    loader::get_app_data_by_name,
    mm::{translated_byte_buffer, MapPermission, VirtAddr},
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
    task::{get_current_syscall_times, map_current, unmap_current, TaskControlBlock},
    timer::{get_time_ms, get_time_us},
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

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
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

/// Copy memory from user space to kernel space
#[allow(dead_code)]
fn copy_from_user(dst: *mut u8, src: *const u8, len: usize) {
    let s: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(dst, len) };
    let buffers = translated_byte_buffer(current_user_token(), src, len);
    let mut start = 0;
    for buffer in buffers {
        let end = start + buffer.len();
        s.copy_from_slice(&buffer[start..end]);
        start = end;
    }
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
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
    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let cur_time = get_time_ms();
    let mut syscall_times = [0; MAX_SYSCALL_NUM];
    for i in 0..syscall_times.len() {
        syscall_times[i] = get_current_syscall_times(i);
    }

    let task = current_task().unwrap();
    let task_inner = task.inner_exclusive_access();

    let info = TaskInfo {
        status: task_inner.task_status,
        syscall_times,
        time: if cur_time > task_inner.start_time {
            cur_time - task_inner.start_time
        } else {
            0
        },
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

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let start_va = VirtAddr(start);
    if !start_va.aligned() {
        return -1;
    }
    if port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }

    let end_va = VirtAddr(start + align_up_pagesize(len));

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

    map_current(start_va, end_va, permission)
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let start_va = VirtAddr(start);
    if !start_va.aligned() {
        return -1;
    }
    let end_va = VirtAddr(start + align_up_pagesize(len));
    unmap_current(start_va, end_va)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    let path = {
        let token = current_user_token();
        translated_str(token, path)
    };
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        let mut task_inner = task.inner_exclusive_access();
        let child = Arc::new(TaskControlBlock::new(data));
        let new_pid = child.pid.0;
        task_inner.children.push(child.clone());
        add_task(child);

        new_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

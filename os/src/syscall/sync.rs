use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec::Vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    let enable_dld = process_inner.enable_dld;
    drop(process_inner);
    drop(process);
    if enable_dld && !mutex.lock_try() {
        return -0xDEAD;
    }
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };

    if process_inner.sem_available.len() < id + 1 {
        process_inner.sem_available.resize(id + 1, 0);
    }
    process_inner.sem_available[id] = res_count as isize;
    if process_inner.sem_work.len() < id + 1 {
        process_inner.sem_work.resize(id + 1, 0);
    }
    process_inner.sem_work[id] = res_count as isize;

    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = {
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    };
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    let nr_need = sem.get_need(tid);

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.sem_allocation.len() < tid + 1 {
        process_inner.sem_allocation.resize(tid + 1, Vec::new());
    }
    if process_inner.sem_allocation[tid].len() < sem_id + 1 {
        process_inner.sem_allocation[tid].resize(sem_id + 1, 0);
    }
    if process_inner.sem_need.len() < tid + 1 {
        process_inner.sem_need.resize(tid + 1, Vec::new());
    }
    if process_inner.sem_need[tid].len() < sem_id + 1 {
        process_inner.sem_need[tid].resize(sem_id + 1, 0);
    }
    process_inner.sem_allocation[tid][sem_id] -= 1;
    process_inner.sem_work[sem_id] += 1;
    process_inner.sem_need[tid][sem_id] = nr_need;

    0
}

fn sem_try_down(cur_tid: usize, sem_id: usize) -> bool {
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    let finish = &process_inner.sem_finish;
    let need = &process_inner.sem_need;
    let work = &process_inner.sem_work;

    let tasks = &process_inner.tasks;
    for task in tasks.iter() {
        if let Some(t) = task {
            let tid = t.get_tid();
            if tid.is_none() {
                continue;
            }
            let tid = tid.unwrap();
            if finish.len() > tid && work.len() > sem_id {
                if need.len() > tid && need[tid].len() > sem_id {
                    if finish[tid] == false && need[tid][sem_id] < work[sem_id] {
                        return true;
                    }
                }
            }
        }
    }

    for e in finish.iter() {
        if !e {
            process_inner.sem_allocation[cur_tid][sem_id] -= 1;
            process_inner.sem_work[sem_id] += 1;
            return false;
        }
    }
    true
}

/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = {
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    };
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let enable_dld = process_inner.enable_dld;

    if process_inner.sem_allocation.len() < tid + 1 {
        process_inner.sem_allocation.resize(tid + 1, Vec::new());
    }
    if process_inner.sem_allocation[tid].len() < sem_id + 1 {
        process_inner.sem_allocation[tid].resize(sem_id + 1, 0);
    }
    if process_inner.sem_need.len() < tid + 1 {
        process_inner.sem_need.resize(tid + 1, Vec::new());
    }
    if process_inner.sem_need[tid].len() < sem_id + 1 {
        process_inner.sem_need[tid].resize(sem_id + 1, 0);
    }
    process_inner.sem_allocation[tid][sem_id] += 1;
    process_inner.sem_work[sem_id] -= 1;
    process_inner.sem_need[tid][sem_id] = sem.get_need(tid);
    if sem.get_count() <= 0 {
        process_inner.sem_need[tid][sem_id] += 1;
    }

    drop(process_inner);

    if enable_dld && !sem_try_down(tid, sem_id) {
        return -0xDEAD;
    }

    sem.down();

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.sem_need[tid][sem_id] = sem.get_need(tid);

    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");
    if enabled != 0 && enabled != 1 {
        return -1;
    }
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.enable_dld = if enabled == 0 { false } else { true };
    0
}
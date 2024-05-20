use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
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
        process_inner.mutex_available[id] = 1;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        let id = process_inner.mutex_list.len() - 1;
        process_inner.mutex_available[id] = 1;
        id as isize
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!("kernel:pid[{}] tid[{}] sys_mutex_lock", pid, tid);
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.mutex_available[mutex_id] <= 0 {
        let mut work = process_inner.mutex_available;
        let allocation = process_inner.mutex_allocation;
        let need = process_inner.mutex_need;
        let mut finish = [false; 20];
        let len = process_inner.tasks.len();
        loop {
            let mut find = false;
            for i in 0..len {
                if finish[i] || i == tid {
                    continue;
                }
                let mut flag = true;
                for j in 0..5 {
                    if need[i][j] != 0 && need[i][j] > work[j] {
                        flag = false;
                    }
                }
                if flag {
                    finish[i] = true;
                    find = true;
                    for j in 0..5 {
                        work[j] += allocation[i][j];
                    }
                }
            }
            if !find {
                break;
            }
        }
        if work[mutex_id] > 0 {
            process_inner.mutex_need[tid][mutex_id] += 1;
        } else {
            return -0xdead;
        }
    } else {
        process_inner.mutex_available[mutex_id] -= 1;
        process_inner.mutex_allocation[tid][mutex_id] += 1;
    }
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!("kernel:pid[{}] tid[{}] sys_mutex_unlock", pid, tid);
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.mutex_allocation[tid][mutex_id] += process_inner.mutex_need[tid][mutex_id];
    process_inner.mutex_need[tid][mutex_id] = 0;
    process_inner.mutex_available[mutex_id] += 1;
    process_inner.mutex_allocation[tid][mutex_id] -= 1;
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!("kernel:pid[{}] tid[{}] sys_semaphore_create", pid, tid);
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
        process_inner.semaphore_available[id] = res_count as isize;
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        let id = process_inner.semaphore_list.len() - 1;
        process_inner.semaphore_available[id] = res_count as isize;
        id
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!("kernel:pid[{}] tid[{}] sys_semaphore_up", pid, tid);
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_deadlock_detect {
        process_inner.semaphore_allocation[tid][sem_id] +=
            process_inner.semaphore_need[tid][sem_id];
        process_inner.semaphore_need[tid][sem_id] = 0;
        process_inner.semaphore_available[sem_id] += 1;
        process_inner.semaphore_allocation[tid][sem_id] -= 1;
    }
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let pid = current_task().unwrap().process.upgrade().unwrap().getpid();
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!("kernel:pid[{}] tid[{}] sys_semaphore_down", pid, tid);
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_deadlock_detect {
        if process_inner.semaphore_available[sem_id] <= 0 {
            let mut work = process_inner.semaphore_available;
            let allocation = process_inner.semaphore_allocation;
            let need = process_inner.semaphore_need;
            let mut finish = [false; 20];
            let len = process_inner.tasks.len();
            loop {
                let mut find = false;
                for i in 0..len {
                    if finish[i] || i == tid {
                        continue;
                    }
                    let mut flag = true;
                    for j in 0..5 {
                        if need[i][j] != 0 && need[i][j] > work[j] {
                            flag = false;
                        }
                    }
                    if flag {
                        finish[i] = true;
                        find = true;
                        for j in 0..5 {
                            work[j] += allocation[i][j];
                        }
                    }
                }
                if !find {
                    break;
                }
            }
            if work[sem_id] > 0 {
                process_inner.semaphore_need[tid][sem_id] += 1;
            } else {
                return -0xdead;
            }
        } else {
            process_inner.semaphore_available[sem_id] -= 1;
            process_inner.semaphore_allocation[tid][sem_id] += 1;
        }
    }

    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();
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
    trace!("kernel: sys_enable_deadlock_detect");
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.enable_deadlock_detect = enabled != 0;
    0
}

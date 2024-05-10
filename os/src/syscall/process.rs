//! Process management syscalls
use alloc::sync::Arc;
use core::mem;

use crate::{
    config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{
        translated_byte_buffer, translated_refmut, translated_str, MapError, MapPermission,
        MemorySet,
    },
    syscall::*,
    task::{
        add_task, current_task, exit_current_and_run_next,
        suspend_current_and_run_next, MmapProtection, TaskStatus,
    },
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

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_exit", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_EXIT] += 1;
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_yield", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_YIELD] += 1;
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel: sys_getpid pid:{}", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_GETPID] += 1;
    drop(inner);
    current_task.pid.0 as isize
}

pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_fork", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_FORK] += 1;
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
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_exec", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_EXEC] += 1;
    drop(inner);
    let token = current_task.get_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task;
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let current_task = current_task().unwrap();
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task.pid.0,
        pid
    );
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = current_task.inner_exclusive_access();
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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_get_time", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_GET_TIME] += 1;
    let buffers = translated_byte_buffer(
        current_task.get_user_token(),
        ts as *const u8,
        mem::size_of::<TimeVal>(),
    );
    match buffers.first() {
        Some(buffer) => {
            let ts = (*buffer).as_ptr() as *mut TimeVal;
            let ts = unsafe { &mut *ts };
            let us = get_time_us();
            ts.sec = us / 1_000_000;
            ts.usec = us % 1_000_000;
            0
        }
        None => -1,
    }
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let current_task = current_task().unwrap();
    trace!(
        "kernel:pid[{}] sys_task_info",
        current_task.pid.0
    );
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_TASK_INFO] += 1;
    let buffers = translated_byte_buffer(
        current_task.get_user_token(),
        ti as *const u8,
        mem::size_of::<TaskInfo>(),
    );
    match buffers.first() {
        Some(buffer) => {
            let ti = (*buffer).as_ptr() as *mut TaskInfo;
            let ti = unsafe { &mut *ti };
            let st = inner.syscall_times;
            let time = get_time_ms();
            ti.syscall_times.copy_from_slice(&st);
            ti.status = inner.task_status;
            ti.time = time - inner.start_time;
            0
        }
        None => -1,
    }
}

fn map_addr(
    memory_set: &mut MemorySet,
    addr: usize,
    size: usize,
    prot: usize,
) -> Result<(), MapError> {
    let prot = MmapProtection::from_bits(
        prot.try_into()
            .map_err(|_| MapError::InvalidPermissionBits(prot))?,
    )
    .ok_or(MapError::InvalidPermissionBits(prot))?;
    if prot.is_empty() {
        return Err(MapError::InvalidPermissionBits(0));
    }
    let mut perm = MapPermission::empty();
    if prot.contains(MmapProtection::R) {
        perm |= MapPermission::R;
    }
    if prot.contains(MmapProtection::W) {
        perm |= MapPermission::W;
    }
    if prot.contains(MmapProtection::X) {
        perm |= MapPermission::X;
    }
    perm |= MapPermission::U;
    memory_set.insert_framed_area(addr.into(), (addr + size).into(), perm)
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_mmap", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_MMAP] += 1;
    match map_addr(&mut inner.memory_set, start, len, prot) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

fn unmap_addr(memory_set: &mut MemorySet, addr: usize, size: usize) -> Result<(), MapError> {
    memory_set.remove_framed_area(addr.into(), (addr + size).into())
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_munmap", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_MUNMAP] += 1;
    match unmap_addr(&mut inner.memory_set, start, len) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_sbrk", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_SBRK] += 1;
    if let Some(old_brk) = current_task.change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

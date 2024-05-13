//! Process management syscalls
use core::mem;

use crate::{
    config::MAX_SYSCALL_NUM,
    mm::translated_byte_buffer,
    syscall::{SYSCALL_EXIT, SYSCALL_GET_TIME, SYSCALL_TASK_INFO, SYSCALL_YIELD},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_run_time,
        get_syscall_times, get_task_status, map_addr, record_syscall, suspend_current_and_run_next,
        unmap_addr, TaskStatus,
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
    record_syscall(SYSCALL_EXIT);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    record_syscall(SYSCALL_YIELD);
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    record_syscall(SYSCALL_GET_TIME);
    let buffers = translated_byte_buffer(
        current_user_token(),
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
    trace!("kernel: sys_task_info");
    record_syscall(SYSCALL_TASK_INFO);
    let buffers = translated_byte_buffer(
        current_user_token(),
        ti as *const u8,
        mem::size_of::<TaskInfo>(),
    );
    match buffers.first() {
        Some(buffer) => {
            let ti = (*buffer).as_ptr() as *mut TaskInfo;
            let ti = unsafe { &mut *ti };
            let st = get_syscall_times();
            ti.syscall_times.copy_from_slice(&st);
            ti.status = get_task_status();
            ti.time = get_run_time();
            0
        }
        None => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    match map_addr(start, len, port) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    match unmap_addr(start, len) {
        Ok(()) => 0,
        Err(_) => -1,
    }
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

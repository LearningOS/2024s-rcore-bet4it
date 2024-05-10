//! File and filesystem-related syscalls
use crate::fs::{open_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::syscall::{SYSCALL_CLOSE, SYSCALL_OPEN, SYSCALL_READ, SYSCALL_WRITE};
use crate::task::current_task;

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_write", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_WRITE] += 1;
    let token = inner.memory_set.token();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_read", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_READ] += 1;
    let token = inner.memory_set.token();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_open", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_OPEN] += 1;
    let token = inner.memory_set.token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_close", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_CLOSE] += 1;
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
    trace!(
        "kernel:pid[{}] sys_fstat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

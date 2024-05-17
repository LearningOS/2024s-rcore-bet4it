//! File and filesystem-related syscalls
use crate::fs::{link_file, open_file, unlink_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::syscall::{
    SYSCALL_CLOSE, SYSCALL_FSTAT, SYSCALL_LINKAT, SYSCALL_OPEN, SYSCALL_READ, SYSCALL_UNLINKAT,
    SYSCALL_WRITE,
};
use crate::task::current_task;
use core::mem;

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
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_fstat", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_FSTAT] += 1;
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    if let Some(inode) = inner.fd_table.get(fd) {
        if let Some(inode) = inode.as_ref() {
            let buffers = translated_byte_buffer(
                inner.memory_set.token(),
                st as *const u8,
                mem::size_of::<Stat>(),
            );
            match buffers.first() {
                Some(buffer) => {
                    let st = (*buffer).as_ptr() as *mut Stat;
                    let st = unsafe { &mut *st };
                    inode.stat(st);
                    0
                }
                None => -1,
            }
        } else {
            -1
        }
    } else {
        -1
    }
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_fstat", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_LINKAT] += 1;
    let token = inner.memory_set.token();
    let oldpath = translated_str(token, old_name);
    let newpath = translated_str(token, new_name);
    link_file(oldpath.as_str(), newpath.as_str())
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_unlinkat", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_UNLINKAT] += 1;
    let token = inner.memory_set.token();
    let path = translated_str(token, name);
    unlink_file(path.as_str())
}

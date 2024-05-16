//! File and filesystem-related syscalls
use crate::mm::translated_byte_buffer;
use crate::sbi::console_getchar;
use crate::syscall::{SYSCALL_READ, SYSCALL_WRITE};
use crate::task::{current_task, suspend_current_and_run_next};

const FD_STDIN: usize = 0;
const FD_STDOUT: usize = 1;

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_write", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_WRITE] += 1;
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(inner.memory_set.token(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let current_task = current_task().unwrap();
    trace!("kernel:pid[{}] sys_read", current_task.pid.0);
    let mut inner = current_task.inner_exclusive_access();
    inner.syscall_times[SYSCALL_READ] += 1;
    match fd {
        FD_STDIN => {
            assert_eq!(len, 1, "Only support len = 1 in sys_read!");
            let mut c: usize;
            loop {
                c = console_getchar();
                if c == 0 {
                    suspend_current_and_run_next();
                    continue;
                } else {
                    break;
                }
            }
            let ch = c as u8;
            let mut buffers = translated_byte_buffer(inner.memory_set.token(), buf, len);
            unsafe {
                buffers[0].as_mut_ptr().write_volatile(ch);
            }
            1
        }
        _ => {
            panic!("Unsupported fd in sys_read!");
        }
    }
}

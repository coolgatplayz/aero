/*
 * Copyright (C) 2021-2022 The Aero Project Developers.
 *
 * This file is part of The Aero Project.
 *
 * Aero is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Aero is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Aero. If not, see <https://www.gnu.org/licenses/>.
 */

//! System Calls are used to call a kernel service from userland.
//!
//! | %rax   | Name                    |
//! |--------|-------------------------|
//! | 0      | read                    |
//! | 1      | write                   |
//! | 2      | open                    |
//! | 3      | close                   |
//! | 4      | shutdown                |
//! | 5      | exit                    |
//! | 6      | fork                    |
//! | 7      | reboot                  |
//! | 8      | mmap                    |
//! | 9      | munmap                  |
//! | 10     | arch_prctl              |
//! | 11     | get_dents               |
//! | 12     | get_cwd                 |
//! | 13     | chdir                   |
//! | 14     | mkdir                   |
//! | 15     | mkdirat                 |
//! | 16     | rmdir                   |
//! | 17     | exec                    |
//! | 18     | log                     |
//! | 19     | uname                   |
//! | 20     | waitpid                 |
//! | 21     | ioctl                   |
//! | 22     | getpid                  |
//! | 23     | socket                  |
//! | 24     | connect                 |
//! | 25     | bind                    |
//! | 26     | listen                  |
//! | 27     | accept                  |
//! | 28     | seek                    |
//! | 29     | gettid                  |
//! | 30     | gettime                 |
//! | 31     | sleep                   |
//! | 32     | access                  |
//! | 33     | pipe                    |
//! | 34     | unlink                  |
//! | 35     | gethostname             |
//! | 36     | sethostname             |
//! | 37     | info                    |
//! | 38     | clone                   |
//! | 39     | sigreturn               |
//! | 40     | sigaction               |
//! | 41     | sigprocmask             |
//! | 42     | dup                     |
//! | 43     | fcntl                   |
//! | 44     | dup2                    |
//! | 45     | ipc_send                |
//! | 46     | ipc_recv                |
//! | 47     | ipc_discover_root       |
//! | 48     | ipc_become_root         |

use core::mem::MaybeUninit;

use aero_syscall::prelude::*;

pub mod fs;
pub mod ipc;
mod net;
pub mod process;
pub mod time;

use alloc::boxed::Box;
use alloc::vec::Vec;

pub use fs::*;
pub use ipc::*;
pub use process::*;
pub use time::*;

use crate::utils::StackHelper;

pub struct ExecArgs {
    inner: Vec<Box<[u8]>>,
}

impl ExecArgs {
    pub fn push_into_stack(&self, stack: &mut StackHelper) -> Vec<u64> {
        let mut tops = Vec::with_capacity(self.inner.len());

        for slice in self.inner.iter() {
            unsafe {
                stack.write(0u8);
                stack.write_bytes(slice);
            }

            tops.push(stack.top());
        }

        tops
    }
}

pub fn exec_args_from_slice(args: usize, size: usize) -> ExecArgs {
    // NOTE: Arguments must be moved into kernel space before we utilize them.
    //
    // struct SliceReference {
    //    ptr: *const usize,
    //    len: usize,
    // }
    let data = args as *const [usize; 2];
    let slice = unsafe { core::slice::from_raw_parts(data, size) };

    let mut result = Vec::new();

    for inner in slice {
        let mut boxed = Box::new_uninit_slice(inner[1]);
        let ptr = inner[0] as *const MaybeUninit<u8>;

        unsafe {
            boxed.as_mut_ptr().copy_from(ptr, inner[1]);

            let inner_slice = boxed.assume_init();
            result.push(inner_slice);
        }
    }

    ExecArgs { inner: result }
}

pub fn generic_do_syscall(
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
    f: usize,
    g: usize,
) -> usize {
    let result = match a {
        SYS_EXIT => process::exit(b),
        SYS_SHUTDOWN => process::shutdown(),
        SYS_FORK => process::fork(),
        SYS_MMAP => process::mmap(b, c, d, e, f, g),
        SYS_MUNMAP => process::munmap(b, c),
        SYS_EXEC => process::exec(b, c, d, e, f, g),
        SYS_LOG => process::log(b, c),
        SYS_UNAME => process::uname(b),
        SYS_WAITPID => process::waitpid(b, c, d),
        SYS_GETPID => process::getpid(),
        SYS_GETTID => process::gettid(),
        SYS_GETHOSTNAME => process::gethostname(b, c),
        SYS_SETHOSTNAME => process::sethostname(b, c),
        SYS_INFO => process::info(b),
        SYS_SIGACTION => process::sigaction(b, c, d, e),
        SYS_CLONE => process::clone(b, c),

        SYS_READ => fs::read(b, c, d),
        SYS_OPEN => fs::open(b, c, d, e),
        SYS_CLOSE => fs::close(b),
        SYS_WRITE => fs::write(b, c, d),
        SYS_GETDENTS => fs::getdents(b, c, d),
        SYS_GETCWD => fs::getcwd(b, c),
        SYS_CHDIR => fs::chdir(b, c),
        SYS_MKDIR => fs::mkdir(b, c),
        SYS_MKDIR_AT => fs::mkdirat(b, c, d),
        SYS_RMDIR => fs::rmdir(b, c),
        SYS_IOCTL => fs::ioctl(b, c, d),
        SYS_SEEK => fs::seek(b, c, d),
        SYS_ACCESS => fs::access(b, c, d, e, f),
        SYS_PIPE => fs::pipe(b, c),
        SYS_UNLINK => fs::unlink(b, c, d, e),
        SYS_DUP => fs::dup(b, c),
        SYS_DUP2 => fs::dup2(b, c, d),
        SYS_FCNTL => fs::fcntl(b, c, d),
        SYS_STAT => fs::stat(b, c, d),

        SYS_SOCKET => net::socket(b, c, d),
        SYS_BIND => net::bind(b, c, d),

        SYS_GETTIME => time::gettime(b, c),
        SYS_SLEEP => time::sleep(b),

        SYS_IPC_SEND => ipc::send(b, c, d),
        SYS_IPC_RECV => ipc::recv(b, c, d, e),
        SYS_IPC_DISCOVER_ROOT => ipc::discover_root(),
        SYS_IPC_BECOME_ROOT => ipc::become_root(),

        _ => {
            log::error!("invalid syscall: {:#x}", a);
            Err(AeroSyscallError::ENOSYS)
        }
    };

    let result_usize = aero_syscall::syscall_result_as_usize(result);

    #[cfg(feature = "syslog")]
    {
        use crate::drivers::uart_16550;
        use alloc::string::String;

        let name = aero_syscall::syscall_as_str(a);
        let mut result_v = String::new();

        if result.is_ok() {
            result_v.push_str("\x1b[1;32m");
        } else {
            result_v.push_str("\x1b[1;31m");
        }

        result_v.push_str(name);
        result_v.push_str("\x1b[0m");

        result_v.push_str("(");

        for (i, arg) in [b, c, d, e, f, g].iter().enumerate() {
            if i != 0 {
                result_v.push_str(", ");
            }

            let hex_arg = alloc::format!("{:#x}", *arg);
            result_v.push_str(&hex_arg);
        }

        result_v.push_str(") = ");

        let result_str = alloc::format!("{:?}", result);
        result_v.push_str(&result_str);

        uart_16550::serial_println!("{}", result_v);
    }

    result_usize
}

//! Stop the real driver at rename entry/exit to inspect lock custody. No syscall
//! is replaced. Linux ABI: <linux/ptrace.h> and ptrace(2),
//! https://man7.org/linux/man-pages/man2/ptrace.2.html.

use std::{
    ffi::{c_int, c_long, c_void},
    io::Write,
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

unsafe extern "C" {
    fn ptrace(request: c_int, pid: c_int, addr: *mut c_void, data: *mut c_void) -> c_long;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
}

#[repr(C)]
#[derive(Default)]
struct SyscallInfo {
    op: u8,
    reserved: u8,
    flags: u16,
    arch: u32,
    instruction_pointer: u64,
    stack_pointer: u64,
    // Enough aligned storage for the largest ptrace_syscall_info union member.
    data: [u64; 8],
}

struct Tracee(Option<Child>);

impl Drop for Tracee {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn request(operation: c_int, pid: c_int, address: usize, data: usize) -> c_long {
    // SAFETY: calls below supply either integer-valued options or a live,
    // correctly sized SyscallInfo buffer. The PID is our stopped child only.
    let result = unsafe { ptrace(operation, pid, address as *mut c_void, data as *mut c_void) };
    assert!(
        result >= 0,
        "ptrace {operation:#x}: {}",
        std::io::Error::last_os_error()
    );
    result
}

fn wait(pid: c_int, deadline: Instant) -> c_int {
    loop {
        let mut status = 0;
        // SAFETY: status is writable; WNOHANG bounds every poll of our child.
        let result = unsafe { waitpid(pid, &mut status, 1) };
        if result == pid {
            return status;
        }
        assert!(result >= 0, "waitpid: {}", std::io::Error::last_os_error());
        assert!(
            Instant::now() < deadline,
            "driver tracing exceeded 20 seconds"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

pub fn driver_choice(
    experiment: &str,
    path: &Path,
    choice: &str,
    mut at_install: impl FnMut(usize, bool),
) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_arkham-driver"));
    command
        .args(["--experiment", experiment, "--load"])
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    // SAFETY: the child performs only an async-signal-safe ptrace call before
    // exec, without accessing parent state or taking any allocation locks.
    unsafe {
        command.pre_exec(|| {
            if ptrace(0, 0, std::ptr::null_mut(), std::ptr::null_mut()) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = Tracee(Some(
        command.spawn().expect("trace the test's own driver child"),
    ));
    let pid = child.0.as_ref().unwrap().id() as c_int;
    let deadline = Instant::now() + Duration::from_secs(20);
    writeln!(
        child.0.as_mut().unwrap().stdin.take().unwrap(),
        "{}",
        serde_json::json!({"op":"choose", "revision":0, "id":choice})
    )
    .unwrap();
    assert_eq!(
        wait(pid, deadline),
        0x57f,
        "expected the initial exec SIGTRAP"
    );
    // PTRACE_O_TRACESYSGOOD | PTRACE_O_EXITKILL.
    request(0x4200, pid, 0, 1 | (1 << 20));
    let mut installs = 0;
    let mut installing = false;
    loop {
        request(24, pid, 0, 0); // PTRACE_SYSCALL
        let status = wait(pid, deadline);
        if status & 0x7f == 0 {
            child.0.take(); // waitpid already reaped this exact child.
            assert_eq!(status >> 8, 0, "driver exit status");
            break;
        }
        assert_eq!(status, 0x857f, "expected a syscall stop, got {status:#x}");
        let mut info = SyscallInfo::default();
        request(
            0x420e,
            pid,
            size_of::<SyscallInfo>(),
            &mut info as *mut _ as usize,
        );
        if info.op == 1 {
            // Linux x86_64 rename, renameat, renameat2; aarch64 has no rename.
            installing = match info.arch {
                0xc000003e => matches!(info.data[0], 82 | 264 | 316),
                0xc00000b7 => matches!(info.data[0], 38 | 276),
                arch => panic!("unsupported audit architecture {arch:#x}"),
            };
            if installing {
                installs += 1;
                at_install(installs, false);
            }
        } else if info.op == 2 && installing {
            assert_eq!(info.data[0], 0, "rename must succeed");
            at_install(installs, true);
            installing = false;
        }
    }
    assert_eq!(
        installs, 2,
        "the driver must install recovery and then active"
    );
}

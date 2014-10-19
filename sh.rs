#![feature(phase, slicing_syntax, struct_variant)]
#[phase(plugin, link)] extern crate log;
extern crate libc;

use libc::funcs::posix88::fcntl;    // open
use libc::funcs::posix88::unistd;   // close, execv
use libc::types::os::arch::c95::c_int;
use libc::types::os::arch::posix88::{mode_t, pid_t};

// C macros
const O_RDWR : c_int = 2;

enum Cmd<'b> {

    ExecCmd {
        argv:   Vec<&'b str>,
        eargv:  Vec<&'b str>
    },

    RedirCmd {
        cmd:    Box<Cmd<'b>>,
        file:   Path,
        efile:  Path,
        mode:   mode_t,
        fd:     c_int
    },

    PipeCmd {
        left:   Box<Cmd<'b>>,
        right:  Box<Cmd<'b>>
    },

    ListCmd {
        left:   Box<Cmd<'b>>,
        right:  Box<Cmd<'b>>
    },

    BackCmd {
        cmd:    Box<Cmd<'b>>
    }

}

fn run_cmd<'b>(cmd: Cmd<'b>) -> c_int {
    match cmd {
        ExecCmd {argv, eargv} =>
            run_exec(argv, eargv),
        RedirCmd {cmd, file, efile, mode, fd} =>
            run_redir(cmd, file, efile, mode, fd),
        PipeCmd {left, right} =>
            run_pipe(left, right),
        ListCmd {left, right} =>
            run_list(left, right),
        BackCmd {cmd} =>
            run_back(cmd)
    }
}

fn run_exec(argv: Vec<&str>, eargv: Vec<&str>) -> c_int {
    fail!("run_exec")
}

fn run_redir(cmd: Box<Cmd>, file: Path, efile: Path,
             mode: mode_t, fd: c_int) -> c_int {
    fail!("run_redir")
}

fn run_pipe(left: Box<Cmd>, right: Box<Cmd>) -> c_int {
    fail!("run_pipe")
}

fn run_list(left: Box<Cmd>, right: Box<Cmd>) -> c_int {
    fail!("run_list")
}

fn run_back(cmd: Box<Cmd>) -> c_int {
    fail!("run_back")
}

fn main() {
    // Assumes three file descriptors open.
    loop {
        let fd = open(Path::new("console"), O_RDWR, 0);
        if fd < 0
            { break }
        if fd >= 3 {
            close(fd);
            break
        }
    }

    // Read and run input commands.
    let mut stdin = std::io::stdin();
    loop {
        match stdin.read_line() {
            Err (e) => fail!(e),
            Ok (line) => process_line(line)
        }
    }
}

fn process_line(line: String) {
    let cmd_args : Vec<&str> =
        line.as_slice().split(|c: char| c.is_whitespace()).collect();
    if cmd_args.len() == 0
        { return }
    if process_builtin(cmd_args[0], cmd_args[1..])
        { return }
}

fn process_builtin(cmd: &str, args: &[&str]) -> bool {
    match cmd {
        "cd" => {
            let dir_str = args[1..];
            if dir_str.len() > 1
                { debug!("cd: ignoring {}", dir_str[1..]); }
            // TODO: case with `cd` and no args at all is not handled!
            let dir = Path::new(dir_str[0]);
            debug!("cd {}", dir_str);
            if chdir(dir) < 0
                { stderr(format!("cannot cd {}\n", dir_str)) };
            true
        }
        _ => false
    }
}

// TODO: make a nice println! like macro for stderr
fn stderr(msg: String) {
    let mut stderr = std::io::stderr();
    match stderr.write_str(msg.as_slice()) {
        Ok (_) => (),
        Err (e) => fail!("cannot write to stderr: {}", e)
    }
}

//
// Syscalls
//

fn open(path: Path, oflag: c_int, mode: mode_t) -> c_int {
    unsafe {
        path.with_c_str(|c_path| fcntl::open(c_path, oflag, mode))
    }
}

fn close(fd: c_int) -> c_int {
    unsafe {
        unistd::close(fd)
    }
}

fn chdir(dir: Path) -> c_int {
    unsafe {
        dir.with_c_str(|c_dir| unistd::chdir(c_dir))
    }
}

fn fork() -> pid_t {
    unsafe {
        unistd::fork()
    }
}

fn fork_or_fail() -> pid_t {
    let pid = fork();
    if pid < 0
        { fail!("cannot fork") }
    pid
}

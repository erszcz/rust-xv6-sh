#![feature(phase, slicing_syntax, struct_variant)]
#[phase(plugin, link)] extern crate log;
extern crate libc;

use libc::consts::os::posix88::O_RDWR;
use libc::funcs::posix88::fcntl;
use libc::funcs::posix88::unistd;
use libc::types::os::arch::c95::c_int;
use libc::types::os::arch::posix88::{mode_t, pid_t};
use std::fmt::{mod, Show};
use std::io;

#[deriving(PartialEq, Show)]
enum Cmd<'b> {

    ExecCmd {
        argv:   Vec<&'b str>
    },

    RedirCmd {
        cmd:    Box<Cmd<'b>>,
        file:   PrintablePath,
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

struct PrintablePath { path: Path }

impl PartialEq for PrintablePath {
    fn eq(&self, other: &PrintablePath) -> bool {
        self.path == other.path
    }
}

impl Show for PrintablePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let PrintablePath { ref path } = *self;
        write!(f, "Path[{}]", path.as_str().unwrap_or(""))
    }
}

fn run_cmd<'b>(cmd: Cmd<'b>) -> c_int {
    debug!("{}", cmd);
    match cmd {
        ExecCmd {argv} =>
            run_exec(argv),
        RedirCmd {cmd, file, mode, fd} =>
            run_redir(cmd, file.path, mode, fd),
        PipeCmd {left, right} =>
            run_pipe(left, right),
        ListCmd {left, right} =>
            run_list(left, right),
        BackCmd {cmd} =>
            run_back(cmd)
    }
}

fn run_exec<'b>(argv: Vec<&'b str>) -> c_int {
    debug!("run_exec: argv={}", argv);
    fail!("run_exec");
}

fn run_redir(cmd: Box<Cmd>, file: Path, mode: mode_t, fd: c_int) -> c_int {
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
    loop {
        match get_line() {
            Err (e) => {
                error!("cannot get_line: {}", e);
                break
            },
            Ok (line) => process_line(line)
        }
    }
}

fn get_line() -> io::IoResult<String> {
    let mut stdout = io::stdout();
    match stdout.write_str("rsh $ ") {
        Err (e) => fail!("cannot write to stdout: {}", e),
        Ok (()) => stdout.flush()
    };
    let mut stdin = io::stdin();
    stdin.read_line()
}

fn process_line(line: String) {
    let cmd_args : Vec<&str> =
        line.as_slice().split(|c: char| c.is_whitespace()).collect();
    if cmd_args.len() == 0
        { return }
    if process_builtin(cmd_args[0], cmd_args[1..])
        { return }
    if fork_or_fail() == 0 {
        run_cmd(parse_cmd(&line));
    }
    let reaped = wait();
    if reaped == -1 {
        fail!("cannot wait");
    } else {
        debug!("reaped {}", reaped);
    }
}

fn process_builtin(cmd: &str, args: &[&str]) -> bool {
    match cmd {
        "cd" => {
            let dir_str = args[1..];
            if dir_str.len() > 1
                { debug!("cd: ignoring {}", dir_str[1..]); }
            // TODO: case with `cd` and no args at all is not handled!
            let dir = Path::new(dir_str[0]);
            debug!("cd {}", dir_str[0]);
            if chdir(dir) < 0
                { stderr(format!("cannot cd {}\n", dir_str)) };
            true
        }
        _ => false
    }
}

// TODO: make a nice `println!` like macro for stderr
fn stderr(msg: String) {
    let mut stderr = std::io::stderr();
    match stderr.write_str(msg.as_slice()) {
        Ok (_) => (),
        Err (e) => fail!("cannot write to stderr: {}", e)
    }
}

fn parse_cmd<'b>(line: &'b String) -> Cmd<'b> {
    let cmdline : &mut &str = &mut line.as_slice();
    let cmd = parse_line(cmdline);
    peek(cmdline, "");
    if *cmdline != ""
        { fail!("leftovers: {}", *cmdline); }
    cmd
}

fn parse_line<'b>(ps: &mut &'b str) -> Cmd<'b> {
    let cmd = ExecCmd { argv: vec!(*ps) };
    (*ps) = (*ps).slice_from((*ps).len());
    cmd
}

fn parse_pipe<'b>(ps: &mut &'b str) -> Cmd<'b> {
    let mut cmd = parse_exec();
}

#[test]
fn parse_exec_cmd_test() {
    let cmd = ExecCmd { argv: vec!("some_cmd") };
    let cmdline = "some_cmd".to_string();
    assert!(cmd == parse_cmd(&cmdline));
}

#[test]
fn parse_pipe_cmd_test() {
    let cmd =
        PipeCmd { left : box ExecCmd { argv: vec!("some_cmd") },
                  right: box PipeCmd { left : box ExecCmd { argv: vec!("other_cmd") },
                                       right: box ExecCmd { argv: vec!("another_cmd") }}};
    let cmdline = "some_cmd | other_cmd | another_cmd".to_string();
    assert!(cmd == parse_cmd(&cmdline));
}

fn peek(ps: &mut &str, toks: &str) -> bool {
    if *ps == ""
        { return false }
    let i = (*ps).chars()
        .enumerate().position(|(_,c)| !c.is_whitespace()).unwrap_or((*ps).len());
    *ps = (*ps).slice_from(i);
    let c = (*ps).char_at(0);
    toks.chars().position(|cc| c == cc).is_some()
}

#[deriving(Show, PartialEq)]
enum TokenKind {
    LRedir,
    RRedir,
    Append,
    Regular
}

#[deriving(Show)]
struct Token<'b> {
    kind: TokenKind,
    buf: &'b str
}

fn get_token<'b>(ps: &mut &'b str) -> Option<Token<'b>> {
    let mut s = *ps;
    while s.len() > 0 && s.char_at(0).is_whitespace()
        { s = next_char(ps); }
    if s.len() == 0
        { return None }
    let c = s.char_at(0);
    let res = match c {
        '|' | '(' | ')' | ';' | '&' | '<' => {
            let t = s;
            s = next_char(ps);
            Token { kind: if c == '<' { LRedir } else { Regular },
                    buf: t.slice_to(1) }
        },
        '>' => {
            let t = s;
            s = next_char(ps);
            if s.len() > 0 && s.char_at(0) == '>' {
                s = next_char(ps);
                Token { kind: Append, buf: t.slice_to(2) }
            } else {
                Token { kind: RRedir, buf: t.slice_to(1) }
            }
        },
        _ => {
            let maybe_to = s.chars() .position(|c| c.is_whitespace() || is_symbol(c));
            let end = match maybe_to {
                None => s.len(),
                Some (idx) => idx
            };
            let t = s;
            s = s.slice_from(end);
            (*ps) = s;
            Token { kind: Regular, buf: t.slice_to(end) }
        }
    };
    while s.len() > 0 && s.char_at(0).is_whitespace()
        { s = next_char(ps); }
    Some (res)
}

#[test]
fn get_token_simple_command_test() {
    let mut s = "/bin/echo a";
    let p = &mut s;
    {
        let tok = get_token(p).unwrap();
        println!("kind    : {}", tok.kind);
        println!("parsed  : {}", *p);
        println!("token   : {}", tok.buf);
        assert!(tok.kind == Regular);
        assert!(*p == "a");
        assert!(tok.buf == "/bin/echo");
    }
    {
        let tok = get_token(p).unwrap();
        println!("kind    : {}", tok.kind);
        println!("p len   : {}", (*p).len());
        println!("parsed  : {}", if (*p).len() == 0 { "(empty)" } else { *p });
        println!("token   : {}", tok.buf);
        assert!(tok.kind == Regular);
        assert!(*p == "");
        assert!(tok.buf == "a");
    }
}

#[test]
fn get_token_lredir_test() {
    let mut s = "/bin/echo < a";
    let p = &mut s;
    get_token(p);
    {
        let tok = get_token(p).unwrap();
        println!("kind    : {}", tok.kind);
        println!("parsed  : {}", *p);
        println!("token   : {}", tok.buf);
        assert!(tok.kind == LRedir);
        assert!(*p == "a");
        assert!(tok.buf == "<");
    }
}

#[test]
fn get_token_rredir_test() {
    let mut s = "/bin/echo > a";
    let p = &mut s;
    get_token(p);
    {
        let tok = get_token(p).unwrap();
        println!("kind    : {}", tok.kind);
        println!("parsed  : {}", *p);
        println!("token   : {}", tok.buf);
        assert!(tok.kind == RRedir);
        assert!(*p == "a");
        assert!(tok.buf == ">");
    }
}

#[test]
fn get_token_append_test() {
    let mut s = "/bin/echo >> a";
    let p = &mut s;
    get_token(p);
    {
        let tok = get_token(p).unwrap();
        println!("kind    : {}", tok.kind);
        println!("parsed  : {}", *p);
        println!("token   : {}", tok.buf);
        println!("tok len : {}", tok.buf.len());
        assert!(tok.kind == Append);
        assert!(*p == "a");
        assert!(tok.buf == ">>");
        assert!(tok.buf.len() == 2);
    }
}

fn next_char<'b>(ps: &mut &'b str) -> &'b str {
    *ps = (*ps).slice_from(1);
    *ps
}

#[test]
fn next_char_test() {
    let mut s = "abc";
    {
        let p = &mut s;
        let t = next_char(p);
        assert!(*p == "bc");
        assert!(t == "bc");
    }
    assert!(s == "bc");
}

fn is_symbol(c: char) -> bool {
    match "<|>&;()".chars().position(|d| d == c) {
        None => false,
        Some (_) => true
    }
}

#[test]
fn peek_test() {
    let mut s = "   (ala ma kota";
    let p = &mut s;
    debug!("{}, {}", peek(p, "("), *p);
    assert!(peek(p, "("));
    assert!(*p == "(ala ma kota");
    debug!("{}, {}", peek(p, "<("), *p);
    assert!(peek(p, "<("));
    assert!(*p == "(ala ma kota");
    debug!("{}, {}", peek(p, "<"), *p);
    assert!(!peek(p, "<"));
    assert!(*p == "(ala ma kota");
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

// For some unknown reason these particular syscalls are not made available
// in the Rust standard library.
mod syscalls {
    use libc::types::os::arch::c95::c_int;
    use libc::types::os::arch::posix88::pid_t;
    extern {
        pub fn wait(status: *mut c_int) -> pid_t;
    }
}

fn wait() -> pid_t {
    unsafe {
        syscalls::wait(0 as *mut c_int)
    }
}

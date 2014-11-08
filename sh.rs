#![feature(phase, slicing_syntax, struct_variant)]
#[phase(plugin, link)] extern crate log;
extern crate libc;

use libc::consts::os::posix88::{O_RDONLY, O_WRONLY, O_RDWR, O_CREAT,
                                S_IRUSR, S_IWUSR};
use libc::funcs::c95::stdlib;
use libc::funcs::posix88::fcntl;
use libc::funcs::posix88::unistd;
use libc::types::os::arch::c95::c_int;
use libc::types::os::arch::posix88::{mode_t, pid_t};
use std::c_str::CString;
use std::fmt::{mod, Show};
use std::io;
use std::io::fs::PathExtensions;

const S_IRGRP: mode_t = 0o40;
const S_IROTH: mode_t = 0o04;

#[deriving(Clone, PartialEq, Show)]
enum Cmd<'b> {

    ExecCmd {
        argv:       Vec<&'b str>
    },

    RedirCmd {
        cmd:        Box<Cmd<'b>>,
        file:       PrintablePath,
        oflags:     c_int,
        fd:         c_int
    },

    PipeCmd {
        left:       Box<Cmd<'b>>,
        right:      Box<Cmd<'b>>
    },

    ListCmd {
        left:       Box<Cmd<'b>>,
        right:      Box<Cmd<'b>>
    },

    BackCmd {
        cmd:        Box<Cmd<'b>>
    }

}

#[deriving(Clone)]
struct PrintablePath { path: Path }

impl PartialEq for PrintablePath {
    fn eq(&self, other: &PrintablePath) -> bool {
        self.path == other.path
    }
}

impl Show for PrintablePath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let PrintablePath { ref path } = *self;
        write!(f, "Path[{}]", path.display())
    }
}

fn run_cmd<'b>(cmd: Cmd<'b>) {
    debug!("{}", cmd);
    match cmd {
        ExecCmd {argv} =>
            run_exec(argv),
        RedirCmd {cmd, file, oflags, fd} =>
            run_redir(cmd, file.path, oflags, fd),
        PipeCmd {left, right} =>
            run_pipe(left, right),
        ListCmd {left, right} =>
            run_list(left, right),
        BackCmd {cmd} =>
            run_back(cmd)
    }
    exit(0);
}

fn run_exec<'b>(argv: Vec<&'b str>) {
    if argv.len() == 0
        { exit(0) }
    match resolve_path(argv[0]) {
        None => stderr(format!("rsh: {}: command not found\n", argv[0])),
        Some (path) => {
            execv(path.clone(), argv);
            stderr(format!("execv {} failed\n", path.display()));
        }
    }
}

fn resolve_path(cmd: &str) -> Option<Path> {
    if cmd.slice(0,2) == "./" || cmd.slice(0,1) == "/" {
        let path = Path::new(cmd);
        return if !path.is_file() { None }
               else { Some (path) }
    }
    match std::os::getenv("PATH") {
        None => None,
        Some (paths) => {
            let pathv : Vec<&str> = paths.split::<>(|c: char| c == ':').collect();
            find_in_paths(cmd, pathv)
        }
    }
}

fn find_in_paths(cmd: &str, paths: Vec<&str>) -> Option<Path> {
    for p in paths.iter() {
        let path = Path::new(p).join(cmd);
        if path.is_file()
            { return Some (path) }
    }
    None
}

fn run_redir(cmd: Box<Cmd>, file: Path, oflags: c_int, fd: c_int) {
    close(fd);
    let mode = S_IRUSR | S_IWUSR | S_IRGRP | S_IROTH;
    if open(file.clone(), oflags, mode) < 0 {
        stderr(format!("open {} failed\n", file.display()));
        exit(0);
    }
    run_cmd(*cmd);
}

fn run_pipe(left: Box<Cmd>, right: Box<Cmd>) {
    let p = match pipe() {
        None => panic!("pipe"),
        Some (fds) => fds
    };
    if fork_or_fail() == 0 {
        close(1);
        dup(p.val1());
        close(p.val0());
        close(p.val1());
        run_cmd(*left);
    }
    if fork_or_fail() == 0 {
        close(0);
        dup(p.val0());
        close(p.val0());
        close(p.val1());
        run_cmd(*right);
    }
    close(p.val0());
    close(p.val1());
    wait();
    wait();
}

fn run_list(left: Box<Cmd>, right: Box<Cmd>) {
    if fork_or_fail() == 0
        { run_cmd(*left); }
    wait();
    run_cmd(*right);
}

fn run_back(cmd: Box<Cmd>) {
    if fork_or_fail() == 0
        { run_cmd(*cmd); }
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
            Err (ref e) if e.kind == io::EndOfFile => break,
            Err (e) => {
                error!("cannot get_line: {}", e);
                break
            },
            Ok (line) => process_line(line.trim())
        }
    }
}

fn get_line() -> io::IoResult<String> {
    let mut stdout = io::stdout();
    match stdout.write_str("rsh $ ") {
        Err (e) => panic!("cannot write to stdout: {}", e),
        Ok (()) => if stdout.flush().is_err() { panic!("flush") }
    };
    let mut stdin = io::stdin();
    stdin.read_line()
}

fn process_line(line: &str) {
    if line.len() == 0
        { return }
    let cmd_args : Vec<&str> = split_line(line);
    if process_builtin(cmd_args[0], cmd_args[1..])
        { return }
    if fork_or_fail() == 0 {
        let cmd = parse_cmd(line);
        stderr(format!("{}\n", cmd));
        run_cmd(cmd);
    }
    wait();
}

fn split_line<'b>(line: &'b str) -> Vec<&'b str> {
    line.split(|c: char| c.is_whitespace()).collect()
}

fn process_builtin(cmd: &str, args: &[&str]) -> bool {
    match cmd {
        "cd" => builtin_cd(args),
        _ => false
    }
}

fn builtin_cd(args: &[&str]) -> bool {
    let dir = match (args.len(), std::os::getenv("HOME")) {
        (0, None) => {
            stderr(format!("rsh: cannot cd\n"));
            return true
        },
        (0, Some (home)) => Path::new(home.as_slice()),
        (_, _) => Path::new(args[0])
    };
    if chdir(dir.clone()) < 0
        { stderr(format!("cannot cd {}\n", dir.display())) };
    true
}

// TODO: make a nice `println!` like macro for stderr
fn stderr(msg: String) {
    let mut stderr = io::stderr();
    match stderr.write_str(msg.as_slice()) {
        Ok (_) => (),
        Err (e) => panic!("cannot write to stderr: {}", e)
    }
}

fn parse_cmd<'b>(line: &'b str) -> Cmd<'b> {
    let cmdline : &mut &str = &mut line.clone();
    let cmd = parse_line(cmdline);
    peek(cmdline, "");
    if *cmdline != ""
        { panic!("leftovers: {}", *cmdline); }
    cmd
}

fn parse_line<'b>(ps: &mut &'b str) -> Cmd<'b> {
    let mut cmd = parse_pipe(ps);
    while peek(ps, "&") {
        get_token(ps);
        cmd = BackCmd { cmd: box cmd };
    }
    if peek(ps, ";") {
        get_token(ps);
        cmd = ListCmd { left: box cmd, right: box parse_line(ps) };
    }
    cmd
}

fn parse_pipe<'b>(ps: &mut &'b str) -> Cmd<'b> {
    let execcmd = parse_exec(ps);
    if peek(ps, "|") {
        get_token(ps);
        PipeCmd { left: box execcmd, right: box parse_pipe(ps) }
    } else {
        execcmd
    }
}

fn parse_redirs<'b>(cmd: Cmd<'b>, ps: &mut &'b str) -> Cmd<'b> {
    while peek(ps, "<>") {
        // peek() returned true, unwrap can't fail
        let tok1 = get_token(ps).unwrap();
        let tok2 = match get_token(ps) {
            None => panic!("missing file for redirection"),
            Some (tok) =>
                if tok.kind != Regular { panic!("expected regular token") }
                else { tok }
        };
        let (oflags, fd) = match tok1.kind {
            Regular => panic!("expected special symbol"),
            LRedir => (O_RDONLY, 0 as i32),
            RRedir => (O_WRONLY | O_CREAT, 1 as i32),
            Append => (O_WRONLY | O_CREAT, 1 as i32)
        };
        return RedirCmd { cmd: box cmd,
                          file: PrintablePath { path: Path::new(tok2.buf) },
                          oflags: oflags,
                          fd: fd }
    }
    cmd
}

#[test]
fn parse_redir_test() {
    let execcmd = ExecCmd { argv: vec!("some_cmd") };
    let mut s = " > some_file";
    let p = &mut s;
    let redircmd = parse_redirs(execcmd.clone(), p);
    assert!(redircmd == RedirCmd { cmd: box execcmd,
                                   file: PrintablePath { path: Path::new("some_file") },
                                   oflags: O_WRONLY | O_CREAT,
                                   fd: 1 as i32 });
}

fn parse_block<'b>(ps: &mut &'b str) -> Cmd<'b> {
    if !peek(ps, "(")
        { panic!("parse_block") }
    get_token(ps);
    let inner_cmd = parse_line(ps);
    if !peek(ps, ")")
        { panic!("syntax - missing )") }
    get_token(ps);
    parse_redirs(inner_cmd, ps)
}

fn parse_exec<'b>(ps: &mut &'b str) -> Cmd<'b> {
    if peek(ps, "(")
        { return parse_block(ps) }
    let mut argv = vec!();
    let mut ret = parse_redirs(ExecCmd { argv: argv.clone() }, ps);
    while !peek(ps, "|)&;") {
        match get_token(ps) {
            None => break,
            Some (token) => {
                if token.kind != Regular
                    { panic!("syntax - expected regular token") }
                argv.push(token.buf);
                ret = parse_redirs(ExecCmd { argv: argv.clone() }, ps);
            }
        }
    }
    ret
}

#[test]
fn parse_exec_simple_test() {
    let cmd = ExecCmd { argv: vec!("some_cmd") };
    let cmdline = "some_cmd".to_string();
    assert!(cmd == parse_cmd(&cmdline));
}

#[test]
fn parse_exec_block_test() {
    let mut s = "(some_cmd | other_cmd)";
    let ps = &mut s;
    let cmd = PipeCmd { left : box ExecCmd { argv: vec!("some_cmd") },
                        right: box ExecCmd { argv: vec!("other_cmd") } };
    let parsed = parse_exec(ps);
    println!("{}", parsed);
    assert!(cmd == parsed);
}

#[test]
fn parse_pipe_cmd_test() {
    let mut s = "some_cmd | other_cmd | another_cmd";
    let ps = &mut s;
    let cmd =
        PipeCmd { left : box ExecCmd { argv: vec!("some_cmd") },
                  right: box PipeCmd { left : box ExecCmd { argv: vec!("other_cmd") },
                                       right: box ExecCmd { argv: vec!("another_cmd") }}};
    let parsed = parse_pipe(ps);
    println!("{}", parsed);
    assert!(cmd == parsed);
}

fn peek(ps: &mut &str, toks: &str) -> bool {
    if (*ps).len() == 0
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
    unsafe { path.with_c_str(|c_path| fcntl::open(c_path, oflag, mode)) }
}

fn close(fd: c_int) -> c_int {
    unsafe { unistd::close(fd) }
}

fn chdir(dir: Path) -> c_int {
    unsafe { dir.with_c_str(|c_dir| unistd::chdir(c_dir)) }
}

fn fork() -> pid_t {
    unsafe { unistd::fork() }
}

fn fork_or_fail() -> pid_t {
    let pid = fork();
    if pid < 0
        { panic!("cannot fork") }
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
        match syscalls::wait(0 as *mut c_int) {
            -1 => panic!("cannot wait"),
            pid => { debug!("reaped {}", pid); pid }
        }
    }
}

fn exit(status: c_int) -> ! {
    unsafe { stdlib::exit(status) }
}

fn execv(path: Path, args: Vec<&str>) -> c_int {
    // We need to have valid CString instances for .as_ptr()s to be valid.
    // See http://doc.rust-lang.org/std/c_str/struct.CString.html#method.as_ptr
    let cstrings : Vec<CString> = args.iter().map(|s| s.to_c_str()).collect();
    let mut argv : Vec<*const i8> = cstrings.iter().map(|s| s.as_ptr()).collect();
    argv.push(0 as *const i8);
    stderr(format!("argv: {}\n", argv));
    unsafe {
        path.with_c_str(|c_path| {
            unistd::execv( c_path, argv.as_mut_slice().as_mut_ptr() )
        })
    }
}

fn pipe() -> Option<(c_int, c_int)> {
    unsafe {
        let mut fds = [0,0];
        if unistd::pipe(fds.as_mut_ptr()) < 0
            { None }
        else
            { Some ((fds[0], fds[1])) }
    }
}

fn dup(fd: c_int) -> c_int {
    unsafe {
        match unistd::dup(fd) {
            -1 => panic!("cannot dup"),
            newfd => newfd
        }
    }
}

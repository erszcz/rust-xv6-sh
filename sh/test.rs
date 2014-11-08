use super::{
    ExecCmd,
    PipeCmd,
    PrintablePath,
    RedirCmd,
    get_token,
    parse_cmd,
    parse_exec,
    parse_pipe,
    parse_redirs,
    peek
};

#[test]
fn parse_redir_test() {
    let execcmd = ExecCmd { argv: vec!("some_cmd") };
    let mut s = " > some_file";
    let p = &mut s;
    let redircmd = parse_redirs(execcmd.clone(), p);
    assert!(redircmd == RedirCmd { cmd: box execcmd,
                                   file: PrintablePath { path: Path::new("some_file") },
                                   oflags: super::O_WRONLY | super::O_CREATE,
                                   fd: 1 as i32 });
}

#[test]
fn parse_exec_simple_test() {
    let cmd = ExecCmd { argv: vec!("some_cmd") };
    let cmdline = "some_cmd";
    assert!(cmd == parse_cmd(cmdline));
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

#[test]
fn get_token_simple_command_test() {
    let mut s = "/bin/echo a";
    let p = &mut s;
    {
        let tok = get_token(p).unwrap();
        println!("kind    : {}", tok.kind);
        println!("parsed  : {}", *p);
        println!("token   : {}", tok.buf);
        assert!(tok.kind == super::Regular);
        assert!(*p == "a");
        assert!(tok.buf == "/bin/echo");
    }
    {
        let tok = get_token(p).unwrap();
        println!("kind    : {}", tok.kind);
        println!("p len   : {}", (*p).len());
        println!("parsed  : {}", if (*p).len() == 0 { "(empty)" } else { *p });
        println!("token   : {}", tok.buf);
        assert!(tok.kind == super::Regular);
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
        assert!(tok.kind == super::LRedir);
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
        assert!(tok.kind == super::RRedir);
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
        assert!(tok.kind == super::Append);
        assert!(*p == "a");
        assert!(tok.buf == ">>");
        assert!(tok.buf.len() == 2);
    }
}

#[test]
fn next_char_test() {
    let mut s = "abc";
    {
        let p = &mut s;
        let t = super::next_char(p);
        assert!(*p == "bc");
        assert!(t == "bc");
    }
    assert!(s == "bc");
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

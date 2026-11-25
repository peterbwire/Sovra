// Generated programs can exceed Windows' command-line limit.
pub fn run_node_source(source: &[u8]) -> std::io::Result<std::process::Output> {
    use std::io::Write as _;
    use std::process::Stdio;
    let mut child = std::process::Command::new("node")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().expect("stdin").write_all(source)?;
    child.wait_with_output()
}

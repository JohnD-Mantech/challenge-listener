use std::{
    env::args,
    io::{Read, Write},
    net::TcpListener,
    process::{Command, Stdio},
    thread,
};

fn main() -> std::io::Result<()> {
    let args = args().collect::<Vec<String>>();

    if args.len() < 3 {
        println!("Invalid arguments.");
        println!("Usage: challListener PORT COMMAND");
        return Ok(());
    }

    let port = args[1].clone();

    let command = args[2..].join(" ");

    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(listener) => listener,
        Err(e) => panic!("Failed to init listiner: {}", e),
    };

    for stream in listener.incoming() {
        handle_client(command.clone(), stream?).unwrap();
    }

    Ok(())
}

fn handle_client(command: String, mut stream: std::net::TcpStream) -> std::io::Result<()> {
    thread::spawn(move || {
        println!("Handling connection: {}", stream.peer_addr().unwrap());
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();

        let mut child_stdin = child.stdin.take().expect("Failed to capture child stdin");
        let mut child_stdout = child.stdout.take().expect("Failed to capture child stdout");
        // let mut child_stderr = child.stderr.take().expect("Failed to capture child stderr");

        let mut stdin_stream = stream.try_clone().unwrap();
        let handle_stdin = thread::spawn(move || {
            let mut buffer = [0; 512];
            loop {
                match stdin_stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Err(e) = child_stdin.write_all(&buffer[..n]) {
                            eprintln!("Failed to write to child: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read from tcp stream: {}", e);
                        break;
                    }
                }
            }
        });

        let handle_stdout = thread::spawn(move || {
            let mut buffer = [0; 512];
            loop {
                match child_stdout.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Err(e) = stream.write_all(&buffer[..n]) {
                            eprintln!("Failed to write to tcp stream: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to read from child: {}", e);
                        break;
                    }
                }
            }
        });

        handle_stdin.join().expect("The stdin thread has panicked");
        handle_stdout
            .join()
            .expect("The stdout thread has panicked");
        println!("Pipes closed");
        let _ = child.wait();
        println!("Thread closed");
    });

    Ok(())
}

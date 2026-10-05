use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::sync::mpsc;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use sqlx::PgPool;

const PLAYER: &str = env!("CARGO_BIN_EXE_player");

fn database_url(pool: &PgPool) -> String {
    let options = pool.connect_options();

    format!(
        "postgres://{}@{}:{}/{}",
        options.get_username(),
        options.get_host(),
        options.get_port(),
        options.get_database().unwrap()
    )
}

/// The port in a `listening on 0.0.0.0:<port>` log line.
fn listening_port(line: &str) -> Option<u16> {
    let address = line.split("listening on ").nth(1)?;

    address.trim().rsplit(':').next()?.parse().ok()
}

#[sqlx::test]
async fn player_serves_until_interrupted(pool: PgPool) {
    let web_root = tempfile::tempdir().unwrap();
    let mut server = Command::new(PLAYER)
        .env("DATABASE_URL", database_url(&pool))
        .env("PORT", "0")
        .env("WEB_ROOT", web_root.path())
        .env("NO_COLOR", "1")
        .env_remove("RUST_LOG")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let port = BufReader::new(server.stdout.take().unwrap())
        .lines()
        .map_while(Result::ok)
        .find_map(|line| listening_port(&line))
        .expect("the server logs the port it listens on");

    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .write_all(
            b"GET /api/sessions/active HTTP/1.1\r\nHost: player\r\nConnection: close\r\n\r\n",
        )
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();

    Command::new("kill")
        .args(["-INT", &server.id().to_string()])
        .status()
        .unwrap();

    assert!(response.starts_with("HTTP/1.1 401"), "{response}");
    assert!(server.wait().unwrap().success());
}

/// Reads the terminal's output on a thread, since reads block until the
/// program writes.
fn terminal_output(mut reader: Box<dyn Read + Send>) -> mpsc::Receiver<String> {
    let (sender, receiver) = mpsc::channel();

    std::thread::spawn(move || {
        let mut buffer = [0_u8; 512];
        while let Ok(count @ 1..) = reader.read(&mut buffer) {
            if sender
                .send(String::from_utf8_lossy(&buffer[..count]).to_string())
                .is_err()
            {
                break;
            }
        }
    });

    receiver
}

fn wait_for(output: &mpsc::Receiver<String>, seen: &mut String, text: &str) {
    while !seen.contains(text) {
        seen.push_str(&output.recv().expect("the program printed {text:?}"));
    }
}

#[sqlx::test]
async fn player_user_add_reads_the_password_from_the_terminal(pool: PgPool) {
    let terminal = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();

    let mut command = CommandBuilder::new(PLAYER);
    command.args(["user", "add", "carol"]);
    command.env("DATABASE_URL", database_url(&pool));
    command.env("NO_COLOR", "1");

    let mut child = terminal.slave.spawn_command(command).unwrap();
    drop(terminal.slave);

    let output = terminal_output(terminal.master.try_clone_reader().unwrap());
    let mut input = terminal.master.take_writer().unwrap();
    let mut seen = String::new();

    wait_for(&output, &mut seen, "Password: ");
    input.write_all(b"correctbattery\n").unwrap();
    wait_for(&output, &mut seen, "Confirm password: ");
    input.write_all(b"correctbattery\n").unwrap();

    assert!(child.wait().unwrap().success(), "{seen}");
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT username FROM users WHERE username = 'carol'")
            .fetch_optional(&pool)
            .await
            .unwrap(),
        Some("carol".to_string())
    );
}

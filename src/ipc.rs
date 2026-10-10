use niri_ipc::{Action, Reply, Request, Response, socket::SOCKET_PATH_ENV};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

fn socket_path() -> std::io::Result<std::ffi::OsString> {
    std::env::var_os(SOCKET_PATH_ENV).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{SOCKET_PATH_ENV} is not set, are you running this within niri?"),
        )
    })
}

pub async fn connect() -> std::io::Result<BufReader<UnixStream>> {
    let path = socket_path()?;
    Ok(BufReader::new(UnixStream::connect(path).await?))
}

pub async fn send(socket: &mut BufReader<UnixStream>, request: Request) -> std::io::Result<Reply> {
    let mut buf = serde_json::to_string(&request).unwrap();
    buf.push('\n');
    socket.get_mut().write_all(buf.as_bytes()).await?;

    buf.clear();
    socket.read_line(&mut buf).await?;

    let reply = serde_json::from_str(&buf)?;
    Ok(reply)
}

pub async fn action(
    socket: &mut BufReader<UnixStream>,
    action: Action,
) -> std::io::Result<Response> {
    match send(socket, Request::Action(action)).await? {
        Ok(response) => Ok(response),
        Err(message) => Err(std::io::Error::other(format!("niri error: {message}"))),
    }
}

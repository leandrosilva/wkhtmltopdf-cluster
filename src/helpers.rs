use std::fs;
use std::io;
use std::path::Path;
use zmq;

pub fn create_dir_if_not_exists(output_dir: &Path) -> io::Result<()> {
    if output_dir.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(output_dir)
}

pub fn zmq_assert_empty(socket: &zmq::Socket, expect_message: &str) {
    assert!(socket.recv_string(0).expect(expect_message).unwrap().is_empty());
}

pub fn zmq_recv_string(socket: &zmq::Socket, expect_message: &str) -> String {
    socket.recv_string(0).expect(expect_message).unwrap()
}

pub fn zmq_send<T: zmq::Sendable>(socket: &zmq::Socket, data: T, expect_message: &str) {
    socket.send(data, 0).expect(expect_message);
}

pub fn zmq_send_multipart<I, T>(socket: &zmq::Socket, frames: I, expect_message: &str)
where
    I: IntoIterator<Item = T>,
    T: Into<zmq::Message>,
{
    socket.send_multipart(frames, 0).expect(expect_message);
}

use super::error::Result;
use super::helpers::get_uid;
use super::helpers::zmq_helpers::{assert_empty, recv_string, send, send_multipart};
use super::pdf::{get_pdf_setting_value, PDF_GLOBAL_SETTINGS, PDF_OBJECT_SETTINGS};
use super::protocol::*;
use serde_json::Value;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use url::Url;
use wkhtmltopdf::PdfApplication;
use zmq;

const MSG_FAILED_TO_ACQUIRE_LOCK_OF_SERVICE_SOCKET: &str =
    "failed to acquire lock of service socket";

#[derive(Debug)]
pub struct Worker {
    id: u32,
    stop_signal: Arc<AtomicBool>,
    output_dir: PathBuf,
    timeout: Duration,
}

impl Worker {
    pub fn new(
        id: u32,
        stop_signal: Arc<AtomicBool>,
        output_dir: &Path,
        timeout: Duration,
    ) -> Worker {
        let instance = Worker {
            id: id,
            stop_signal: stop_signal,
            output_dir: PathBuf::from(output_dir),
            timeout: timeout,
        };
        instance
    }

    pub fn run<'a, F: 'a + Fn()>(&'a mut self, on_ready: F) -> Result<()> {
        let service_socket_guard =
            create_service_socket(self.id).expect("failed to get a service socket guard");
        let (heartbeat_tx, heartbeat_rx) = channel::<()>();
        self.watch_eventloop(heartbeat_rx);
        self.run_eventloop(service_socket_guard.clone(), heartbeat_tx, on_ready)
    }

    fn watch_eventloop(&self, heartbeat_rx: Receiver<()>) {
        let id = self.id;
        let timeout = self.timeout;
        let stop_signal = self.stop_signal.clone();

        thread::spawn(move || {
            while !stop_signal.load(Ordering::SeqCst) {
                if heartbeat_rx.recv_timeout(timeout).is_err() {
                    println!(
                        "[#{}] Hugging for more than {}s and will terminate now",
                        id,
                        timeout.as_secs()
                    );
                    process::exit(666);
                }
            }
        });
    }

    fn run_eventloop<'a, F: 'a + Fn()>(
        &'a mut self,
        service_socket_guard: Arc<Mutex<zmq::Socket>>,
        heartbeat_tx: Sender<()>,
        on_ready: F,
    ) -> Result<()> {
        // Enclosing scope for PdfApplication
        {
            let mut pdf_app = PdfApplication::new().expect("failed to init PDF application");

            // worker is ready, so notify it
            on_ready();
            send_messsage(
                service_socket_guard.clone(),
                "READY",
                "failed sending <READY> to broker",
            );

            while !self.stop_signal.load(Ordering::SeqCst) {
                // send heartbeat to sign it's not hugging
                if let Err(reason) = heartbeat_tx.send(()) {
                    println!(
                        "[#{}] Something went weird with the heartbeat monitoring: {:?}",
                        self.id, reason
                    );
                    break;
                }

                // try to grap some reply message from broker, which might be a command or a
                // client ID followed by an actual request
                let message = match recv_message(service_socket_guard.clone()) {
                    Ok(received) => received.unwrap(),
                    Err(_) => continue,
                };
                println!("[#{}] Message: {}", self.id, message);

                // -- from broker
                if message == "STOP" {
                    send_messsage(
                        service_socket_guard.clone(),
                        "GONE",
                        "failed to send <GONE> response",
                    );
                    break;
                }

                // -- from client
                let (client_id, request) =
                    self.read_client_request(service_socket_guard.clone(), message);
                println!("[#{}] Client #{} request: {}", self.id, client_id, request);

                self.handle_client_request(
                    service_socket_guard.clone(),
                    &client_id,
                    &request,
                    &mut pdf_app,
                );
            }
        }

        println!("[#{}] Stopping...", self.id);
        finish_service_socket(service_socket_guard.clone());
        println!("[#{}] Disconnected from broker to stop", self.id);

        Ok(())
    }

    fn read_client_request(
        &self,
        service_socket_guard: Arc<Mutex<zmq::Socket>>,
        message: String,
    ) -> (String, String) {
        let service_socket = service_socket_guard
            .lock()
            .expect(MSG_FAILED_TO_ACQUIRE_LOCK_OF_SERVICE_SOCKET);
        // read multipart envelope from client as:
        //   CLIENT, EMPTY, REQUEST
        let client_id = message;
        assert_empty(
            &service_socket,
            "failed reading 1nd <EMPTY> of client's envelope",
        );
        let request = recv_string(
            &service_socket,
            "failed reading <REQUEST> from client's envelope",
        );
        (client_id, request)
    }

    fn handle_client_request(
        &self,
        service_socket_guard: Arc<Mutex<zmq::Socket>>,
        client_id: &String,
        request: &String,
        pdf_app: &mut PdfApplication,
    ) {
        // parse request body
        let payload: Value =
            serde_json::from_str(request.as_str()).expect("failed parsing request as JSON");
        println!("[#{}] Client #{} payload: {}", self.id, client_id, payload);

        if payload == Value::Null {
            let err_msg = format!("Payload cannot be null");
            println!("{}", err_msg.as_str());

            send_client_reply_with_error(
                service_socket_guard.clone(),
                &client_id,
                REP_400_BAD_REQUEST,
                &err_msg,
            );
            return;
        }

        // parse the actual request
        let message_id = get_uid();
        let url = match payload["url"].as_str() {
            Some(s) => match Url::from_str(&s) {
                Ok(parsed) => parsed,
                Err(_) => {
                    let err_msg = format!("Cannot parse URL: {}", payload);
                    println!(
                        "[#{}] Reply to client #{}: {}",
                        self.id,
                        client_id,
                        err_msg.as_str()
                    );

                    send_client_reply_with_error(
                        service_socket_guard.clone(),
                        &client_id,
                        REP_400_BAD_REQUEST,
                        &err_msg,
                    );
                    return;
                }
            },
            None => {
                let err_msg = format!("URL is missing in request payload: {}", payload);
                println!(
                    "[#{}] Reply to client #{}: {}",
                    self.id,
                    client_id,
                    err_msg.as_str()
                );

                send_client_reply_with_error(
                    service_socket_guard.clone(),
                    &client_id,
                    REP_400_BAD_REQUEST,
                    &err_msg,
                );
                return;
            }
        };
        let filepath = self.output_dir.join(Path::new(
            format!("req-{}-{}.pdf", self.id, message_id).as_str(),
        ));

        // actual pdf building
        unsafe {
            let pdf_builder = pdf_app.builder();

            // global converter settings
            let mut pdf_global_settings = pdf_builder
                .global_settings()
                .expect("failed to create global settings");

            if let Value::Object(json_global_settings) = &payload["global"] {
                for (json_key, json_value) in json_global_settings {
                    if let Some(pdf_setting) = PDF_GLOBAL_SETTINGS.get(json_key.as_str()) {
                        match get_pdf_setting_value(pdf_setting, json_value) {
                            Ok(v) => pdf_global_settings.set(json_key, v.as_str()).expect(
                                format!("failed setting global option {}", &json_key).as_str(),
                            ),
                            Err(e) => {
                                send_client_reply_with_error(
                                    service_socket_guard.clone(),
                                    &client_id,
                                    REP_400_BAD_REQUEST,
                                    &e.details,
                                );
                                return;
                            }
                        }
                    }
                }
            }

            // object page settings
            let mut pdf_object_settings = pdf_builder
                .object_settings()
                .expect("failed to create object settings");

            if let Value::Object(json_object_setting) = &payload["object"] {
                for (json_key, json_value) in json_object_setting {
                    if let Some(pdf_setting) = PDF_OBJECT_SETTINGS.get(json_key.as_str()) {
                        match get_pdf_setting_value(pdf_setting, json_value) {
                            Ok(v) => pdf_object_settings.set(json_key, v.as_str()).expect(
                                format!("failed setting object option {}", &json_key).as_str(),
                            ),
                            Err(e) => {
                                send_client_reply_with_error(
                                    service_socket_guard.clone(),
                                    &client_id,
                                    REP_400_BAD_REQUEST,
                                    &e.details,
                                );
                                return;
                            }
                        }
                    }
                }
            }

            let mut pdf_converter = pdf_global_settings.create_converter();
            pdf_converter.add_page_object(pdf_object_settings, url.as_str());

            // warning behavior
            let local_id = self.id;
            let local_client_id = client_id.clone();
            let local_service_socket_guard = service_socket_guard.clone();
            pdf_converter.set_warning_callback(Some(Box::new(move |warn| {
                println!("[#{}] Warning: {}", local_id, warn);
                send_client_reply_with_error(
                    local_service_socket_guard.clone(),
                    &local_client_id,
                    REP_502_BAD_GATEWAY,
                    &warn,
                );
                // Waits just a bit to let message goes to client
                thread::sleep(Duration::from_millis(50));
                panic!(
                    "worker #{} for client #{} is aborting due to potential JavaScript error",
                    local_id, local_client_id
                );
            })));

            // build
            let mut pdf_out = pdf_converter.convert().expect(
                format!(
                    "failed to convert {} to {}",
                    url,
                    filepath.to_str().unwrap()
                )
                .as_str(),
            );

            // save
            let mut pdf_file = File::create(&filepath)
                .expect(format!("failed to create {}", filepath.to_str().unwrap()).as_str());
            let pdf_bytes =
                io::copy(&mut pdf_out, &mut pdf_file).expect("failed to write to basic.pdf");
            println!(
                "[#{}] Wrote {} bytes to file: {}",
                self.id,
                pdf_bytes,
                filepath.to_str().unwrap()
            );
        }

        println!(
            "[#{}] PDF built for client #{}: {}",
            self.id,
            client_id,
            filepath.to_str().unwrap()
        );

        // TODO: reply with pdf binary content instead of this dummy message
        let content = format!("PDF saved at output directory");

        send_client_reply_with_success(service_socket_guard.clone(), &client_id, &content);
    }
}

// Service socket
//

fn create_service_socket(id: u32) -> Result<Arc<Mutex<zmq::Socket>>> {
    let socket_id = format!("W{}", id);
    let context = zmq::Context::new();
    let service_socket = context.socket(zmq::REQ).unwrap();
    service_socket.set_identity(socket_id.as_bytes())?;
    service_socket.set_sndtimeo(1000)?;
    service_socket.set_rcvtimeo(1000)?;
    service_socket
        .connect("tcp://127.0.0.1:6661")
        .expect("failed listening on port 6661");
    let guard = Arc::new(Mutex::new(service_socket));
    Ok(guard)
}
fn finish_service_socket(service_socket_guard: Arc<Mutex<zmq::Socket>>) {
    let service_socket = service_socket_guard
        .lock()
        .expect(MSG_FAILED_TO_ACQUIRE_LOCK_OF_SERVICE_SOCKET);
    service_socket
        .disconnect("tcp://127.0.0.1:6661")
        .expect("failed disconnecting on port 6661");
}

fn send_messsage(
    service_socket_guard: Arc<Mutex<zmq::Socket>>,
    message: &str,
    expect_message: &str,
) {
    let service_socket = service_socket_guard
        .lock()
        .expect(MSG_FAILED_TO_ACQUIRE_LOCK_OF_SERVICE_SOCKET);
    send(&service_socket, message, expect_message);
}

fn recv_message(
    service_socket_guard: Arc<Mutex<zmq::Socket>>,
) -> std::result::Result<std::result::Result<String, Vec<u8>>, zmq::Error> {
    let service_socket = service_socket_guard
        .lock()
        .expect(MSG_FAILED_TO_ACQUIRE_LOCK_OF_SERVICE_SOCKET);
    service_socket.recv_string(0)
}

fn send_client_reply_with_success(
    service_socket_guard: Arc<Mutex<zmq::Socket>>,
    client_id: &String,
    content: &String,
) {
    send_client_reply(
        service_socket_guard.clone(),
        &client_id,
        REP_200_SUCCESS,
        &content,
    );
}

fn send_client_reply_with_error(
    service_socket_guard: Arc<Mutex<zmq::Socket>>,
    client_id: &String,
    err_code: &str,
    err_msg: &String,
) {
    send_client_reply(
        service_socket_guard.clone(),
        &client_id,
        &err_code,
        &err_msg,
    );
}

fn send_client_reply(
    service_socket_guard: Arc<Mutex<zmq::Socket>>,
    client_id: &String,
    reply_type: &str,
    reply_content: &String,
) {
    let service_socket = service_socket_guard
        .lock()
        .expect(MSG_FAILED_TO_ACQUIRE_LOCK_OF_SERVICE_SOCKET);
    // build reply multipart envelope to client as:
    //   CLIENT, EMPTY, REPLY|ERROR, EMPTY, CONTENT
    let reply_envelope = vec![
        client_id.as_bytes().to_vec(),
        b"".to_vec(),
        reply_type.as_bytes().to_vec(),
        b"".to_vec(),
        reply_content.as_bytes().to_vec(),
    ];

    send_multipart(
        &service_socket,
        reply_envelope,
        format!("failed sending reply to client #{}", client_id).as_str(),
    );
}

// Unit testing
//

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_worker() {
        let worker = Worker::new(
            123,
            Arc::new(AtomicBool::new(false)),
            Path::new("out"),
            Duration::from_secs(3),
        );
        assert_eq!(worker.id, 123);
        assert_eq!(worker.output_dir.as_os_str(), "out");
    }

    #[test]
    fn start_worker() {
        assert!(true);
    }

    #[test]
    fn stop_worker() {
        assert!(true);
    }
}

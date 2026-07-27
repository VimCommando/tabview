use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

#[derive(Clone)]
pub struct Response {
    pub status: u16,
    pub body: &'static str,
    pub delay: Duration,
}

impl Response {
    pub fn ok(body: &'static str) -> Self {
        Self {
            status: 200,
            body,
            delay: Duration::ZERO,
        }
    }

    pub fn delayed(body: &'static str, delay: Duration) -> Self {
        Self {
            status: 200,
            body,
            delay,
        }
    }

    pub fn error(status: u16, body: &'static str) -> Self {
        Self {
            status,
            body,
            delay: Duration::ZERO,
        }
    }
}

pub struct Server {
    endpoint: String,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    pub fn start(responses: Vec<Response>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock Elasticsearch");
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let worker_requests = requests.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
        let worker = std::thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("mock accept: {error}"),
                };
                stream.set_nonblocking(false).unwrap();
                let connection_requests = worker_requests.clone();
                let connection_responses = responses.clone();
                std::thread::spawn(move || {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut request = Vec::new();
                    let mut chunk = [0_u8; 4096];
                    let header_end = loop {
                        let count = match stream.read(&mut chunk) {
                            Ok(0) => return,
                            Ok(count) => count,
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                                ) =>
                            {
                                return;
                            }
                            Err(error) => panic!("read mock request: {error}"),
                        };
                        request.extend_from_slice(&chunk[..count]);
                        if let Some(index) =
                            request.windows(4).position(|window| window == b"\r\n\r\n")
                        {
                            break index + 4;
                        }
                    };
                    let header = String::from_utf8_lossy(&request[..header_end]);
                    let content_length = header
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                        .unwrap_or_default();
                    while request.len().saturating_sub(header_end) < content_length {
                        let count = match stream.read(&mut chunk) {
                            Ok(0) => return,
                            Ok(count) => count,
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                                ) =>
                            {
                                return;
                            }
                            Err(error) => panic!("read mock body: {error}"),
                        };
                        request.extend_from_slice(&chunk[..count]);
                    }
                    connection_requests
                        .lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(&request).into_owned());
                    let response = connection_responses
                        .lock()
                        .unwrap()
                        .pop_front()
                        .expect("unexpected Elasticsearch request");
                    if !response.delay.is_zero() {
                        std::thread::sleep(response.delay);
                    }
                    let reason = if response.status < 400 {
                        "OK"
                    } else {
                        "Bad Request"
                    };
                    write!(
                        stream,
                        "HTTP/1.1 {} {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        response.status,
                        reason,
                        response.body.len(),
                        response.body
                    )
                    .expect("mock response");
                    stream.flush().expect("flush mock response");
                    stream.shutdown(Shutdown::Write).ok();
                    stream
                        .set_read_timeout(Some(Duration::from_millis(200)))
                        .ok();
                    while stream.read(&mut chunk).is_ok_and(|count| count > 0) {}
                });
            }
        });
        Self {
            endpoint,
            requests,
            stop,
            worker: Some(worker),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let result = worker.join();
            if !std::thread::panicking() {
                result.expect("mock Elasticsearch worker panicked");
            }
        }
    }
}

// Exercise the real transport state machine with deterministic socket/TLS boundaries.
// These tests validate polling and request ordering, not cryptography or live interoperability.
extern crate alloc;
extern crate self as nonos_libc;
extern crate self as nonos_tls;
use nonos_chat_protocol_tests::protocol;
use std::{cell::RefCell, collections::VecDeque};
#[path = "../capsule/src/transport.rs"]
mod transport;
#[derive(Default)]
struct State {
    now: i64,
    incoming: VecDeque<Result<Vec<u8>, ()>>,
    sent: Vec<Vec<u8>>,
    closed: usize,
}
thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }
pub fn mk_uptime_ms() -> i64 {
    STATE.with(|s| s.borrow().now)
}
pub fn rtc_now() -> u64 {
    1
}
pub mod flight {
    pub struct ClientFlight {
        pub record: Vec<u8>,
        pub private: Vec<u8>,
    }
}
pub struct TrafficKeys;
pub struct Complete {
    pub app: TrafficKeys,
}
pub fn client_flight(_: &[u8]) -> Option<flight::ClientFlight> {
    Some(flight::ClientFlight {
        record: b"client hello".to_vec(),
        private: vec![1; 32],
    })
}
pub fn server_finished_flight_ready(bytes: &[u8]) -> bool {
    bytes == b"verified server flight"
}
pub fn application_write(
    _: &flight::ClientFlight,
    flight: &[u8],
    request: &[u8],
    _: &[u8],
    _: u64,
) -> Option<Vec<u8>> {
    server_finished_flight_ready(flight).then(|| request.to_vec())
}
pub fn server_complete(
    _: &flight::ClientFlight,
    flight: &[u8],
    _: &[u8],
    _: u64,
) -> Option<Complete> {
    server_finished_flight_ready(flight).then_some(Complete { app: TrafficKeys })
}
pub fn application_plaintext_cached(_: &TrafficKeys, bytes: &[u8]) -> Vec<u8> {
    bytes.to_vec()
}
mod net {
    use super::STATE;
    pub fn lookup(_: &[u8]) -> u32 {
        1
    }
    pub fn socket_open(_: u32) -> Result<u32, ()> {
        Ok(7)
    }
    pub fn socket_connect_host(_: u32, _: u32, _: &str, _: u16) -> Result<(), ()> {
        Ok(())
    }
    pub fn socket_send(_: u32, _: u32, bytes: &[u8]) -> Result<(), ()> {
        STATE.with(|s| s.borrow_mut().sent.push(bytes.to_vec()));
        Ok(())
    }
    pub fn socket_recv(_: u32, _: u32, out: &mut [u8]) -> Result<usize, ()> {
        let data = STATE
            .with(|s| s.borrow_mut().incoming.pop_front())
            .unwrap_or(Err(()))?;
        assert!(data.len() <= out.len());
        out[..data.len()].copy_from_slice(&data);
        Ok(data.len())
    }
    pub fn socket_close(_: u32, _: u32) {
        STATE.with(|s| s.borrow_mut().closed += 1);
    }
}
fn job() -> transport::Job {
    STATE.with(|s| *s.borrow_mut() = State::default());
    transport::Job::new(
        protocol::Endpoint::parse("https://example.com/v1").unwrap(),
        b"authorized request".to_vec(),
    )
}
fn queue(data: Result<&[u8], ()>) {
    STATE.with(|s| s.borrow_mut().incoming.push_back(data.map(|b| b.to_vec())));
}
#[test]
fn transient_empty_polls_do_not_abort_handshake_or_response() {
    let mut j = job();
    assert!(j.poll().unwrap().is_none()); // connect
    assert!(j.poll().unwrap().is_none()); // client hello
    queue(Err(()));
    assert!(j.poll().unwrap().is_none()); // temporary empty receive
    STATE.with(|s| assert_eq!(s.borrow().sent, vec![b"client hello".to_vec()]));
    queue(Ok(b"verified server flight"));
    assert!(j.poll().unwrap().is_none());
    assert!(j.poll().unwrap().is_none()); // application request after verification
    queue(Err(()));
    assert!(j.poll().unwrap().is_none());
    queue(Ok(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}"));
    let reply = j.poll().unwrap().expect("complete response");
    assert_eq!(reply.status, 200);
    assert_eq!(reply.body, b"{}");
    STATE.with(|s| assert_eq!(s.borrow().sent[1], b"authorized request"));
    drop(j);
    STATE.with(|s| assert_eq!(s.borrow().closed, 1));
}
#[test]
fn silent_handshake_still_times_out_without_sending_request() {
    let mut j = job();
    j.poll().unwrap();
    j.poll().unwrap();
    STATE.with(|s| s.borrow_mut().now = 20_001);
    assert_eq!(j.poll().err(), Some("TLS handshake timed out."));
    STATE.with(|s| assert_eq!(s.borrow().sent.len(), 1));
}
#[test]
fn silent_response_still_times_out() {
    let mut j = job();
    j.poll().unwrap();
    j.poll().unwrap();
    queue(Ok(b"verified server flight"));
    j.poll().unwrap();
    j.poll().unwrap();
    STATE.with(|s| s.borrow_mut().now = 90_001);
    assert_eq!(j.poll().err(), Some("Request timed out. Please try again."));
}

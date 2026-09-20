// SPDX-License-Identifier: AGPL-3.0-or-later
use crate::{
    net,
    protocol::{self, Endpoint, Response, MAX_RESPONSE},
};
use alloc::vec::Vec;
use nonos_tls::{flight::ClientFlight, TrafficKeys};
use zeroize::Zeroize;
pub struct Job {
    endpoint: Endpoint,
    port: u32,
    handle: u32,
    stage: u8,
    request: Vec<u8>,
    outgoing: Vec<u8>,
    offset: usize,
    client: Option<ClientFlight>,
    flight: Vec<u8>,
    keys: Option<TrafficKeys>,
    encrypted: Vec<u8>,
    started: i64,
    last_data: i64,
    wall: u64,
}
impl Job {
    pub fn new(endpoint: Endpoint, request: Vec<u8>) -> Self {
        let now = nonos_libc::mk_uptime_ms();
        Self {
            endpoint,
            port: 0,
            handle: 0,
            stage: 0,
            request,
            outgoing: Vec::new(),
            offset: 0,
            client: None,
            flight: Vec::new(),
            keys: None,
            encrypted: Vec::new(),
            started: now,
            last_data: now,
            wall: nonos_tls::rtc_now(),
        }
    }
    pub fn poll(&mut self) -> Result<Option<Response>, &'static str> {
        let now = nonos_libc::mk_uptime_ms();
        if now - self.started > 180_000 || (self.stage >= 4 && now - self.last_data > 90_000) {
            return Err("Request timed out. Please try again.");
        }
        match self.stage {
            0 => {
                self.port = net::lookup(b"net.sockets");
                if self.port == 0 {
                    return Err("NONOS network service is unavailable.");
                }
                self.handle =
                    net::socket_open(self.port).map_err(|_| "Could not open a network socket.")?;
                net::socket_connect_host(
                    self.port,
                    self.handle,
                    &self.endpoint.host,
                    self.endpoint.port,
                )
                .map_err(|_| "DNS lookup or TCP connection failed.")?;
                let cf = nonos_tls::client_flight(self.endpoint.host.as_bytes())
                    .ok_or("TLS initialization failed.")?;
                self.outgoing = cf.record.clone();
                self.client = Some(cf);
                self.stage = 1;
            }
            1 | 3 => {
                let end = (self.offset + 1200).min(self.outgoing.len());
                net::socket_send(self.port, self.handle, &self.outgoing[self.offset..end])
                    .map_err(|_| "Network error while sending.")?;
                self.offset = end;
                if self.offset == self.outgoing.len() {
                    self.outgoing.zeroize();
                    self.offset = 0;
                    self.stage += 1;
                    self.last_data = now;
                }
            }
            2 => {
                if now - self.last_data > 20_000 {
                    return Err("TLS handshake timed out.");
                }
                let mut b = [0u8; 4096];
                // The upstream socket adapter also returns Err for a transient empty poll.
                // Match the browser transport: retry within the idle/total deadlines.
                let n = net::socket_recv(self.port, self.handle, &mut b).unwrap_or(0);
                if n > 0 {
                    self.last_data = now;
                    if self.flight.len() + n > 128 * 1024 {
                        return Err("TLS handshake is too large.");
                    }
                    self.flight.extend_from_slice(&b[..n]);
                }
                if nonos_tls::server_finished_flight_ready(&self.flight) {
                    let cf = self.client.as_ref().ok_or("Invalid TLS state.")?;
                    self.outgoing = nonos_tls::application_write(
                        cf,
                        &self.flight,
                        &self.request,
                        self.endpoint.host.as_bytes(),
                        self.wall,
                    )
                    .ok_or("TLS certificate or handshake is invalid. The request was not sent.")?;
                    self.keys = Some(
                        nonos_tls::server_complete(
                            cf,
                            &self.flight,
                            self.endpoint.host.as_bytes(),
                            self.wall,
                        )
                        .ok_or("TLS server verification failed.")?
                        .app,
                    );
                    self.request.zeroize();
                    self.stage = 3;
                }
            }
            4 => {
                let mut b = [0u8; 4096];
                let n = net::socket_recv(self.port, self.handle, &mut b).unwrap_or(0);
                if n > 0 {
                    self.last_data = now;
                    if self.encrypted.len() + n > MAX_RESPONSE + 32768 {
                        return Err("Response is too large.");
                    }
                    self.encrypted.extend_from_slice(&b[..n]);
                    let keys = self.keys.as_ref().ok_or("Missing TLS state.")?;
                    let plain = nonos_tls::application_plaintext_cached(keys, &self.encrypted);
                    return protocol::response(&plain);
                }
            }
            _ => return Err("Invalid connection state."),
        }
        Ok(None)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        if self.port != 0 && self.handle != 0 {
            net::socket_close(self.port, self.handle);
        }
        self.request.zeroize();
        self.outgoing.zeroize();
        if let Some(cf) = self.client.as_mut() {
            cf.private.zeroize();
        }
    }
}

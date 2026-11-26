//! Real independent loopback client against the bounded trusted Rust socket owner.
#![cfg(target_os = "linux")]
use sovra::compiler::v2::*;
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, TcpStream},
    time::Duration,
};
fn ok() -> Result<(), &'static str> {
    Ok(())
}
fn network(limits: NetworkLimits) -> LoopbackNetwork {
    let mut network = LoopbackNetwork::new(limits).unwrap();
    network.grant_port(0).unwrap();
    network
}
fn client(port: u16) -> TcpStream {
    let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
}
#[test]
fn default_denial_lowerable_limits_and_finite_exact_port_grants() {
    let denied = LoopbackNetwork::default();
    assert_eq!(denied.listen(0, ok).unwrap().unwrap_err().code, "authority");
    assert_eq!(denied.usage(), (0, 0));
    for limits in [
        NetworkLimits {
            listeners: 9,
            ..NetworkLimits::DEFAULT
        },
        NetworkLimits {
            streams: 65,
            ..NetworkLimits::DEFAULT
        },
        NetworkLimits {
            bytes: 65537,
            ..NetworkLimits::DEFAULT
        },
        NetworkLimits {
            milliseconds: 30001,
            ..NetworkLimits::DEFAULT
        },
    ] {
        assert_eq!(LoopbackNetwork::new(limits).unwrap_err().code, "policy");
    }
    let mut grants = LoopbackNetwork::default();
    for port in 0..16 {
        grants.grant_port(port).unwrap();
    }
    grants.grant_port(0).unwrap();
    assert_eq!(grants.grant_port(16).unwrap_err().code, "limit");
    let none = network(NetworkLimits {
        listeners: 0,
        ..NetworkLimits::DEFAULT
    });
    assert_eq!(none.listen(0, ok).unwrap().unwrap_err().code, "limit");
}
#[test]
fn listener_conflict_cancellation_and_drop_release_reservations() {
    let mut network = network(NetworkLimits {
        listeners: 2,
        ..NetworkLimits::DEFAULT
    });
    let mut checks = 0;
    let cancelled = network.listen(0, || {
        checks += 1;
        if checks == 2 {
            Err("cancelled publication")
        } else {
            Ok(())
        }
    });
    assert_eq!(cancelled.unwrap_err(), "cancelled publication");
    assert_eq!(network.usage(), (0, 0));
    let listener = network.listen(0, ok).unwrap().unwrap();
    let port = listener.port().unwrap();
    // An ephemeral-port grant does not grant a subsequently selected explicit port.
    assert_eq!(
        network.listen(port, ok).unwrap().unwrap_err().code,
        "authority"
    );
    network.grant_port(port).unwrap();
    assert_eq!(network.listen(port, ok).unwrap().unwrap_err().code, "io");
    assert_eq!(network.usage(), (1, 0));
    listener.close();
    assert_eq!(network.usage(), (0, 0));
    let rebound = network.listen(port, ok).unwrap().unwrap();
    drop(rebound);
    assert_eq!(network.usage(), (0, 0));
}
#[test]
fn accept_timeout_and_callback_cancellation_do_not_leak_stream_slots() {
    let network = network(NetworkLimits {
        streams: 1,
        milliseconds: 5,
        ..NetworkLimits::DEFAULT
    });
    let listener = network.listen(0, ok).unwrap().unwrap();
    assert_eq!(listener.accept(ok).unwrap().unwrap_err().code, "timeout");
    assert_eq!(network.usage(), (1, 0));
    let cancelled = listener.accept(|| {
        assert_eq!(network.usage(), (1, 1));
        assert_eq!(listener.accept(ok).unwrap().unwrap_err().code, "limit");
        Err("stop accepting")
    });
    assert_eq!(cancelled.unwrap_err(), "stop accepting");
    assert_eq!(network.usage(), (1, 0));
    let zero = network_zero();
    let listener = zero.listen(0, ok).unwrap().unwrap();
    assert_eq!(listener.accept(ok).unwrap().unwrap_err().code, "timeout");
    assert_eq!(zero.usage(), (1, 0));
}
fn network_zero() -> LoopbackNetwork {
    network(NetworkLimits {
        milliseconds: 0,
        ..NetworkLimits::DEFAULT
    })
}
#[test]
fn binary_chunks_partial_io_oversize_refusal_and_orderly_eof_with_independent_client() {
    let network = network(NetworkLimits {
        bytes: 4,
        milliseconds: 1000,
        ..NetworkLimits::DEFAULT
    });
    let listener = network.listen(0, ok).unwrap().unwrap();
    let port = listener.port().unwrap();
    let peer = std::thread::spawn(move || {
        let mut peer = client(port);
        peer.write_all(&[0, 255, 1, 2, 3, 4]).unwrap();
        let mut reply = [0; 6];
        peer.read_exact(&mut reply).unwrap();
        assert_eq!(reply, [0, 255, 1, 2, 3, 4]);
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    });
    let mut stream = listener.accept(ok).unwrap().unwrap();
    let mut received = Vec::new();
    while received.len() < 6 {
        match stream.read(2, ok).unwrap().unwrap() {
            NetworkRead::Data(bytes) => {
                assert!(bytes.len() <= 2);
                received.extend(bytes);
            }
            NetworkRead::Eof => panic!("premature EOF"),
        }
    }
    assert_eq!(
        stream.write(&[9; 5], ok).unwrap().unwrap_err().code,
        "limit"
    );
    assert_eq!(stream.read(5, ok).unwrap().unwrap_err().code, "limit");
    assert_eq!(stream.read(0, ok).unwrap().unwrap_err().code, "input");
    for chunk in received.chunks(4) {
        let mut sent = 0;
        while sent < chunk.len() {
            sent += stream.write(&chunk[sent..], ok).unwrap().unwrap();
        }
    }
    stream.close().unwrap();
    peer.join().unwrap();
    drop(listener);
    assert_eq!(network.usage(), (0, 0));
}
#[test]
fn accepted_owner_survives_listener_close_and_cancelled_reads_preserve_connection() {
    let network = network(NetworkLimits {
        streams: 1,
        milliseconds: 1000,
        ..NetworkLimits::DEFAULT
    });
    let listener = network.listen(0, ok).unwrap().unwrap();
    let mut peer = client(listener.port().unwrap());
    let mut stream = listener.accept(ok).unwrap().unwrap();
    listener.close();
    assert_eq!(network.usage(), (0, 1));
    assert_eq!(
        stream.read(1, || Err("cancel read")).unwrap_err(),
        "cancel read"
    );
    let next = network.listen(0, ok).unwrap().unwrap();
    assert_eq!(next.accept(ok).unwrap().unwrap_err().code, "limit");
    peer.write_all(&[42]).unwrap();
    assert_eq!(
        stream.read(1, ok).unwrap().unwrap(),
        NetworkRead::Data(vec![42])
    );
    drop(peer);
    assert_eq!(stream.read(1, ok).unwrap().unwrap(), NetworkRead::Eof);
    drop(stream);
    assert_eq!(network.usage(), (1, 0));
    drop(next);
    assert_eq!(network.usage(), (0, 0));
}

#[test]
fn pending_read_deadline_and_cancelled_write_preserve_the_owned_stream() {
    let network = network(NetworkLimits {
        milliseconds: 20,
        ..NetworkLimits::DEFAULT
    });
    let listener = network.listen(0, ok).unwrap().unwrap();
    let mut peer = client(listener.port().unwrap());
    let mut stream = listener.accept(ok).unwrap().unwrap();
    assert_eq!(stream.read(1, ok).unwrap().unwrap_err().code, "timeout");
    assert_eq!(
        stream.write(&[9], || Err("cancel write")).unwrap_err(),
        "cancel write"
    );
    assert_eq!(stream.write(&[], ok).unwrap().unwrap(), 0);
    assert_eq!(stream.write(&[7], ok).unwrap().unwrap(), 1);
    let mut byte = [0];
    peer.read_exact(&mut byte).unwrap();
    assert_eq!(byte, [7]);
    drop(stream);
    drop(listener);
    assert_eq!(network.usage(), (0, 0));
}

#[test]
fn concurrent_embedding_callers_cannot_exceed_the_domain_listener_limit() {
    use std::sync::{Arc, Barrier};
    let network = Arc::new(network(NetworkLimits {
        listeners: 4,
        ..NetworkLimits::DEFAULT
    }));
    let ready = Arc::new(Barrier::new(17));
    let release = Arc::new(Barrier::new(17));
    let mut threads = Vec::new();
    for _ in 0..16 {
        let network = network.clone();
        let ready = ready.clone();
        let release = release.clone();
        threads.push(std::thread::spawn(move || {
            let owner = network.listen(0, ok).unwrap();
            if let Err(ref error) = owner {
                assert_eq!(error.code, "limit");
            }
            ready.wait();
            release.wait();
            owner.is_ok()
        }));
    }
    ready.wait();
    assert_eq!(network.usage(), (4, 0));
    release.wait();
    assert_eq!(
        threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(|accepted| *accepted)
            .count(),
        4
    );
    assert_eq!(network.usage(), (0, 0));
}

#[test]
fn cooperative_write_yields_at_backpressure_and_other_stream_still_progresses() {
    let domain = network(NetworkLimits::DEFAULT);
    let listener = domain.listen(0, ok).unwrap().unwrap();
    let port = listener.port().unwrap();
    let _slow_peer = client(port);
    let mut slow = listener.accept(ok).unwrap().unwrap();
    let mut fast_peer = client(port);
    let mut fast = listener.accept(ok).unwrap().unwrap();
    let chunk = vec![42; NetworkLimits::DEFAULT.bytes];
    let mut blocked = false;
    for _ in 0..1024 {
        match slow.try_write(&chunk, ok).unwrap().unwrap() {
            Some(n) => assert!(n > 0 && n <= chunk.len()),
            None => {
                blocked = true;
                break;
            }
        }
    }
    assert!(
        blocked,
        "non-reading peer must eventually apply backpressure"
    );
    assert_eq!(
        fast.try_write(&vec![0; chunk.len() + 1], ok)
            .unwrap()
            .unwrap_err()
            .code,
        "limit"
    );
    assert_eq!(fast.try_write(b"x", ok).unwrap().unwrap(), Some(1));
    let mut byte = [0];
    fast_peer.read_exact(&mut byte).unwrap();
    assert_eq!(byte, [b'x']);
    assert_eq!(domain.usage(), (1, 2));
    drop(slow);
    drop(fast);
    drop(listener);
    assert_eq!(domain.usage(), (0, 0));
}

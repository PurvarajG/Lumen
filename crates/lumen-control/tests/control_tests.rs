use std::net::SocketAddr;
use std::sync::mpsc;
use std::time::Duration;

use lumen_control::{
    ControlCommand, ControlError, ControlEvent, HttpConfig, HttpServer, MidiMapping, OscListener,
    Source, StatusHandle, StatusSnapshot, map_midi_message, midi_ports,
};

#[test]
fn test_command_json_roundtrip_and_rejection() {
    let cases = vec![
        (ControlCommand::Next, r#"{"cmd":"Next"}"#),
        (ControlCommand::Previous, r#"{"cmd":"Previous"}"#),
        (
            ControlCommand::Trigger { index: 3 },
            r#"{"cmd":"Trigger","index":3}"#,
        ),
        (
            ControlCommand::Blackout { on: true },
            r#"{"cmd":"Blackout","on":true}"#,
        ),
        (
            ControlCommand::Blackout { on: false },
            r#"{"cmd":"Blackout","on":false}"#,
        ),
        (ControlCommand::ClearText, r#"{"cmd":"ClearText"}"#),
        (ControlCommand::ClearAll, r#"{"cmd":"ClearAll"}"#),
        (
            ControlCommand::RunMacro {
                name: "PreShow".to_string(),
            },
            r#"{"cmd":"RunMacro","name":"PreShow"}"#,
        ),
        (
            ControlCommand::TimerStart {
                name: "Clock".to_string(),
            },
            r#"{"cmd":"TimerStart","name":"Clock"}"#,
        ),
        (
            ControlCommand::TimerPause {
                name: "Clock".to_string(),
            },
            r#"{"cmd":"TimerPause","name":"Clock"}"#,
        ),
        (
            ControlCommand::TimerReset {
                name: "Clock".to_string(),
            },
            r#"{"cmd":"TimerReset","name":"Clock"}"#,
        ),
        (
            ControlCommand::StageMessage {
                text: "5 minutes remaining".to_string(),
            },
            r#"{"cmd":"StageMessage","text":"5 minutes remaining"}"#,
        ),
    ];

    for (cmd, expected_json) in cases {
        let serialized = serde_json::to_string(&cmd).expect("serialization failed");
        assert_eq!(serialized, expected_json);
        let deserialized: ControlCommand =
            serde_json::from_str(&serialized).expect("deserialization failed");
        assert_eq!(deserialized, cmd);
    }

    // StageMessage with exactly 500 chars must succeed
    let msg_500 = "a".repeat(500);
    let json_500 = format!(r#"{{"cmd":"StageMessage","text":"{msg_500}"}}"#);
    let cmd_500: ControlCommand = serde_json::from_str(&json_500).expect("500 chars should pass");
    assert_eq!(
        cmd_500,
        ControlCommand::StageMessage {
            text: msg_500.clone()
        }
    );

    // StageMessage with 501 chars must be rejected
    let msg_501 = "a".repeat(501);
    let json_501 = format!(r#"{{"cmd":"StageMessage","text":"{msg_501}"}}"#);
    let err_501 = serde_json::from_str::<ControlCommand>(&json_501);
    assert!(err_501.is_err(), "501 chars must be rejected");

    // Invalid commands must be rejected
    assert!(serde_json::from_str::<ControlCommand>(r#"{"cmd":"NonExistent"}"#).is_err());
    assert!(serde_json::from_str::<ControlCommand>(r#"{"cmd":"Trigger"}"#).is_err());
    assert!(serde_json::from_str::<ControlCommand>(r#"{"cmd":"Blackout"}"#).is_err());
    assert!(serde_json::from_str::<ControlCommand>(r#"{"random":"object"}"#).is_err());
    assert!(serde_json::from_str::<ControlCommand>("not json").is_err());
}

#[test]
fn test_status_handle_and_snapshot_serde() {
    let snapshot = StatusSnapshot {
        project: "Sunday Service".to_string(),
        live_index: Some(2),
        live_text: "Welcome".to_string(),
        next_text: "Opening Song".to_string(),
        blackout: false,
        timers: vec![("Service".to_string(), "10:00".to_string())],
    };

    let serialized = serde_json::to_string(&snapshot).expect("serialize snapshot");
    let deserialized: StatusSnapshot =
        serde_json::from_str(&serialized).expect("deserialize snapshot");
    assert_eq!(deserialized, snapshot);

    let handle = StatusHandle::new(snapshot.clone());
    assert_eq!(handle.get(), snapshot);

    let mut updated = snapshot;
    updated.live_index = Some(3);
    updated.blackout = true;
    handle.set(updated.clone());
    assert_eq!(handle.get(), updated);
}

#[test]
fn test_http_token_too_short_rejected() {
    let (tx, _rx) = mpsc::channel();
    let status = StatusHandle::default();

    let config = HttpConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        token: "short".to_string(),
    };
    let result = HttpServer::start(config, tx, status);
    assert!(matches!(result, Err(ControlError::TokenTooShort(_))));
}

#[test]
fn test_http_server_endpoints_and_lifecycle() {
    let (tx, rx) = mpsc::channel();
    let initial_snapshot = StatusSnapshot {
        project: "Youth Night".to_string(),
        live_index: Some(0),
        live_text: "Slide 1".to_string(),
        next_text: "Slide 2".to_string(),
        blackout: false,
        timers: vec![("Countdown".to_string(), "05:00".to_string())],
    };
    let status = StatusHandle::new(initial_snapshot.clone());

    let token = "valid_secret_token_12345".to_string();
    let config = HttpConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        token: token.clone(),
    };

    let mut server =
        HttpServer::start(config, tx, status.clone()).expect("server start should succeed");
    let addr = server.local_addr().expect("bound local address");

    // Helper for HTTP requests
    let send_req =
        |method: &str, path: &str, auth: Option<&str>, body: Option<&str>| -> (u16, String) {
            use std::io::{Read, Write};
            use std::net::TcpStream;

            let mut stream = TcpStream::connect(addr).expect("connect to server");
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();

            let body_str = body.unwrap_or("");
            let mut req_str = format!("{method} {path} HTTP/1.1\r\nHost: {addr}\r\n");
            if let Some(tok) = auth {
                req_str.push_str(&format!("Authorization: {tok}\r\n"));
            }
            if !body_str.is_empty() {
                req_str.push_str(&format!("Content-Length: {}\r\n", body_str.len()));
                req_str.push_str("Content-Type: application/json\r\n");
            }
            req_str.push_str("Connection: close\r\n\r\n");
            req_str.push_str(body_str);

            stream.write_all(req_str.as_bytes()).expect("write request");
            stream.flush().expect("flush request");

            let mut response_bytes = Vec::new();
            let _ = stream.read_to_end(&mut response_bytes);
            let response_str = String::from_utf8_lossy(&response_bytes).to_string();

            let status_code = response_str
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|c| c.parse::<u16>().ok())
                .unwrap_or(0);

            let body_part = response_str
                .split("\r\n\r\n")
                .nth(1)
                .unwrap_or("")
                .to_string();
            (status_code, body_part)
        };

    // 1. 401 without token
    let (code, _) = send_req("GET", "/status", None, None);
    assert_eq!(code, 401, "expected 401 without token");

    // 2. 401 with wrong token
    let (code, _) = send_req(
        "GET",
        "/status",
        Some("Bearer wrong_secret_token_abcdef"),
        None,
    );
    assert_eq!(code, 401, "expected 401 with wrong token");

    // 3. 200 GET /status with valid token
    let auth_header = format!("Bearer {token}");
    let (code, body) = send_req("GET", "/status", Some(&auth_header), None);
    assert_eq!(code, 200, "expected 200 on /status");
    let returned_snapshot: StatusSnapshot =
        serde_json::from_str(&body).expect("decode status snapshot");
    assert_eq!(returned_snapshot, initial_snapshot);

    // Update status and verify dynamic reflection
    let mut updated_snapshot = initial_snapshot;
    updated_snapshot.live_text = "Updated text".to_string();
    status.set(updated_snapshot.clone());

    let (code, body) = send_req("GET", "/status", Some(&auth_header), None);
    assert_eq!(code, 200);
    let returned_snapshot: StatusSnapshot =
        serde_json::from_str(&body).expect("decode updated status");
    assert_eq!(returned_snapshot, updated_snapshot);

    // 4. 202 POST /command with valid command
    let (code, _) = send_req(
        "POST",
        "/command",
        Some(&auth_header),
        Some(r#"{"cmd":"Next"}"#),
    );
    assert_eq!(code, 202, "expected 202 for valid command");
    let event = rx
        .recv_timeout(Duration::from_millis(500))
        .expect("command event received on channel");
    assert_eq!(
        event,
        ControlEvent {
            source: Source::Http,
            command: ControlCommand::Next,
        }
    );

    // 5. 400 malformed command JSON
    let (code, body) = send_req(
        "POST",
        "/command",
        Some(&auth_header),
        Some(r#"{"cmd":"Trigger"}"#),
    );
    assert_eq!(code, 400, "expected 400 for malformed command");
    assert!(!body.is_empty(), "expected error message body");

    // 6. 413/400 oversize body (> 4 KiB)
    let large_body = format!(r#"{{"cmd":"StageMessage","text":"{}"}}"#, "x".repeat(4097));
    let (code, _) = send_req("POST", "/command", Some(&auth_header), Some(&large_body));
    assert!(
        code == 413 || code == 400,
        "expected 413 or 400 for oversized payload, got {code}"
    );

    // 7. 404 for unknown routes
    let (code, _) = send_req("GET", "/unknown", Some(&auth_header), None);
    assert_eq!(code, 404, "expected 404 for unknown route");

    let (code, _) = send_req("POST", "/status", Some(&auth_header), None);
    assert_eq!(code, 404, "expected 404 for POST /status");

    // 8. Shutdown joins thread cleanly
    server.shutdown();
    // After shutdown, connection attempts should fail
    std::thread::sleep(Duration::from_millis(50));
    assert!(std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_err());
}

#[test]
fn test_http_server_drop_joins() {
    let (tx, _rx) = mpsc::channel();
    let status = StatusHandle::default();
    let token = "drop_test_token_12345".to_string();
    let config = HttpConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        token,
    };
    let addr = {
        let server = HttpServer::start(config, tx, status).expect("server start");
        server.local_addr().expect("local addr")
    };
    // server dropped here
    std::thread::sleep(Duration::from_millis(50));
    assert!(std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_err());
}

#[test]
fn test_osc_listener() {
    use rosc::encoder;
    use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
    use std::net::UdpSocket;

    let (tx, rx) = mpsc::channel();
    let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let mut listener = OscListener::start(bind_addr, tx).expect("start osc listener");
    let listener_addr = listener.local_addr();

    let client = UdpSocket::bind("127.0.0.1:0").expect("bind client socket");

    let send_packet = |packet: OscPacket| {
        let bytes = encoder::encode(&packet).expect("encode osc packet");
        client
            .send_to(&bytes, listener_addr)
            .expect("send osc packet");
    };

    // Test addresses
    let test_cases = vec![
        (
            OscMessage {
                addr: "/lumen/next".to_string(),
                args: vec![],
            },
            ControlCommand::Next,
        ),
        (
            OscMessage {
                addr: "/lumen/previous".to_string(),
                args: vec![],
            },
            ControlCommand::Previous,
        ),
        (
            OscMessage {
                addr: "/lumen/trigger".to_string(),
                args: vec![OscType::Int(12)],
            },
            ControlCommand::Trigger { index: 12 },
        ),
        (
            OscMessage {
                addr: "/lumen/blackout".to_string(),
                args: vec![OscType::Int(1)],
            },
            ControlCommand::Blackout { on: true },
        ),
        (
            OscMessage {
                addr: "/lumen/blackout".to_string(),
                args: vec![OscType::Int(0)],
            },
            ControlCommand::Blackout { on: false },
        ),
        (
            OscMessage {
                addr: "/lumen/clear/text".to_string(),
                args: vec![],
            },
            ControlCommand::ClearText,
        ),
        (
            OscMessage {
                addr: "/lumen/clear/all".to_string(),
                args: vec![],
            },
            ControlCommand::ClearAll,
        ),
        (
            OscMessage {
                addr: "/lumen/macro".to_string(),
                args: vec![OscType::String("LightingCue".to_string())],
            },
            ControlCommand::RunMacro {
                name: "LightingCue".to_string(),
            },
        ),
        (
            OscMessage {
                addr: "/lumen/timer/start".to_string(),
                args: vec![OscType::String("Speaker".to_string())],
            },
            ControlCommand::TimerStart {
                name: "Speaker".to_string(),
            },
        ),
        (
            OscMessage {
                addr: "/lumen/timer/pause".to_string(),
                args: vec![OscType::String("Speaker".to_string())],
            },
            ControlCommand::TimerPause {
                name: "Speaker".to_string(),
            },
        ),
        (
            OscMessage {
                addr: "/lumen/timer/reset".to_string(),
                args: vec![OscType::String("Speaker".to_string())],
            },
            ControlCommand::TimerReset {
                name: "Speaker".to_string(),
            },
        ),
        (
            OscMessage {
                addr: "/lumen/message".to_string(),
                args: vec![OscType::String("Wrap up in 2 mins".to_string())],
            },
            ControlCommand::StageMessage {
                text: "Wrap up in 2 mins".to_string(),
            },
        ),
    ];

    for (msg, expected_cmd) in test_cases {
        send_packet(OscPacket::Message(msg));
        let event = rx
            .recv_timeout(Duration::from_millis(500))
            .expect("receive osc event");
        assert_eq!(
            event,
            ControlEvent {
                source: Source::Osc,
                command: expected_cmd,
            }
        );
    }

    // Bundles flattened
    let bundle = OscBundle {
        timetag: OscTime {
            seconds: 0,
            fractional: 0,
        },
        content: vec![
            OscPacket::Message(OscMessage {
                addr: "/lumen/next".to_string(),
                args: vec![],
            }),
            OscPacket::Message(OscMessage {
                addr: "/lumen/clear/all".to_string(),
                args: vec![],
            }),
        ],
    };
    send_packet(OscPacket::Bundle(bundle));

    let ev1 = rx
        .recv_timeout(Duration::from_millis(500))
        .expect("bundle msg 1");
    let ev2 = rx
        .recv_timeout(Duration::from_millis(500))
        .expect("bundle msg 2");
    assert_eq!(ev1.command, ControlCommand::Next);
    assert_eq!(ev2.command, ControlCommand::ClearAll);

    // Test malformed / unknown / oversized UDP packets increment dropped counter
    let initial_dropped = listener.dropped();

    // 1. Unknown address
    send_packet(OscPacket::Message(OscMessage {
        addr: "/lumen/nonexistent".to_string(),
        args: vec![],
    }));

    // 2. Malformed arguments: /lumen/trigger with string instead of int
    send_packet(OscPacket::Message(OscMessage {
        addr: "/lumen/trigger".to_string(),
        args: vec![OscType::String("not an int".to_string())],
    }));

    // 3. Invalid blackout argument: /lumen/blackout with 2
    send_packet(OscPacket::Message(OscMessage {
        addr: "/lumen/blackout".to_string(),
        args: vec![OscType::Int(2)],
    }));

    // 4. StageMessage > 500 chars
    send_packet(OscPacket::Message(OscMessage {
        addr: "/lumen/message".to_string(),
        args: vec![OscType::String("x".repeat(501))],
    }));

    // 5. Completely invalid non-OSC packet
    client
        .send_to(b"not valid osc data", listener_addr)
        .expect("send raw bytes");

    // Give background thread a moment to process
    std::thread::sleep(Duration::from_millis(100));

    let final_dropped = listener.dropped();
    assert_eq!(
        final_dropped - initial_dropped,
        5,
        "5 malformed packets should increment dropped count by 5"
    );

    // Shutdown listener
    listener.shutdown();
}

#[test]
fn test_midi_mapping_logic() {
    let mappings = vec![
        MidiMapping {
            channel: 0,
            note: 60,
            command: ControlCommand::Next,
        },
        MidiMapping {
            channel: 0,
            note: 61,
            command: ControlCommand::Previous,
        },
        MidiMapping {
            channel: 15,
            note: 120,
            command: ControlCommand::Blackout { on: true },
        },
    ];

    // Channel 0 Note On (0x90), Note 60, Velocity 100 -> Next
    let note_on_ch0_60 = [0x90, 60, 100];
    assert_eq!(
        map_midi_message(&note_on_ch0_60, &mappings),
        Some(ControlCommand::Next)
    );

    // Channel 0 Note On, Note 61, Velocity 1 -> Previous
    let note_on_ch0_61 = [0x90, 61, 1];
    assert_eq!(
        map_midi_message(&note_on_ch0_61, &mappings),
        Some(ControlCommand::Previous)
    );

    // Channel 15 Note On, Note 120, Velocity 127 -> Blackout { on: true }
    let note_on_ch15 = [0x9F, 120, 127];
    assert_eq!(
        map_midi_message(&note_on_ch15, &mappings),
        Some(ControlCommand::Blackout { on: true })
    );

    // Velocity 0 (treated as Note Off / non-triggering) -> None
    let note_on_vel0 = [0x90, 60, 0];
    assert_eq!(map_midi_message(&note_on_vel0, &mappings), None);

    // Note Off (0x80) -> None
    let note_off = [0x80, 60, 100];
    assert_eq!(map_midi_message(&note_off, &mappings), None);

    // Channel mismatch: Channel 1 (0x91), Note 60, Velocity 100 -> None
    let ch_mismatch = [0x91, 60, 100];
    assert_eq!(map_midi_message(&ch_mismatch, &mappings), None);

    // Note mismatch: Channel 0, Note 62, Velocity 100 -> None
    let note_mismatch = [0x90, 62, 100];
    assert_eq!(map_midi_message(&note_mismatch, &mappings), None);

    // Incomplete or invalid MIDI frames -> None
    assert_eq!(map_midi_message(&[], &mappings), None);
    assert_eq!(map_midi_message(&[0x90], &mappings), None);
    assert_eq!(map_midi_message(&[0x90, 60], &mappings), None);
}

#[test]
fn test_midi_ports_query() {
    // Calling midi_ports() must return an Ok Result on the current platform
    let ports = midi_ports();
    assert!(ports.is_ok(), "midi_ports() should succeed: {:?}", ports);
}

//! Loopback-only HTTP tests exercise the production adapter without Google access.
use crate::drive_api::{DriveApi, GoogleDrive};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

type Check = Box<dyn Fn(&str, &[u8]) + Send>;
struct Step {
    method: &'static str,
    path: &'static str,
    status: u16,
    headers: String,
    body: Vec<u8>,
    check: Check,
    truncate_response: bool,
}
impl Step {
    fn json(method: &'static str, path: &'static str, body: Value) -> Self {
        Self {
            method,
            path,
            status: 200,
            headers: "Content-Type: application/json\r\n".into(),
            body: serde_json::to_vec(&body).unwrap(),
            check: Box::new(|_, _| {}),
            truncate_response: false,
        }
    }
}
struct Wire {
    origin: String,
    worker: thread::JoinHandle<()>,
}
impl Wire {
    fn new(steps: impl FnOnce(&str) -> Vec<Step>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let steps = steps(&origin);
        let worker = thread::spawn(move || {
            for step in steps {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                let mut all = Vec::new();
                let header_end = loop {
                    let mut chunk = [0u8; 4096];
                    let n = socket.read(&mut chunk).unwrap();
                    assert!(n > 0, "request ended before headers");
                    all.extend_from_slice(&chunk[..n]);
                    if let Some(at) = all.windows(4).position(|b| b == b"\r\n\r\n") {
                        break at + 4;
                    }
                    assert!(all.len() < 64 * 1024);
                };
                let header = String::from_utf8(all[..header_end].to_vec()).unwrap();
                let line = header.lines().next().unwrap();
                assert!(
                    line.starts_with(&format!("{} {}", step.method, step.path)),
                    "unexpected request: {line}"
                );
                assert!(header
                    .to_lowercase()
                    .contains("authorization: bearer disposable-test-token"));
                let size = header
                    .lines()
                    .find_map(|line| {
                        line.split_once(':')
                            .filter(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                            .map(|(_, v)| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                assert!(size < 16 * 1024 * 1024);
                while all.len() - header_end < size {
                    let mut chunk = [0u8; 65536];
                    let n = socket.read(&mut chunk).unwrap();
                    assert!(n > 0, "request ended before body");
                    all.extend_from_slice(&chunk[..n]);
                }
                (step.check)(&header, &all[header_end..header_end + size]);
                let header = format!(
                    "HTTP/1.1 {} Test\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n",
                    step.status,
                    step.body.len(),
                    step.headers
                );
                socket.write_all(header.as_bytes()).unwrap();
                let body = if step.truncate_response {
                    &step.body[..step.body.len() / 2]
                } else {
                    &step.body
                };
                socket.write_all(body).unwrap();
            }
        });
        Self { origin, worker }
    }
    fn api(&self) -> GoogleDrive {
        GoogleDrive::loopback_test(&self.origin, |_| Ok(()))
    }
    fn finish(self) {
        self.worker.join().unwrap();
    }
}
fn remote(name: &str, size: usize, version: u8) -> Value {
    json!({"id":"same-file","name":name,"mimeType":"text/markdown","parents":["project-folder"],"size":size.to_string(),"version":version.to_string(),"trashed":false})
}

#[test]
fn drive_wire_interrupted_upload_receipt_is_retryable_and_same_id_can_be_verified() {
    let bytes = b"# ContextDock interrupted response marker";
    let wire = Wire::new(|origin| {
        let mut init = Step::json("POST", "/upload/drive/v3/files?", json!({}));
        init.headers.push_str(&format!(
            "Location: {origin}/upload/drive/v3/files?session=interrupted\r\n"
        ));
        let mut upload = Step::json(
            "PUT",
            "/upload/drive/v3/files?session=interrupted",
            remote("ContextDock_Transfer_v1.md", bytes.len(), 1),
        );
        upload.truncate_response = true;
        upload.check = Box::new(move |_, body| assert_eq!(body, bytes));
        let get = Step::json(
            "GET",
            "/drive/v3/files/same-file?",
            remote("ContextDock_Transfer_v1.md", bytes.len(), 1),
        );
        let mut download = Step::json("GET", "/drive/v3/files/same-file?", json!({}));
        download.body = bytes.to_vec();
        vec![init, upload, get, download]
    });
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("ContextDock_Transfer_v1.md");
    std::fs::write(&path, bytes).unwrap();
    let api = wire.api();
    let failure = api
        .upload(
            "same-file",
            json!({"name":"ContextDock_Transfer_v1.md","mimeType":"text/markdown"}),
            &path,
            true,
            None,
        )
        .unwrap_err();
    assert!(failure.retry);
    assert!(!failure.auth);
    assert_eq!(api.get("same-file").unwrap().id, "same-file");
    assert_eq!(
        api.download_hash("same-file", bytes.len() as i64).unwrap(),
        format!("{:x}", Sha256::digest(bytes))
    );
    wire.finish();
}

#[test]
fn drive_wire_resumable_create_and_patch_keep_id_and_verify_exact_bytes() {
    let one = b"# First marker".to_vec();
    let two = crate::drive_publish_tests::sample_docx("Second Word marker");
    let wire = Wire::new(|origin| {
        let mut steps = Vec::new();
        for (index, bytes) in [&one, &two].into_iter().enumerate() {
            let name = if index == 0 {
                "ContextDock_Pilot_v1.md"
            } else {
                "ContextDock_Pilot_v2.docx"
            };
            let mut init = Step::json(
                if index == 0 { "POST" } else { "PATCH" },
                if index == 0 {
                    "/upload/drive/v3/files?"
                } else {
                    "/upload/drive/v3/files/same-file?"
                },
                json!({}),
            );
            init.headers.push_str(&format!(
                "Location: {origin}/upload/drive/v3/files?session={index}\r\n"
            ));
            init.check = Box::new(move |headers, body| {
                let metadata: Value = serde_json::from_slice(body).unwrap();
                assert_eq!(metadata["name"], name);
                if index == 0 {
                    assert_eq!(metadata["id"], "same-file");
                } else {
                    assert!(headers.to_lowercase().contains("if-match: receipt-1"));
                    assert!(metadata.get("id").is_none());
                }
            });
            steps.push(init);
            let mut put = Step::json(
                "PUT",
                "/upload/drive/v3/files?session=",
                remote(name, bytes.len(), index as u8 + 1),
            );
            let expected = bytes.to_vec();
            put.check = Box::new(move |_, body| assert_eq!(body, expected));
            steps.push(put);
            let mut get = Step::json("GET", "/drive/v3/files/same-file?alt=media", Value::Null);
            get.body = bytes.to_vec();
            steps.push(get);
        }
        steps
    });
    let api = wire.api();
    let t = tempfile::tempdir().unwrap();
    for (index, bytes) in [&one, &two].into_iter().enumerate() {
        let name = if index == 0 {
            "ContextDock_Pilot_v1.md"
        } else {
            "ContextDock_Pilot_v2.docx"
        };
        let path = t.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        let file = api
            .upload(
                "same-file",
                json!({"name":name,"mimeType":"application/octet-stream"}),
                &path,
                index == 0,
                (index == 1).then_some("receipt-1"),
            )
            .unwrap();
        assert_eq!(file.id, "same-file");
        assert_eq!(
            api.download_hash(&file.id, bytes.len() as i64).unwrap(),
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    wire.finish();
}

#[test]
fn drive_wire_provider_failures_are_classified_and_bounded() {
    let wire = Wire::new(|_| {
        [429, 401, 412, 404]
            .into_iter()
            .map(|status| {
                let mut step = Step::json(
                    "GET",
                    "/drive/v3/files/same-file?",
                    json!({"error":{"message":"must not enter app diagnostics"}}),
                );
                step.status = status;
                step.headers.push_str("Retry-After: 27\r\n");
                step
            })
            .collect()
    });
    let api = wire.api();
    let rate = api.get("same-file").unwrap_err();
    assert!(rate.retry);
    assert_eq!(rate.delay, 27);
    let auth = api.get("same-file").unwrap_err();
    assert!(auth.auth);
    assert!(!auth.retry);
    let conflict = api.get("same-file").unwrap_err();
    assert!(!conflict.retry);
    assert!(!conflict.message.contains("must not"));
    assert!(api.get("same-file").unwrap_err().missing);
    wire.finish();
}

#[test]
fn drive_wire_never_follows_an_untrusted_session_location() {
    let wire = Wire::new(|_| {
        let mut step = Step::json("POST", "/upload/drive/v3/files?", json!({}));
        step.headers
            .push_str("Location: https://example.invalid/upload/drive/v3/files?secret=session\r\n");
        vec![step]
    });
    let t = tempfile::tempdir().unwrap();
    let path = t.path().join("ContextDock_Pilot_v1.md");
    std::fs::write(&path, b"harmless").unwrap();
    let failure = wire
        .api()
        .upload(
            "same-file",
            json!({"name":"ContextDock_Pilot_v1.md"}),
            &path,
            true,
            None,
        )
        .unwrap_err();
    assert!(!failure.retry);
    assert!(!failure.message.contains("secret"));
    wire.finish();
}

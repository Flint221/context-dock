use crate::drive_auth::*;
use std::{
    cell::RefCell,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::atomic::AtomicBool,
    time::Duration,
};

fn config() -> ClientConfig {
    ClientConfig::parse(br#"{"installed":{"client_id":"123-contextdock.apps.googleusercontent.com","client_secret":"sample-client-secret"}}"#).unwrap()
}
fn account() -> Account {
    Account {
        id: "sample-account-id".into(),
        email: "sample@example.test".into(),
        name: "Sample".into(),
    }
}
fn grant(refresh: Option<&str>, scope: Option<&str>) -> Grant {
    Grant {
        access_token: "sample-access-token".into(),
        token_type: "Bearer".into(),
        expires_in: 3600,
        refresh_token: refresh.map(str::to_owned),
        scope: scope.map(str::to_owned),
    }
}
#[derive(Default)]
struct MemoryVault {
    value: RefCell<Option<String>>,
    fail_save: bool,
}
impl Vault for MemoryVault {
    fn load(&self) -> Result<Option<Stored>> {
        Ok(self
            .value
            .borrow()
            .as_ref()
            .map(|s| serde_json::from_str(s).unwrap()))
    }
    fn save(&self, v: &Stored) -> Result<()> {
        if self.fail_save {
            return Err("vault unavailable".into());
        }
        *self.value.borrow_mut() = Some(serde_json::to_string(v).unwrap());
        Ok(())
    }
    fn clear(&self) -> Result<()> {
        *self.value.borrow_mut() = None;
        Ok(())
    }
}
struct FakeHttp {
    response: RefCell<Option<Result<Grant>>>,
    forms: RefCell<Vec<Vec<(String, String)>>>,
    who: Result<Account>,
    revoke_error: Option<String>,
}
impl FakeHttp {
    fn new(response: Result<Grant>) -> Self {
        Self {
            response: RefCell::new(Some(response)),
            forms: RefCell::default(),
            who: Ok(account()),
            revoke_error: None,
        }
    }
}
impl OAuthHttp for FakeHttp {
    fn token(&self, form: &[(&str, &str)]) -> Result<Grant> {
        self.forms.borrow_mut().push(
            form.iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        );
        self.response
            .borrow_mut()
            .take()
            .expect("unexpected token request")
    }
    fn account(&self, _: &str) -> Result<Account> {
        self.who.clone()
    }
    fn revoke(&self, _: &str) -> Result<()> {
        self.revoke_error.clone().map_or(Ok(()), Err)
    }
}
fn expired_vault() -> MemoryVault {
    let vault = MemoryVault::default();
    vault
        .save(&Stored {
            config: config(),
            session: Some(Session {
                access_token: "old-access".into(),
                refresh_token: "old-refresh".into(),
                expires_at: 1,
                account: account(),
            }),
        })
        .unwrap();
    vault
}
fn state(flow: &Flow) -> String {
    reqwest::Url::parse(&flow.url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "state")
        .unwrap()
        .1
        .to_string()
}

#[test]
fn drive_pkce_matches_rfc7636_and_requests_only_file_scope() {
    assert_eq!(
        challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
    );
    let a = Flow::new(&config(), 12345);
    let b = Flow::new(&config(), 12345);
    assert_ne!(state(&a), state(&b));
    let u = reqwest::Url::parse(&a.url).unwrap();
    let q = u.query_pairs().collect::<std::collections::HashMap<_, _>>();
    assert_eq!(q["scope"], SCOPE);
    assert_eq!(q["access_type"], "offline");
    assert_eq!(q["code_challenge_method"], "S256");
    assert_eq!(q["redirect_uri"], "http://127.0.0.1:12345/callback");
    assert!(!a.url.contains("client_secret"));
}
#[test]
fn drive_client_import_rejects_web_clients_and_untrusted_endpoints() {
    assert!(ClientConfig::parse(br#"{"web":{"client_id":"x"}}"#).is_err());
    assert!(ClientConfig::parse(
        br#"{"installed":{"client_id":"attacker.example","client_secret":"x"}}"#
    )
    .is_err());
    assert!(ClientConfig::parse(&vec![b'x'; 65_537]).is_err());
    let c = ClientConfig::parse(br#"{"installed":{"client_id":"sample.apps.googleusercontent.com","client_secret":"x","auth_uri":"https://attacker.test/"}}"#).unwrap();
    assert!(Flow::new(&c, 1)
        .url
        .starts_with("https://accounts.google.com/"));
}
#[test]
fn drive_callback_validates_state_path_method_and_duplicate_parameters() {
    let flow = Flow::new(&config(), 1);
    let state = state(&flow);
    assert_eq!(
        &*flow
            .callback(&format!(
                "GET /callback?state={state}&code=sample%2Bcode HTTP/1.1\r\n\r\n"
            ))
            .unwrap(),
        "sample+code"
    );
    for request in [
        "GET /callback?state=wrong&code=secret HTTP/1.1".into(),
        format!("GET /other?state={state}&code=secret HTTP/1.1"),
        format!("POST /callback?state={state}&code=secret HTTP/1.1"),
        format!("GET /callback?state={state}&state={state}&code=secret HTTP/1.1"),
        format!("GET /callback?state={state}&code=x&code=y HTTP/1.1"),
    ] {
        let error = flow.callback(&request).unwrap_err();
        assert!(!error.contains("secret"));
    }
    assert!(flow
        .callback(&format!(
            "GET /callback?state={state}&error=access_denied HTTP/1.1"
        ))
        .unwrap_err()
        .contains("declined"));
}
#[test]
fn drive_loopback_receives_a_real_http_callback() {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let flow = Flow::new(&config(), addr.port());
    let state = state(&flow);
    let worker = std::thread::spawn(move || {
        let mut socket = TcpStream::connect(addr).unwrap();
        write!(
            socket,
            "GET /callback?state={state}&code=sample-code HTTP/1.1\r\nHost: {addr}\r\n\r\n"
        )
        .unwrap();
        let mut result = String::new();
        socket.read_to_string(&mut result).unwrap();
        result
    });
    assert_eq!(
        &*flow
            .wait(listener, &AtomicBool::new(false), Duration::from_secs(3))
            .unwrap(),
        "sample-code"
    );
    let response = worker.join().unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(!response.contains("sample-code"));
}
#[test]
fn drive_loopback_cancel_and_timeout_are_bounded() {
    for cancel in [false, true] {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let flow = Flow::new(&config(), listener.local_addr().unwrap().port());
        let error = flow
            .wait(
                listener,
                &AtomicBool::new(cancel),
                Duration::from_millis(10),
            )
            .unwrap_err();
        assert!(error.contains(if cancel { "cancelled" } else { "timed out" }));
    }
}
#[test]
fn drive_finish_uses_pkce_and_verifies_account_before_accepting_session() {
    let c = config();
    let flow = Flow::new(&c, 1);
    let http = FakeHttp::new(Ok(grant(Some("refresh"), Some(SCOPE))));
    let s = finish(&c, &flow, "sample-code", &http, 100).unwrap();
    assert_eq!(s.account, account());
    assert_eq!(s.expires_at, 3700);
    let forms = http.forms.borrow();
    let form = &forms[0];
    assert!(form.iter().any(|(k, v)| k == "code_verifier"
        && challenge(v)
            == reqwest::Url::parse(&flow.url)
                .unwrap()
                .query_pairs()
                .find(|(k, _)| k == "code_challenge")
                .unwrap()
                .1));
    let mut denied = FakeHttp::new(Ok(grant(Some("refresh"), Some(SCOPE))));
    denied.who = Err("Drive disabled".into());
    assert_eq!(
        finish(&c, &flow, "sample-code", &denied, 100)
            .err()
            .unwrap(),
        "Drive disabled"
    );
}
#[test]
fn drive_finish_rejects_missing_scope_offline_access_and_wrong_token_type() {
    let c = config();
    let flow = Flow::new(&c, 1);
    for g in [
        grant(None, Some(SCOPE)),
        grant(Some("r"), None),
        grant(Some("r"), Some("openid")),
    ] {
        assert!(finish(&c, &flow, "code", &FakeHttp::new(Ok(g)), 1).is_err());
    }
    let mut g = grant(Some("r"), Some(SCOPE));
    g.token_type = "Basic".into();
    assert!(finish(&c, &flow, "code", &FakeHttp::new(Ok(g)), 1).is_err());
}
#[test]
fn drive_refresh_persists_rotated_tokens_and_keeps_account_binding() {
    let vault = expired_vault();
    let http = FakeHttp::new(Ok(grant(Some("rotated"), None)));
    assert_eq!(&*access(&vault, &http, 100).unwrap(), "sample-access-token");
    let stored = vault.load().unwrap().unwrap();
    let s = stored.session.unwrap();
    assert_eq!(s.refresh_token, "rotated");
    assert_eq!(s.account, account());
    assert!(http.forms.borrow()[0]
        .iter()
        .any(|(k, v)| k == "refresh_token" && v == "old-refresh"));
    let no_request = FakeHttp::new(Err("must not request".into()));
    assert_eq!(
        &*access(&vault, &no_request, 101).unwrap(),
        "sample-access-token"
    );
    assert!(no_request.forms.borrow().is_empty());
}
#[test]
fn drive_refresh_preserves_refresh_token_when_google_omits_it() {
    let vault = expired_vault();
    access(&vault, &FakeHttp::new(Ok(grant(None, None))), 100).unwrap();
    assert_eq!(
        vault
            .load()
            .unwrap()
            .unwrap()
            .session
            .unwrap()
            .refresh_token,
        "old-refresh"
    );
}
#[test]
fn drive_refresh_failure_preserves_credentials_and_vault_failure_blocks_new_token() {
    let mut vault = expired_vault();
    let before = vault.value.borrow().clone();
    assert!(access(&vault, &FakeHttp::new(Err("offline".into())), 100).is_err());
    assert_eq!(*vault.value.borrow(), before);
    vault.fail_save = true;
    assert!(access(
        &vault,
        &FakeHttp::new(Ok(grant(Some("rotated"), None))),
        100
    )
    .is_err());
    assert_eq!(*vault.value.borrow(), before);
}

#[test]
fn drive_disconnect_clears_tokens_before_remote_revocation_and_retains_client() {
    let vault = expired_vault();
    let mut http = FakeHttp::new(Err("no token request expected".into()));
    http.revoke_error = Some("revocation offline".into());
    assert_eq!(disconnect(&vault, &http).unwrap_err(), "revocation offline");
    let saved = vault.load().unwrap().unwrap();
    assert!(saved.session.is_none());
    assert_eq!(saved.config.client_id, config().client_id);
    assert!(access(&vault, &http, 100).is_err());
}

#[test]
fn drive_disconnect_vault_failure_keeps_existing_connection_for_recovery() {
    let mut vault = expired_vault();
    let before = vault.value.borrow().clone();
    vault.fail_save = true;
    assert!(disconnect(&vault, &FakeHttp::new(Err("unused".into()))).is_err());
    assert_eq!(*vault.value.borrow(), before);
}

#[test]
fn drive_configure_does_not_replace_a_connected_account_or_accept_invalid_json() {
    let vault = expired_vault();
    let before = vault.value.borrow().clone();
    assert!(configure(&vault, b"{}").unwrap_err().contains("Disconnect"));
    assert_eq!(*vault.value.borrow(), before);
    vault.clear().unwrap();
    assert!(configure(&vault, b"{}").is_err());
    assert!(vault.load().unwrap().is_none());
    let encoded = serde_json::json!({"installed":config()}).to_string();
    configure(&vault, encoded.as_bytes()).unwrap();
    assert!(vault.load().unwrap().unwrap().session.is_none());
}

#[test]
fn drive_native_picker_requests_one_folder_and_validates_returned_identity() {
    let flow = Flow::pick_folder(&config(), 12345);
    let parsed = reqwest::Url::parse(&flow.url).unwrap();
    let pairs = parsed
        .query_pairs()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(pairs["trigger_onepick"], "true");
    assert_eq!(pairs["scope"], SCOPE);
    assert_eq!(pairs["allow_folder_selection"], "true");
    assert_eq!(pairs["mimetypes"], "application/vnd.google-apps.folder");
    let state = state(&flow);
    let callback = flow
        .callback(&format!(
            "GET /callback?state={state}&code=secret-code&picked_file_ids=folder-one HTTP/1.1"
        ))
        .unwrap();
    assert_eq!(callback.picked, ["folder-one"]);
    assert!(!format!("{callback:?}").contains("secret-code"));
    for ids in ["", "one,two", "../escape"] {
        assert!(flow
            .callback(&format!(
                "GET /callback?state={state}&code=secret-code&picked_file_ids={ids} HTTP/1.1"
            ))
            .is_err());
    }
}

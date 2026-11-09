//! Desktop OAuth: system browser, loopback callback, PKCE and a native-only vault.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use zeroize::{Zeroize, Zeroizing};

pub type Result<T> = std::result::Result<T, String>;
pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const VAULT: &str = "ContextDock.GoogleDrive";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DECLINED: &str = "Google sign-in was declined. No connection was saved.";

#[derive(Serialize, Deserialize)]
pub struct ClientConfig {
    pub client_id: String,
    pub client_secret: String,
}
impl Drop for ClientConfig {
    fn drop(&mut self) {
        self.client_secret.zeroize();
    }
}
impl ClientConfig {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        #[derive(Deserialize)]
        struct Download {
            installed: ClientConfig,
        }
        if bytes.len() > 65_536 {
            return Err("Choose the downloaded Desktop OAuth client JSON (under 64 KB).".into());
        }
        let config = serde_json::from_slice::<Download>(bytes)
            .map_err(|_| "Choose a Google OAuth client JSON with application type Desktop app.")?
            .installed;
        let prefix = config.client_id.strip_suffix(".apps.googleusercontent.com");
        if !prefix.is_some_and(|p| {
            !p.is_empty()
                && p.len() <= 256
                && p.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        }) || config.client_secret.is_empty()
            || config.client_secret.len() > 512
            || !config.client_secret.bytes().all(|c| (33..127).contains(&c))
        {
            return Err("The Desktop OAuth client configuration is invalid.".into());
        }
        // Never use endpoints supplied by an imported file.
        Ok(config)
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub email: String,
    pub name: String,
}
#[derive(Serialize, Deserialize)]
pub struct Session {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub account: Account,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.access_token.zeroize();
        self.refresh_token.zeroize();
    }
}
#[derive(Serialize, Deserialize)]
pub struct Stored {
    pub config: ClientConfig,
    pub session: Option<Session>,
}
pub trait Vault {
    fn load(&self) -> Result<Option<Stored>>;
    fn save(&self, value: &Stored) -> Result<()>;
    fn clear(&self) -> Result<()>;
}
pub struct NativeVault;
impl Vault for NativeVault {
    fn load(&self) -> Result<Option<Stored>> {
        crate::cloud_credentials::read_named(VAULT)?
            .map(|s| {
                serde_json::from_str(&s).map_err(|_| {
                    "The saved Drive connection is invalid. Import the client again.".into()
                })
            })
            .transpose()
    }
    fn save(&self, value: &Stored) -> Result<()> {
        let encoded = Zeroizing::new(
            serde_json::to_string(value).map_err(|_| "Could not encode the Drive connection.")?,
        );
        crate::cloud_credentials::save_named(VAULT, &encoded)
    }
    fn clear(&self) -> Result<()> {
        crate::cloud_credentials::delete_named(VAULT)
    }
}

pub fn configure(vault: &dyn Vault, bytes: &[u8]) -> Result<()> {
    if vault.load()?.is_some_and(|s| s.session.is_some()) {
        return Err("Disconnect Google Drive before replacing its OAuth client.".into());
    }
    let config = ClientConfig::parse(bytes)?;
    vault.save(&Stored {
        config,
        session: None,
    })
}

pub fn disconnect(vault: &dyn Vault, http: &dyn OAuthHttp) -> Result<()> {
    if let Some(mut saved) = vault.load()? {
        let session = saved.session.take();
        vault.save(&saved)?;
        // Clear locally before contacting Google; offline revocation cannot
        // leave a background worker authorized to continue publishing.
        if let Some(session) = session {
            http.revoke(&session.refresh_token)?;
        }
    }
    Ok(())
}

pub struct Flow {
    verifier: Zeroizing<String>,
    state: String,
    pub redirect: String,
    pub url: String,
}
impl Flow {
    pub fn new(config: &ClientConfig, port: u16) -> Self {
        let verifier = Zeroizing::new(format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        ));
        let state = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let redirect = format!("http://127.0.0.1:{port}/callback");
        let mut url = reqwest::Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
            .expect("fixed Google URL");
        url.query_pairs_mut().extend_pairs([
            ("client_id", config.client_id.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("response_type", "code"),
            ("scope", SCOPE),
            ("access_type", "offline"),
            ("prompt", "consent select_account"),
            ("state", state.as_str()),
            ("code_challenge_method", "S256"),
            ("code_challenge", &challenge(&verifier)),
        ]);
        Self {
            verifier,
            state,
            redirect,
            url: url.into(),
        }
    }
    pub fn callback(&self, request: &str) -> Result<Zeroizing<String>> {
        let mut parts = request
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        if parts.next() != Some("GET") {
            return Err("Invalid Google sign-in callback.".into());
        }
        let path = parts.next().unwrap_or_default();
        if !path.starts_with("/callback?") || path.contains('#') {
            return Err("Invalid Google sign-in callback.".into());
        }
        let url = reqwest::Url::parse(&format!("http://127.0.0.1{path}"))
            .map_err(|_| "Invalid Google sign-in callback.")?;
        let pairs = url.query_pairs().collect::<Vec<_>>();
        let state = pairs
            .iter()
            .filter(|(k, _)| k == "state")
            .collect::<Vec<_>>();
        if state.len() != 1 || state[0].1 != self.state {
            return Err("The Google sign-in state did not match. Try connecting again.".into());
        }
        if pairs.iter().any(|(k, _)| k == "error") {
            return Err(DECLINED.into());
        }
        let codes = pairs
            .iter()
            .filter(|(k, _)| k == "code")
            .collect::<Vec<_>>();
        if codes.len() != 1 || codes[0].1.is_empty() || codes[0].1.len() > 4096 {
            return Err("Google did not return a valid authorization code.".into());
        }
        Ok(Zeroizing::new(codes[0].1.to_string()))
    }
    pub fn wait(
        &self,
        listener: TcpListener,
        cancel: &AtomicBool,
        timeout: Duration,
    ) -> Result<Zeroizing<String>> {
        listener
            .set_nonblocking(true)
            .map_err(|_| "Could not listen for Google sign-in.")?;
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline && !cancel.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, peer)) => {
                    if !peer.ip().is_loopback() {
                        continue;
                    }
                    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
                    stream.set_write_timeout(Some(Duration::from_secs(2))).ok();
                    let mut bytes = Zeroizing::new(Vec::new());
                    let mut block = [0u8; 1024];
                    while bytes.len() < 8192 {
                        let count = match stream.read(&mut block) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => n,
                        };
                        bytes.extend_from_slice(&block[..count]);
                        if bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    let result = self.callback(&String::from_utf8_lossy(&bytes));
                    let (status, body) = if result.is_ok() {
                        ("200 OK", "Authorization received. Return to ContextDock to confirm the connection.")
                    } else {
                        (
                            "400 Bad Request",
                            "Sign-in was not accepted. Return to ContextDock and try again.",
                        )
                    };
                    let _ = write!(stream, "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len());
                    // Ignore unrelated/stale callbacks without consuming the active flow.
                    if result.is_ok() || result.as_ref().err().is_some_and(|e| e == DECLINED) {
                        return result;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100))
                }
                Err(_) => return Err("Could not receive the Google sign-in callback.".into()),
            }
        }
        Err(if cancel.load(Ordering::SeqCst) {
            "Google sign-in cancelled."
        } else {
            "Google sign-in timed out. Try connecting again."
        }
        .into())
    }
}
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[derive(Deserialize)]
pub struct Grant {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}
impl Drop for Grant {
    fn drop(&mut self) {
        self.access_token.zeroize();
        if let Some(token) = &mut self.refresh_token {
            token.zeroize();
        }
    }
}
pub trait OAuthHttp {
    fn token(&self, form: &[(&str, &str)]) -> Result<Grant>;
    fn account(&self, access: &str) -> Result<Account>;
    fn revoke(&self, refresh: &str) -> Result<()>;
}
pub struct GoogleHttp(pub reqwest::blocking::Client);
impl GoogleHttp {
    pub fn new() -> Result<Self> {
        reqwest::blocking::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(40))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map(Self)
            .map_err(|_| "Could not initialize the Google connection.".into())
    }
}
impl OAuthHttp for GoogleHttp {
    fn token(&self, form: &[(&str, &str)]) -> Result<Grant> {
        let response = self
            .0
            .post(TOKEN_URL)
            .form(form)
            .send()
            .map_err(|_| "Google could not be reached. Check your connection and retry.")?;
        if !response.status().is_success() {
            // Never include response bodies, authorization codes or tokens in errors.
            return Err(if response.status().is_client_error() {
                "Google authorization was rejected. Check the Desktop OAuth client and connect again."
            } else { "Google authorization is temporarily unavailable. Retry shortly." }.into());
        }
        response
            .json()
            .map_err(|_| "Google returned an invalid token response.".into())
    }
    fn account(&self, access: &str) -> Result<Account> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct User {
            permission_id: String,
            email_address: String,
            display_name: String,
        }
        #[derive(Deserialize)]
        struct About {
            user: User,
        }
        let response = self
            .0
            .get("https://www.googleapis.com/drive/v3/about")
            .query(&[("fields", "user(permissionId,emailAddress,displayName)")])
            .bearer_auth(access)
            .send()
            .map_err(|_| "Google Drive could not be reached. Check your connection and retry.")?;
        if !response.status().is_success() {
            return Err("Google Drive access could not be verified. Enable the Drive API, grant file access, and connect again.".into());
        }
        let about: About = response
            .json()
            .map_err(|_| "Google Drive returned an invalid account response.")?;
        if about.user.permission_id.is_empty() || about.user.email_address.is_empty() {
            return Err("Google Drive did not identify the connected account.".into());
        }
        Ok(Account {
            id: about.user.permission_id,
            email: about.user.email_address,
            name: about.user.display_name,
        })
    }
    fn revoke(&self, refresh: &str) -> Result<()> {
        let response = self.0.post("https://oauth2.googleapis.com/revoke").form(&[("token", refresh)])
            .send().map_err(|_| "Saved credentials were removed, but Google revocation could not be confirmed. You can remove access in your Google account.")?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err("Saved credentials were removed, but Google revocation could not be confirmed. You can remove access in your Google account.".into())
        }
    }
}
fn session(
    grant: &Grant,
    refresh: Option<&str>,
    account: Account,
    now: i64,
    initial: bool,
) -> Result<Session> {
    let refresh = grant
        .refresh_token
        .as_deref()
        .filter(|s| !s.is_empty())
        .or(refresh);
    if grant.access_token.is_empty()
        || !grant.token_type.eq_ignore_ascii_case("bearer")
        || !(1..=86_400).contains(&grant.expires_in)
        || refresh.is_none()
        || (initial && grant.scope.is_none())
        || grant
            .scope
            .as_ref()
            .is_some_and(|s| !s.split_whitespace().any(|scope| scope == SCOPE))
    {
        return Err("Google did not grant offline Drive file access. Connect again and allow the requested file permission.".into());
    }
    Ok(Session {
        access_token: grant.access_token.clone(),
        refresh_token: refresh.unwrap().into(),
        expires_at: now + grant.expires_in,
        account,
    })
}
pub fn finish(
    config: &ClientConfig,
    flow: &Flow,
    code: &str,
    http: &dyn OAuthHttp,
    now: i64,
) -> Result<Session> {
    let grant = http.token(&[
        ("client_id", &config.client_id),
        ("client_secret", &config.client_secret),
        ("code", code),
        ("code_verifier", &flow.verifier),
        ("redirect_uri", &flow.redirect),
        ("grant_type", "authorization_code"),
    ])?;
    let account = http.account(&grant.access_token)?;
    session(&grant, None, account, now, true)
}
pub fn access(vault: &dyn Vault, http: &dyn OAuthHttp, now: i64) -> Result<Zeroizing<String>> {
    let mut saved = vault
        .load()?
        .ok_or("Import the ContextDock Desktop OAuth client in Settings first.")?;
    let previous = saved
        .session
        .as_ref()
        .ok_or("Connect Google Drive in Settings first.")?;
    if previous.expires_at > now + 60 {
        return Ok(Zeroizing::new(previous.access_token.clone()));
    }
    let grant = http.token(&[
        ("client_id", &saved.config.client_id),
        ("client_secret", &saved.config.client_secret),
        ("refresh_token", &previous.refresh_token),
        ("grant_type", "refresh_token"),
    ])?;
    let next = session(
        &grant,
        Some(&previous.refresh_token),
        previous.account.clone(),
        now,
        false,
    )?;
    let token = Zeroizing::new(next.access_token.clone());
    saved.session = Some(next);
    // Persist rotated tokens before allowing any caller to use the new access token.
    vault.save(&saved)?;
    Ok(token)
}

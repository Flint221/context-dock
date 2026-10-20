use reqwest::{
    blocking::{multipart, Client},
    header::{HeaderValue, AUTHORIZATION},
    Method,
};
use serde_json::{json, Value};
use std::io::Read;
#[derive(Debug)]
pub struct ApiError {
    pub message: String,
    pub retry: bool,
    pub ambiguous: bool,
    pub missing: bool,
    pub delay: i64,
}
impl ApiError {
    pub fn local(message: &str) -> Self {
        Self {
            message: message.into(),
            retry: false,
            ambiguous: false,
            missing: false,
            delay: 0,
        }
    }
}
pub trait CloudApi {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, ApiError>;
    fn upload(&self, name: &str, bytes: Vec<u8>) -> Result<Value, ApiError>;
    fn content(&self, id: &str) -> Result<Vec<u8>, ApiError>;
}
pub struct OpenAi {
    client: Client,
    authorization: HeaderValue,
    deadline: std::time::Instant,
}
impl OpenAi {
    pub fn new(key: &str) -> Result<Self, String> {
        Self::with_timeout(key, 30)
    }
    pub fn for_analysis(key: &str) -> Result<Self, String> {
        Self::with_timeout(key, 90)
    }
    fn with_timeout(key: &str, seconds: u64) -> Result<Self, String> {
        let mut authorization = HeaderValue::from_str(&format!("Bearer {key}"))
            .map_err(|_| "Invalid credential.".to_string())?;
        authorization.set_sensitive(true);
        let client = Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(seconds))
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .build()
            .map_err(|_| "Cannot initialize secure API client.".to_string())?;
        Ok(Self {
            client,
            authorization,
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(90),
        })
    }
    fn send(
        &self,
        request: reqwest::blocking::RequestBuilder,
        mutation: bool,
        max: u64,
    ) -> Result<Vec<u8>, ApiError> {
        if std::time::Instant::now() > self.deadline {
            return Err(ApiError::local("Cloud reconciliation reached its time limit. No further requests were sent. Retry to continue safely."));
        }
        let response = request
            .header(AUTHORIZATION, self.authorization.clone())
            .send()
            .map_err(|e| ApiError {
                message: "OpenAI request could not complete. Check your network and retry.".into(),
                retry: !mutation || e.is_connect(),
                ambiguous: mutation && !e.is_connect(),
                missing: false,
                delay: 0,
            })?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            let delay = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| {
                    s.parse::<i64>().ok().or_else(|| {
                        chrono::DateTime::parse_from_rfc2822(s)
                            .ok()
                            .map(|t| t.timestamp() - chrono::Utc::now().timestamp())
                    })
                })
                .unwrap_or(0)
                .clamp(0, 3600);
            let message = match status {401=>"OpenAI rejected the API key. Replace or test the saved key.",403=>"This key does not have permission for this OpenAI resource.",404=>"The remote resource is missing or inaccessible. Reconcile the project.",429=>"OpenAI rate or quota limit reached. Check billing; retry after the delay.",400|413|422=>"OpenAI rejected this file or request. Check supported type, size and account limits.",_=>"OpenAI returned an error. Reconcile interrupted operations before retrying."};
            return Err(ApiError {
                message: message.into(),
                retry: status == 429 || (!mutation && status >= 500),
                ambiguous: mutation && status >= 500,
                missing: status == 404,
                delay,
            });
        }
        let mut bytes = Vec::new();
        response
            .take(max + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ApiError {
                message: "OpenAI response was interrupted; reconcile before retrying.".into(),
                retry: !mutation,
                ambiguous: mutation,
                missing: false,
                delay: 0,
            })?;
        if bytes.len() as u64 > max {
            return Err(ApiError::local(
                "OpenAI response exceeded the safety limit.",
            ));
        }
        Ok(bytes)
    }
}
impl CloudApi for OpenAi {
    fn request(&self, method: &str, path: &str, body: Option<Value>) -> Result<Value, ApiError> {
        let method = Method::from_bytes(method.as_bytes())
            .map_err(|_| ApiError::local("Invalid API operation."))?;
        if !path.starts_with('/') || path.contains("://") || path.contains("..") {
            return Err(ApiError::local("Invalid API path."));
        }
        let mutation = method != Method::GET;
        let mut req = self
            .client
            .request(method, format!("https://api.openai.com/v1{path}"));
        // The current HTTP reference retains this compatibility header for the
        // vector-store resource. No Assistants, Threads or Runs endpoints are used. Analysis uses Responses separately.
        if path.starts_with("/vector_stores") && !path.ends_with("/search") {
            req = req.header("OpenAI-Beta", "assistants=v2");
        }
        if let Some(body) = body {
            req = req.json(&body);
        }
        let bytes = self.send(req, mutation, 8 * 1024 * 1024)?;
        serde_json::from_slice(&bytes).map_err(|_| ApiError {
            message: "OpenAI returned an invalid response; reconcile before retrying.".into(),
            retry: !mutation,
            ambiguous: mutation,
            missing: false,
            delay: 0,
        })
    }
    fn upload(&self, name: &str, bytes: Vec<u8>) -> Result<Value, ApiError> {
        let form = multipart::Form::new().text("purpose", "assistants").part(
            "file",
            multipart::Part::bytes(bytes).file_name(name.to_owned()),
        );
        let bytes = self.send(
            self.client
                .post("https://api.openai.com/v1/files")
                .multipart(form),
            true,
            1024 * 1024,
        )?;
        serde_json::from_slice(&bytes).map_err(|_| ApiError {
            message: "Upload response was invalid. Reconcile before retrying.".into(),
            retry: false,
            ambiguous: true,
            missing: false,
            delay: 0,
        })
    }
    fn content(&self, id: &str) -> Result<Vec<u8>, ApiError> {
        valid_id(id, "file")?;
        self.send(
            self.client
                .get(format!("https://api.openai.com/v1/files/{id}/content")),
            false,
            16 * 1024 * 1024,
        )
    }
}
pub fn valid_id(id: &str, prefix: &str) -> Result<(), ApiError> {
    if id.len() > 128
        || !id.starts_with(prefix)
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(ApiError::local("Remote identifier is invalid."));
    }
    Ok(())
}
pub fn list(api: &dyn CloudApi, path: &str) -> Result<Vec<Value>, ApiError> {
    let mut rows = Vec::new();
    let mut after = String::new();
    for _ in 0..100 {
        let separator = if path.contains('?') { '&' } else { '?' };
        let page = api.request("GET", &format!("{path}{separator}limit=100{after}"), None)?;
        let data = page["data"]
            .as_array()
            .ok_or_else(|| ApiError::local("Remote listing was invalid."))?;
        rows.extend(data.iter().cloned());
        if page["has_more"] == false {
            return Ok(rows);
        }
        let id = data
            .last()
            .and_then(|v| v["id"].as_str())
            .ok_or_else(|| ApiError::local("Remote pagination was incomplete."))?;
        valid_id(
            id,
            if path.starts_with("/files") || path.ends_with("/files") {
                "file"
            } else {
                "vs_"
            },
        )?;
        let next = format!("&after={id}");
        if after == next {
            break;
        }
        after = next;
    }
    Err(ApiError::local(
        "Remote listing exceeded the reconciliation limit. No deletion was performed.",
    ))
}
pub fn attributes(
    install: &str,
    project: &str,
    document: &str,
    version: &str,
    hash: &str,
    current: bool,
) -> Value {
    json!({"contextdock":install,"project":project,"document":document,"version":version,"sha256":hash,"is_current":current})
}

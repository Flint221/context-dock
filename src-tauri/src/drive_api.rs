//! Narrow Drive v3 adapter. Uploads and verification stream bounded buffers.
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    io::Read,
    path::Path,
    time::Duration,
};
use zeroize::Zeroizing;

pub const FOLDER: &str = "application/vnd.google-apps.folder";
const FIELDS: &str = "id,name,mimeType,parents,appProperties,size,version,trashed";
#[derive(Clone, Debug)]
pub struct ApiError {
    pub message: String,
    pub retry: bool,
    pub auth: bool,
    pub missing: bool,
    pub delay: u64,
    pub waiting: bool,
}
impl ApiError {
    pub fn local(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retry: false,
            auth: false,
            missing: false,
            delay: 0,
            waiting: false,
        }
    }
    fn offline() -> Self {
        Self {
            retry: true,
            ..Self::local("Google Drive could not be reached. The upload remains queued.")
        }
    }
}
impl From<String> for ApiError {
    fn from(value: String) -> Self {
        if crate::storage::temporary_editor_read(&value)
            || value.contains("changed while being captured")
        {
            Self {
                retry: true,
                waiting: true,
                ..Self::local(value)
            }
        } else {
            Self::local(value)
        }
    }
}
pub type Result<T> = std::result::Result<T, ApiError>;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub app_properties: BTreeMap<String, String>,
    pub size: Option<String>,
    pub version: Option<String>,
    #[serde(default)]
    pub trashed: bool,
    #[serde(skip)]
    pub etag: Option<String>,
}
pub trait DriveApi {
    fn generate_id(&self) -> Result<String>;
    fn get(&self, id: &str) -> Result<Remote>;
    fn folders(&self, parent: &str, name: &str) -> Result<Vec<Remote>>;
    fn create_folder(
        &self,
        id: &str,
        parent: &str,
        name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Result<Remote>;
    fn upload(
        &self,
        id: &str,
        metadata: Value,
        file: &Path,
        create: bool,
        etag: Option<&str>,
    ) -> Result<Remote>;
    fn download_hash(&self, id: &str, size: i64) -> Result<String>;
}
pub fn valid_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 256
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err(ApiError::local(
            "Google Drive returned an invalid file identity.",
        ));
    }
    Ok(())
}
pub struct GoogleDrive {
    http: Client,
    access: Zeroizing<String>,
    guard: Box<dyn Fn(bool) -> Result<()>>,
    #[cfg(test)]
    test_origin: Option<String>,
}
impl GoogleDrive {
    pub fn new(
        access: Zeroizing<String>,
        guard: impl Fn(bool) -> Result<()> + 'static,
    ) -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(180))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| ApiError::local("Could not initialize Google Drive."))?;
        Ok(Self {
            http,
            access,
            guard: Box::new(guard),
            #[cfg(test)]
            test_origin: None,
        })
    }
    fn url(&self, path: &str) -> String {
        #[cfg(test)]
        if let Some(origin) = &self.test_origin {
            return format!("{origin}{path}");
        }
        format!("https://www.googleapis.com{path}")
    }
    #[cfg(test)]
    pub(crate) fn loopback_test(
        origin: &str,
        guard: impl Fn(bool) -> Result<()> + 'static,
    ) -> Self {
        let url = reqwest::Url::parse(origin).unwrap();
        assert_eq!(url.scheme(), "http");
        assert_eq!(url.host_str(), Some("127.0.0.1"));
        let mut api = Self::new(Zeroizing::new("disposable-test-token".into()), guard).unwrap();
        api.test_origin = Some(origin.to_owned());
        api
    }
    fn send(&self, request: reqwest::blocking::RequestBuilder, write: bool) -> Result<Response> {
        (self.guard)(write)?;
        let response = request
            .bearer_auth(&*self.access)
            .send()
            .map_err(|_| ApiError::offline())?;
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status().as_u16();
        let delay = response
            .headers()
            .get("retry-after")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
            .min(3600);
        let mut body = Vec::new();
        let _ = response.take(8192).read_to_end(&mut body);
        let data: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        let quota = data["error"]["errors"].as_array().is_some_and(|es| {
            es.iter().any(|e| {
                ["rateLimitExceeded", "userRateLimitExceeded", "backendError"]
                    .contains(&e["reason"].as_str().unwrap_or(""))
            })
        });
        let retry = status == 308
            || status == 408
            || status == 409
            || status == 429
            || status >= 500
            || quota;
        let message = match status {
            401 => "Google authorization expired or was revoked. Connect Google Drive again.",
            403 if !quota => "Google Drive refused access. Check file permissions, storage space and the enabled Drive API.",
            404 => "The mapped Google Drive file or folder is missing or inaccessible. Review the destination before retrying.",
            412 => "The cloud file changed during publication. Review it before retrying.",
            _ if retry => "Google Drive is temporarily unavailable. The upload remains queued.",
            _ => "Google Drive rejected the operation. Review the connection and destination.",
        };
        Err(ApiError {
            message: message.into(),
            retry,
            auth: status == 401,
            missing: status == 404,
            delay,
            waiting: false,
        })
    }
    fn remote(response: Response) -> Result<Remote> {
        let etag = response
            .headers()
            .get("etag")
            .and_then(|h| h.to_str().ok())
            .map(str::to_owned);
        let mut remote: Remote = response
            .json()
            .map_err(|_| ApiError::local("Google Drive returned incomplete file metadata."))?;
        valid_id(&remote.id)?;
        remote.etag = etag;
        Ok(remote)
    }
}
impl DriveApi for GoogleDrive {
    fn generate_id(&self) -> Result<String> {
        let data: Value = self
            .send(
                self.http
                    .get(self.url("/drive/v3/files/generateIds"))
                    .query(&[("count", "1"), ("space", "drive"), ("type", "files")]),
                false,
            )?
            .json()
            .map_err(|_| ApiError::local("Google Drive did not reserve a file identity."))?;
        let id = data["ids"]
            .as_array()
            .filter(|a| a.len() == 1)
            .and_then(|a| a[0].as_str())
            .ok_or_else(|| {
                ApiError::local("Google Drive did not reserve exactly one file identity.")
            })?;
        valid_id(id)?;
        Ok(id.into())
    }
    fn get(&self, id: &str) -> Result<Remote> {
        valid_id(id)?;
        Self::remote(
            self.send(
                self.http
                    .get(self.url(&format!("/drive/v3/files/{id}")))
                    .query(&[("fields", FIELDS)]),
                false,
            )?,
        )
    }
    fn folders(&self, parent: &str, name: &str) -> Result<Vec<Remote>> {
        valid_id(parent)?;
        let name = name.replace('\\', "\\\\").replace('\'', "\\'");
        let query = format!("'{parent}' in parents and name = '{name}' and mimeType = '{FOLDER}' and trashed = false");
        let mut page = String::new();
        let mut pages = HashSet::new();
        let mut all = vec![];
        loop {
            if !pages.insert(page.clone()) || pages.len() > 500 {
                return Err(ApiError::local(
                    "Google Drive folder listing was incomplete. No new folder was created.",
                ));
            }
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Listing {
                #[serde(default)]
                files: Vec<Remote>,
                next_page_token: Option<String>,
                #[serde(default)]
                incomplete_search: bool,
            }
            let listing: Listing = self
                .send(
                    self.http.get(self.url("/drive/v3/files")).query(&[
                        ("q", query.as_str()),
                        ("spaces", "drive"),
                        ("pageSize", "100"),
                        (
                            "fields",
                            &format!("nextPageToken,incompleteSearch,files({FIELDS})"),
                        ),
                        ("pageToken", page.as_str()),
                    ]),
                    false,
                )?
                .json()
                .map_err(|_| {
                    ApiError::local("Google Drive folder listing could not be verified.")
                })?;
            if listing.incomplete_search {
                return Err(ApiError::local(
                    "Google Drive returned an incomplete folder search. No folder was created.",
                ));
            }
            all.extend(listing.files);
            match listing.next_page_token.filter(|p| !p.is_empty()) {
                Some(next) => page = next,
                None => return Ok(all),
            }
        }
    }
    fn create_folder(
        &self,
        id: &str,
        parent: &str,
        name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Result<Remote> {
        valid_id(id)?;
        valid_id(parent)?;
        Self::remote(self.send(self.http.post(self.url("/drive/v3/files")).query(&[("fields",FIELDS)])
            .json(&json!({"id":id,"name":name,"mimeType":FOLDER,"parents":[parent],"appProperties":properties})), true)?)
    }
    fn upload(
        &self,
        id: &str,
        mut metadata: Value,
        file: &Path,
        create: bool,
        etag: Option<&str>,
    ) -> Result<Remote> {
        valid_id(id)?;
        let handle = crate::storage::locked_read(file, false).map_err(ApiError::local)?;
        let length = handle
            .metadata()
            .map_err(|_| ApiError::local("The temporary upload copy is unavailable."))?
            .len();
        let (method, url) = if create {
            metadata["id"] = json!(id);
            (reqwest::Method::POST, self.url("/upload/drive/v3/files"))
        } else {
            (
                reqwest::Method::PATCH,
                self.url(&format!("/upload/drive/v3/files/{id}")),
            )
        };
        let mut request = self
            .http
            .request(method, url)
            .query(&[("uploadType", "resumable"), ("fields", FIELDS)])
            .header(
                "X-Upload-Content-Type",
                metadata["mimeType"]
                    .as_str()
                    .unwrap_or("application/octet-stream"),
            )
            .header("X-Upload-Content-Length", length)
            .json(&metadata);
        if let Some(etag) = etag {
            request = request.header("If-Match", etag);
        }
        let response = self.send(request, true)?;
        let location = response
            .headers()
            .get("location")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| ApiError::local("Google Drive did not return an upload session."))?;
        let location = reqwest::Url::parse(location)
            .map_err(|_| ApiError::local("Google Drive returned an invalid upload session."))?;
        let allowed = location.scheme() == "https"
            && location.host_str() == Some("www.googleapis.com")
            && location.port_or_known_default() == Some(443);
        #[cfg(test)]
        let allowed = allowed
            || self
                .test_origin
                .as_ref()
                .is_some_and(|origin| location.origin().ascii_serialization() == *origin);
        if !allowed
            || !location.username().is_empty()
            || location.password().is_some()
            || !location.path().starts_with("/upload/drive/v3/files")
        {
            return Err(ApiError::local(
                "Google Drive returned an unsupported upload destination.",
            ));
        }
        // Session URLs are short-lived capabilities: keep them in native memory,
        // never in SQLite, UI status, activity logs or error messages. After a
        // restart, reconcile the reserved file ID and open a fresh session.
        Self::remote(
            self.send(
                self.http
                    .put(location)
                    .header("Content-Length", length)
                    .header(
                        "Content-Type",
                        metadata["mimeType"]
                            .as_str()
                            .unwrap_or("application/octet-stream"),
                    )
                    .body(reqwest::blocking::Body::sized(handle, length)),
                true,
            )?,
        )
    }
    fn download_hash(&self, id: &str, size: i64) -> Result<String> {
        valid_id(id)?;
        if size < 0 {
            return Err(ApiError::local("Invalid expected download size."));
        }
        let mut response = self.send(
            self.http
                .get(self.url(&format!("/drive/v3/files/{id}")))
                .query(&[("alt", "media")]),
            false,
        )?;
        let mut hash = Sha256::new();
        let mut count = 0u64;
        let mut buffer = [0u8; 65_536];
        loop {
            let n = response
                .read(&mut buffer)
                .map_err(|_| ApiError::offline())?;
            if n == 0 {
                break;
            }
            count += n as u64;
            if count > size as u64 {
                return Err(ApiError::local(
                    "Downloaded Drive content exceeded the expected size. Verification failed.",
                ));
            }
            hash.update(&buffer[..n]);
        }
        if count != size as u64 {
            return Err(ApiError::local(
                "Downloaded Drive content was incomplete. Verification failed.",
            ));
        }
        Ok(format!("{:x}", hash.finalize()))
    }
}

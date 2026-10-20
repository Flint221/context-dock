use crate::{
    ai_commands::Runtime,
    ai_inference, chat,
    cloud::{self, s},
    cloud_api::{ApiError, CloudApi, OpenAi},
    cloud_credentials,
    registry::{Registry, Result},
};
use serde_json::Value;
struct CredentialApi<'a> {
    api: OpenAi,
    key: &'a str,
    preview: Option<tauri::ipc::Channel<String>>,
}
impl CloudApi for CredentialApi<'_> {
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> std::result::Result<Value, ApiError> {
        if body
            .as_ref()
            .is_some_and(|v| ai_inference::contains_credential(v, self.key))
        {
            return Err(ApiError::local("The question or retrieved text contains the saved API key. Remove it before sending. No request was sent."));
        }
        let response = if method == "POST" && path == "/responses" {
            self.api.stream_answer(
                body.ok_or_else(|| ApiError::local("Missing answer request."))?,
                &mut |text| {
                    if let Some(channel) = &self.preview {
                        let safe = crate::chat_stream::safe_preview(text, self.key);
                        let _ = channel.send(safe);
                    }
                },
            )?
        } else {
            self.api.request(method, path, body)?
        };
        if ai_inference::contains_credential(&response, self.key) {
            return Err(ApiError::local(
                "The response contained credential-like text and was not saved.",
            ));
        }
        Ok(response)
    }
    fn upload(&self, _: &str, _: Vec<u8>) -> std::result::Result<Value, ApiError> {
        Err(ApiError::local("Chat cannot upload files."))
    }
    fn content(&self, _: &str) -> std::result::Result<Vec<u8>, ApiError> {
        Err(ApiError::local("Chat cannot download files."))
    }
}
#[tauri::command]
pub async fn chat_action(
    state: tauri::State<'_, Runtime>,
    project: String,
    action: String,
    payload: Value,
    preview: tauri::ipc::Channel<String>,
) -> Result<Value> {
    let path = state.path.clone();
    let gate = state.gate.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let r = Registry {
            db: cloud::connect(&path)?,
            store: path.parent().ok_or("Invalid chat storage.")?.to_path_buf(),
        };
        match action.as_str() {
            "list" => chat::list(&r, &project),
            "create" => chat::create(&r, &project),
            "get" => chat::get(&r, &project, s(&payload, "id")),
            "delete" => {
                chat::delete(&r, &project, s(&payload, "id"))?;
                Ok(Value::Null)
            }
            "send" => {
                let _permit = gate
                    .try_lock()
                    .map_err(|_| "Another answer is still running. Please wait before sending.")?;
                // Authorization is checked before touching credentials, then again for every request.
                let scope: String =
                    r.db.query_row(
                        "SELECT COALESCE(scope,'') FROM cloud_connection WHERE singleton=1",
                        [],
                        |r| r.get(0),
                    )
                    .map_err(crate::registry::error)?;
                chat::authorize(&r.db, &project, &scope)?;
                let key = cloud_credentials::read()?
                    .ok_or("Save and test your OpenAI key in Settings first.")?;
                if cloud::scope(&key) != scope {
                    return Err("The saved key changed. Test it in Settings before sending.".into());
                }
                let api = CredentialApi {
                    api: OpenAi::for_analysis(&key)?,
                    key: &key,
                    preview: Some(preview),
                };
                chat::ask(
                    &r,
                    &project,
                    s(&payload, "id"),
                    s(&payload, "question"),
                    &api,
                    &scope,
                )
            }
            _ => Err("Unknown chat action.".into()),
        }
    })
    .await
    .map_err(|_| {
        "The answer task stopped. Open the saved chat before sending another paid request."
            .to_string()
    })?
}

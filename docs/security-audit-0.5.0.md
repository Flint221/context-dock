# ContextDock 0.5.0 — OpenAI integration security review

Scope: repository source and available local Git history, build/configuration inputs, credential/UI/native HTTP code, analysis persistence, cloud worker and existing Actions workflow. This is a focused engineering review with bounded pattern checks, not a penetration test, a proof that no secret has ever escaped, or an inspection of the user's real saved key/account.

## Redacted pattern results

The initial pre-Milestone-5 scan examined 633 objects/345 blobs with no candidates. A final available-history scan through commit ba213b5 examined 698 Git objects, including 389 unique text/binary blobs totaling 5,643,599 bytes. It searched broadly for OpenAI-style key strings and Discord webhook endpoints without printing matched strings. Two historical blob versions of one explicit synthetic, nonfunctional fixture in `src-tauri/src/ai_tests.rs` matched (redacted match fingerprint `687199f4`); a separate 101-file tracked-source scan identified the same fixture. It is an output-rejection test, not an application credential. No candidate requiring credential remediation was found. Patterns are heuristic: a negative match is not universal secret detection, and the history scan covers only available local refs/objects. No shared Git history was rewritten.

## Credential boundary

- The only credential store remains the per-user Windows generic credential `ContextDock.OpenAI`; no second store, environment fallback or plaintext database field was added.
- Save uses native Credential Manager. Reads return `Zeroizing<String>`; the native credential blob is wiped before freeing. The browser password field clears immediately after Save; it is uncontrolled and saved keys are never returned via IPC, stored in browser storage or inserted into report/evidence JSON.
- Authorization is set only in the existing native fixed-origin HTTPS client. Sensitive header marking is retained. Client/header memory exists transiently while requests run; zeroizing the vault read is not a guarantee against same-user malware, debuggers, OS compromise or all allocator copies.
- Connected key fingerprints are rechecked immediately before analysis dispatch. The model never receives the authorization header as prompt data. Missing/replaced/disconnected credentials stop new inference without blocking local features.

## Network and logging

- Application API origin is fixed to `https://api.openai.com/v1`. HTTPS-only, normal Windows certificate validation, TLS 1.2 minimum and redirect refusal remain enforced. No arbitrary user-supplied inference URL or proxy API credential destination was added.
- Inference adds only `POST /responses` through a narrow `InferenceApi::respond` abstraction. Existing cloud Files/vector-store endpoints remain independent. Upload `purpose=assistants` is an API parameter, not use of deprecated Assistants/Threads/Runs endpoints.
- Inference uses a separate 90-second bounded request and its own gate/SQLite connection. It does not occupy the cloud gate, shared registry mutex or SQL write transaction during HTTP. Explicit tasks perform at most one paid request, with no automatic retry or recurring analysis worker.
- Transport/status/parse errors are generic and never return raw provider error bodies, request headers or reqwest debug dumps. No secret-bearing console/println/debug telemetry was introduced. Stored diagnostics do not contain the full provider response or request authorization.
- Browser account/docs links have the existing official-site allowlist. No Discord, webhook, email, messaging or telemetry integration was added. Standard NSIS WebView2 installation may download Microsoft's runtime when missing; that is separate from OpenAI authentication.

## Evidence, prompting and persistence

Every selected snapshot is directory-guarded and SHA-256 verified. Project/version membership, pair identity, baseline ownership and timeline range are checked natively. Missing, tampered, escaped or excessive evidence fails before a paid request. Full selected extracted text and all selected transitions are included under explicit limits; no silent similarity-search substitution or truncated comprehensive review exists. DOCX body extraction exclusions are displayed and saved.

The Responses request sets `store:false`, `background:false`, fixed developer instructions and strict JSON Schema. There are no tools, function calls, conversations, hosted agents, file search or autonomous actions in analysis requests. Document/baseline/diff contents are explicitly untrusted JSON evidence. A document instruction cannot gain credential access, delete data, upload files or trigger additional requests. This reduces prompt-injection capability; it does not prove the model's interpretation is correct.

Preflight rejects OpenAI credential-like text in selected evidence and baseline fields before saving an analysis plan or approving a baseline. A final native check blocks the saved credential itself from appearing anywhere in the outgoing body. Detected values are never included in the error. Pattern checks can reject long synthetic placeholders too; use redacted sample documentation.

The provider returns one bounded structured message. Refusals, incomplete outputs, unknown fields/categories, invented evidence IDs, nonmatching quotations and invalid timeline anchors/order are rejected. Credential-like strings in model output are rejected without persisting the text. Only numeric expected usage counters and a validated response ID are retained from provider metadata. Reports include locally selected evidence and can contain sensitive documentation by design; they are local plaintext application data protected by the user's OS account, not an encrypted document vault.

Approved baselines have immutable revision rows with approval/source/hash/supersession metadata. Reports pin their exact baseline revision, snapshots, evidence, model, limits, prompt/schema versions and price assumptions. Review decisions are local to one report/finding. No AI action edits sources, archives or baseline history. Cache lookup/opening is local; a fresh generation requires another explicit approval.

`store:false` disables Responses application-state storage for this request; it does not override all OpenAI abuse monitoring, processing or retention policies. Existing Files/vector stores have their separately controlled retention lifecycle. See [OpenAI data controls](https://developers.openai.com/api/docs/guides/your-data).

## Build, GitHub and installation

`.gitignore` excludes `.env*` (except an example), databases/WAL/SHM files, logs, `node_modules`, generated Rust/Tauri outputs and installer targets. Tauri packages the built frontend and native binary; application data, the vault, source-document snapshots and test fixture data directories are not configured as bundle resources. Test-only synthetic credentials are under `#[cfg(test)]`, outside the release binary. No developer key is used for compilation or packaging.

The existing Actions workflow has read-only default permissions and pinned actions. It does not receive an OpenAI key; tests use fixtures/mocks. Its tag-only release job has its own GitHub token for GitHub operations, not an OpenAI token. This milestone creates no tag/public release, updater, workflow secret or telemetry destination. Browser CSP restricts connections to local IPC; inference stays native. Existing capability permissions remain limited to local folder-dialog/event operations.

## Validation and limitations

Native tests exercise exact project scope, integrity, approved baseline revisions, budget refusal, no automatic inference, stateless/schema/tool boundaries, synthetic credential-like output rejection, invalid citations, cancellations, failure/timeout/rate-limit state and restart uncertainty. Existing credential-form tests verify immediate input clearing and no browser-storage writes. All tests use synthetic files/credentials and mocked inference; there was no real credential read or authenticated OpenAI call by the agent.

A repository audit cannot establish whether an actual key was posted in private Discord messages, chats, screenshots, other repositories or unrelated applications. No such access was requested/performed. If the user knows a real key was exposed anywhere, revoke and replace it in OpenAI and update Credential Manager. Never copy it into an issue/report; shared history should not be rewritten automatically.

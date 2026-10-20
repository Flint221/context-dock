//! Local, deterministic evidence and immutable approved baseline revisions. No network here.
use crate::{
    ai_models::*,
    cloud::{self, s},
    comparison, extraction, models,
    registry::{error, Registry, Result},
};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
pub const MINI: &str = "gpt-5.4-mini-2026-03-17";
pub const QUALITY: &str = "gpt-5.4-2026-03-05";
pub fn rates(model: &str) -> Result<(f64, f64)> {
    match model {
        MINI => Ok((0.75, 4.5)),
        QUALITY => Ok((2.5, 15.0)),
        _ => Err("Choose a supported analysis model.".into()),
    }
}
pub fn project(r: &Registry, id: &str) -> Result<()> {
    let present: i64 =
        r.db.query_row(
            "SELECT COUNT(*) FROM projects WHERE id=?1 AND removed_at IS NULL",
            [id],
            |v| v.get(0),
        )
        .map_err(error)?;
    if present != 1 {
        return Err("Project is not registered.".into());
    }
    Ok(())
}
pub fn source(r: &Registry, project_id: &str, id: &str) -> Result<(Source, String, Vec<String>)> {
    let owner:String=r.db.query_row("SELECT d.project_id FROM document_versions v JOIN documents d ON d.id=v.document_id WHERE v.id=?1",[id],|v|v.get(0)).map_err(|_|"Captured version not found.".to_string())?;
    if owner != project_id {
        return Err("Evidence must belong to the selected project.".into());
    }
    let job = r.comparison_job(id, id)?;
    let bytes = comparison::verified_bytes(&job.store, &job.earlier)?;
    let extracted = extraction::extract(&bytes, &job.kind)?;
    let v = &job.earlier;
    let current: bool =
        r.db.query_row(
            "SELECT COALESCE(current_version_id=?2,0) FROM documents WHERE id=?1",
            rusqlite::params![v.document_id, id],
            |v| v.get(0),
        )
        .map_err(error)?;
    Ok((
        Source {
            document: v.document_id.clone(),
            version: v.id.clone(),
            number: v.number,
            name: job.name,
            hash: v.sha256.clone(),
            captured_at: v.captured_at.clone(),
            current,
        },
        extracted.text,
        extracted.warnings,
    ))
}
pub fn baseline(r: &Registry, p: &str, id: &str) -> Result<Value> {
    cloud::rows(
        &r.db,
        "SELECT * FROM ai_baselines WHERE project_id=?1 AND id=?2",
        &[&p, &id],
    )?
    .pop()
    .ok_or("Approved baseline not found in this project.".into())
}
pub fn approve(r: &mut Registry, p: &str, payload: &Value) -> Result<String> {
    project(r, p)?;
    if payload["approve"] != true {
        return Err("Approve this baseline explicitly before saving.".into());
    }
    let title = s(payload, "title").trim();
    let content: Goals = serde_json::from_value(payload["content"].clone())
        .map_err(|_| "Invalid baseline fields.")?;
    let serialized = serde_json::to_string(&content).map_err(|_| "Invalid baseline.")?;
    if title.is_empty()
        || title.len() > 160
        || content.purpose.trim().is_empty()
        || serialized.len() > 24000
    {
        return Err(
            "Enter a baseline title and purpose, with at most 24,000 bytes of baseline text."
                .into(),
        );
    }
    let source_id = payload["sourceVersion"].as_str().filter(|s| !s.is_empty());
    let hash = source_id
        .map(|id| source(r, p, id).map(|v| v.0.hash))
        .transpose()?;
    let tx = r.db.transaction().map_err(error)?;
    let previous: Option<String> = tx
        .query_row(
            "SELECT baseline_id FROM ai_current_baseline WHERE project_id=?1",
            [p],
            |v| v.get(0),
        )
        .optional()
        .map_err(error)?;
    if previous.as_deref() != payload["supersedes"].as_str() {
        return Err("The active baseline changed. Reopen the baseline editor.".into());
    }
    let revision: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(revision),0)+1 FROM ai_baselines WHERE project_id=?1",
            [p],
            |v| v.get(0),
        )
        .map_err(error)?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = models::now();
    tx.execute(
        "INSERT INTO ai_baselines VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",
        params![id, p, title, revision, previous, source_id, hash, serialized, now],
    )
    .map_err(error)?;
    tx.execute("INSERT INTO ai_current_baseline VALUES(?1,?2) ON CONFLICT(project_id) DO UPDATE SET baseline_id=excluded.baseline_id",params![p,id]).map_err(error)?;
    tx.commit().map_err(error)?;
    Ok(id)
}
pub fn view(r: &Registry, p: &str) -> Result<Value> {
    project(r, p)?;
    let name: String =
        r.db.query_row("SELECT name FROM projects WHERE id=?1", [p], |v| v.get(0))
            .map_err(error)?;
    Ok(
        json!({"projectName":name,"baselines":cloud::rows(&r.db,"SELECT b.*,b.id=c.baseline_id active FROM ai_baselines b LEFT JOIN ai_current_baseline c ON c.project_id=b.project_id WHERE b.project_id=?1 ORDER BY revision DESC",&[&p])?,
    "versions":cloud::rows(&r.db,"SELECT v.id,v.document_id document,v.number,v.captured_at capturedAt,d.family_name name,d.current_version_id=v.id current FROM document_versions v JOIN documents d ON d.id=v.document_id WHERE d.project_id=?1 ORDER BY d.family_name,v.number",&[&p])?,
    "reports":cloud::rows(&r.db,"SELECT id,kind,state,created_at,completed_at,error FROM ai_reports WHERE project_id=?1 ORDER BY created_at DESC LIMIT 100",&[&p])?}),
    )
}
fn chunks(evidence: &mut Vec<Evidence>, version: &str, text: &str) {
    let lines: Vec<_> = text.split('\n').collect();
    let mut heading = String::new();
    for (part, chunk) in lines.chunks(24).enumerate() {
        for line in chunk {
            if line.starts_with('#') {
                heading = line
                    .trim_start_matches('#')
                    .trim()
                    .chars()
                    .take(160)
                    .collect();
                break;
            }
        }
        evidence.push(Evidence {
            id: format!("e{}", evidence.len() + 1),
            version: version.into(),
            heading: heading.clone(),
            start: part * 24 + 1,
            end: part * 24 + chunk.len(),
            text: chunk.join("\n"),
        });
    }
}
pub fn plan(r: &Registry, p: &str, request: &Request) -> Result<Plan> {
    project(r, p)?;
    let (input_rate, output_rate) = rates(&request.model)?;
    if !(4096..=65536).contains(&request.input_budget)
        || !(512..=8192).contains(&request.output_tokens)
    {
        return Err("Input budget must be 4096–65536 conservative token units; output limit 512–8192 tokens.".into());
    }
    if !["compare", "alignment", "timeline", "consistency"].contains(&request.kind.as_str()) {
        return Err("Unknown analysis workflow.".into());
    }
    let mut ids = request.versions.clone();
    if ids.is_empty() || ids.len() > 12 || ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err("Select 1–12 distinct captured versions.".into());
    }
    let mut data = ids
        .iter()
        .map(|id| source(r, p, id))
        .collect::<Result<Vec<_>>>()?;
    data.sort_by(|a, b| (&a.0.document, a.0.number).cmp(&(&b.0.document, b.0.number)));
    if ["compare", "timeline"].contains(&request.kind.as_str()) {
        if data.len() != 2 || data[0].0.document != data[1].0.document {
            return Err("Select two distinct endpoints from one document family.".into());
        }
        if request.kind == "timeline" {
            ids=cloud::rows(&r.db,"SELECT id FROM document_versions WHERE document_id=?1 AND number BETWEEN ?2 AND ?3 ORDER BY number",&[&data[0].0.document,&data[0].0.number,&data[1].0.number])?.iter().map(|v|s(v,"id").to_string()).collect();
            if ids.len() > 12 {
                return Err("This range contains more than 12 captures. Choose a smaller range; no intermediate versions were omitted or sent.".into());
            }
            data = ids
                .iter()
                .map(|id| source(r, p, id))
                .collect::<Result<Vec<_>>>()?;
        }
    }
    if request.kind == "consistency"
        && (data.len() < 2
            || data
                .iter()
                .map(|d| &d.0.document)
                .collect::<HashSet<_>>()
                .len()
                != data.len()
            || data.len() > 6)
    {
        return Err(
            "Select one version from each of 2–6 distinct families in this project.".into(),
        );
    }
    if request.kind == "alignment" && data.len() > 6 {
        return Err("Choose at most six versions for alignment.".into());
    }
    let mut selected_baseline = if request.kind == "alignment" || request.baseline.is_some() {
        Some(baseline(
            r,
            p,
            request
                .baseline
                .as_deref()
                .ok_or("Approve and select a baseline first.")?,
        )?)
    } else {
        None
    };
    let mut evidence = vec![];
    let mut warnings = vec![];
    let mut transitions = vec![];
    for (v, text, notes) in &data {
        chunks(&mut evidence, &v.version, text);
        warnings.extend(
            notes
                .iter()
                .map(|note| format!("{} v{}: {}", v.name, v.number, note)),
        );
    }
    if let Some(b) = &mut selected_baseline {
        if let Some(id) = b["source_version"].as_str() {
            let verified = source(r, p, id)?;
            if verified.0.hash != s(b, "source_hash") {
                return Err("Baseline source integrity changed.".into());
            }
            b["source"] =
                serde_json::to_value(verified.0).map_err(|_| "Invalid baseline source.")?;
        }
        chunks(&mut evidence, s(b, "id"), s(b, "content"));
    }
    if ["compare", "timeline"].contains(&request.kind.as_str()) {
        for pair in data.windows(2) {
            let diff =
                comparison::compare(r.comparison_job(&pair[0].0.version, &pair[1].0.version)?)?;
            transitions.push(Transition {
                earlier: diff.earlier.id,
                later: diff.later.id,
                blocks: serde_json::to_value(
                    diff.blocks
                        .into_iter()
                        .filter(|b| b.kind == "change")
                        .collect::<Vec<_>>(),
                )
                .map_err(|_| "Invalid factual diff.")?,
            });
        }
    }
    let mut normalized = request.clone();
    // Timelines retain endpoints in the request; full intermediate identities are in sources.
    normalized.versions = if request.kind == "timeline" {
        vec![
            data[0].0.version.clone(),
            data.last().unwrap().0.version.clone(),
        ]
    } else {
        data.iter().map(|v| v.0.version.clone()).collect()
    };
    let scope = if warnings.iter().any(|s| s.contains("DOCX body")) {
        "Complete selected evidence under the disclosed DOCX body-text policy; excluded Word structures are not analyzed."
    } else {
        "Complete text of the explicitly selected snapshots and baseline; no excerpts were silently dropped. Other project documents are outside scope."
    };
    let mut plan = Plan {
        project: p.into(),
        request: normalized,
        sources: data.into_iter().map(|v| v.0).collect(),
        baseline: selected_baseline,
        evidence,
        transitions,
        warnings,
        scope: scope.into(),
        prompt_version: 1,
        schema_version: 1,
        input_units: 0,
        estimated_tokens: 0,
        estimate_usd: 0.0,
        conservative_usd: 0.0,
        pricing_date: "2026-10-19; standard USD token rates, excludes taxes/account adjustments"
            .into(),
        cache_key: String::new(),
    };
    let input = serde_json::to_vec(&plan)
        .map_err(|_| "Cannot assemble evidence.")?
        .len()
        + crate::ai_inference::instructions().len()
        + crate::ai_inference::schema().to_string().len()
        + 1024;
    if input > request.input_budget {
        return Err(format!("Complete evidence needs {input} conservative input units; configured limit {}. No request was sent. Narrow the version/family range or explicitly raise the budget (maximum 65536).",request.input_budget));
    }
    plan.input_units = input;
    plan.estimated_tokens = input.div_ceil(4);
    plan.estimate_usd = (plan.estimated_tokens as f64 * input_rate
        + request.output_tokens as f64 * output_rate)
        / 1_000_000.0;
    plan.conservative_usd =
        (input as f64 * input_rate + request.output_tokens as f64 * output_rate) / 1_000_000.0;
    plan.cache_key = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&plan).map_err(|_| "Invalid evidence.")?)
    );
    Ok(plan)
}
pub fn report(r: &Registry, p: &str, id: &str) -> Result<Value> {
    let mut row = cloud::rows(
        &r.db,
        "SELECT * FROM ai_reports WHERE project_id=?1 AND id=?2",
        &[&p, &id],
    )?
    .pop()
    .ok_or("Saved analysis not found in this project.")?;
    for field in ["request", "plan", "result", "usage"] {
        if let Some(text) = row[field].as_str() {
            row[field] = serde_json::from_str(text).map_err(|_| "Saved analysis is invalid.")?;
        }
    }
    row["decisions"] = json!(cloud::rows(
        &r.db,
        "SELECT finding,decision,updated_at FROM ai_decisions WHERE report_id=?1",
        &[&id]
    )?);
    Ok(row)
}
pub fn prepare(r: &Registry, p: &str, request: Request) -> Result<Value> {
    let plan = plan(r, p, &request)?;
    let cached=cloud::rows(&r.db,"SELECT id FROM ai_reports WHERE project_id=?1 AND cache_key=?2 AND state='completed' ORDER BY completed_at DESC LIMIT 1",&[&p,&plan.cache_key])?.pop();
    let id = uuid::Uuid::new_v4().to_string();
    r.db.execute("INSERT INTO ai_reports(id,project_id,cache_key,kind,state,request,plan,created_at) VALUES(?1,?2,?3,?4,'preview',?5,?6,?7)",params![id,p,plan.cache_key,request.kind,serde_json::to_string(&plan.request).map_err(|_|"Invalid request.")?,serde_json::to_string(&plan).map_err(|_|"Invalid plan.")?,models::now()]).map_err(error)?;
    Ok(
        json!({"id":id,"plan":plan,"cached":cached.map(|v|s(&v,"id").to_string()),"requests":1,"retrieval":false}),
    )
}

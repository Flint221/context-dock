//! Filename versions are published versions; immutable saves are revisions.
use crate::{
    matching,
    registry::{error, Result},
};
use rusqlite::{Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};

pub fn key(name: &str) -> String {
    matching::version_label(name)
        .map(|label| {
            let mut parts = label[1..]
                .split('.')
                .map(|p| p.trim_start_matches('0'))
                .map(|p| if p.is_empty() { "0" } else { p })
                .collect::<Vec<_>>();
            while parts.len() > 1 && parts.last() == Some(&"0") {
                parts.pop();
            }
            format!("v{}", parts.join("."))
        })
        .unwrap_or_else(|| "working".into())
}
fn groups(db: &Connection, document: &str) -> Result<Vec<(String, String)>> {
    let mut query = db
        .prepare(
            "SELECT id,source_name,version_key FROM document_versions WHERE document_id=?1 ORDER BY number",
        )
        .map_err(error)?;
    let rows = query
        .query_map([document], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    let mut group = "working".to_string();
    Ok(rows
        .into_iter()
        .map(|(id, name, saved)| {
            if let Some(saved) = saved {
                group = saved;
            } else if matching::version_label(&name).is_some() {
                group = key(&name);
            }
            (id, group.clone())
        })
        .collect())
}
pub fn effective(db: &Connection, document: &str, name: &str) -> Result<String> {
    if matching::version_label(name).is_some() {
        return Ok(key(name));
    }
    let current: Option<String> = db
        .query_row(
            "SELECT current_version_id FROM documents WHERE id=?1",
            [document],
            |r| r.get(0),
        )
        .map_err(error)?;
    Ok(groups(db, document)?
        .into_iter()
        .find(|(id, _)| Some(id) == current.as_ref())
        .map(|(_, key)| key)
        .unwrap_or_else(|| "working".into()))
}
pub fn same(db: &Connection, previous: &str, name: &str) -> Result<bool> {
    let document: String = db
        .query_row(
            "SELECT document_id FROM document_versions WHERE id=?1",
            [previous],
            |r| r.get(0),
        )
        .map_err(error)?;
    let next = effective(db, &document, name)?;
    Ok(groups(db, &document)?
        .into_iter()
        .any(|(id, key)| id == previous && key == next))
}
pub fn content_match(
    db: &Connection,
    document: &str,
    hash: &str,
    name: &str,
) -> Result<Option<String>> {
    let target = effective(db, document, name)?;
    let matches = groups(db, document)?
        .into_iter()
        .filter(|(_, group)| group == &target)
        .map(|(id, _)| id)
        .collect::<HashSet<_>>();
    let mut query=db.prepare("SELECT id FROM document_versions WHERE document_id=?1 AND sha256=?2 ORDER BY number DESC").map_err(error)?;
    let rows = query
        .query_map(rusqlite::params![document, hash], |r| r.get::<_, String>(0))
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    Ok(rows.into_iter().find(|id| matches.contains(id)))
}
/// Immutable saves remain retained. Only the newest save of each filename
/// version is published. The current pointer wins after a byte-for-byte undo.
pub fn heads(db: &Connection, document: &str) -> Result<HashSet<String>> {
    let current: Option<String> = db
        .query_row(
            "SELECT current_version_id FROM documents WHERE id=?1",
            [document],
            |r| r.get(0),
        )
        .optional()
        .map_err(error)?
        .flatten();
    let mut published = HashMap::new();
    let mut active = None;
    for (id, key) in groups(db, document)? {
        if Some(&id) == current.as_ref() {
            active = Some((key.clone(), id.clone()));
        }
        published.insert(key, id);
    }
    if let Some((key, id)) = active {
        published.insert(key, id);
    }
    Ok(published.into_values().collect())
}

pub fn would_lower(db: &Connection, document: &str, name: &str) -> Result<bool> {
    let previous = effective(db, document, "")?;
    let next = key(name);
    let order = |label: &str| {
        label.strip_prefix('v').map(|s| {
            s.split('.')
                .map(|p| (p.len(), p.to_owned()))
                .collect::<Vec<_>>()
        })
    };
    Ok(matches!((order(&previous),order(&next)),(Some(old),Some(new)) if new < old))
}

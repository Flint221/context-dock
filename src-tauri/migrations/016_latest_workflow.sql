-- Existing bytes and cloud resource identities remain recoverable. New tracking
-- stores version metadata; it does not grow a permanent document archive.
INSERT OR IGNORE INTO preferences(key,value) VALUES('workflow.latest_only','true');
UPDATE projects SET archive_enabled=0 WHERE (SELECT value FROM preferences WHERE key='workflow.latest_only')='true';
UPDATE archive_intents SET enabled=0,pending=0 WHERE (SELECT value FROM preferences WHERE key='workflow.latest_only')='true';
UPDATE cloud_projects SET enabled=0,paused=1,sync_requested=0,reconcile_requested=0 WHERE (SELECT value FROM preferences WHERE key='workflow.latest_only')='true';
UPDATE operations SET state='legacy_paused' WHERE kind IN ('archive','restore','capture') AND state NOT IN ('done','cancelled','restored') AND (SELECT value FROM preferences WHERE key='workflow.latest_only')='true';

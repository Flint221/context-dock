# ContextDock readable archive names

Included in the ContextDock 0.9.0 release alongside the project workspace tabs. The original naming change was validated and pushed separately before packaging was requested.

New finalized archives keep the original filename, extension and author version label. Family folders use readable sanitized document titles without UUID suffixes.

```text
Old Context Files/RAP Master Context/RAP_Master_Context_v5.md
Old Context Files/plans/RAP Master Context/RAP_Master_Context_v7.2.md
Old Context Files/RAP Master Context/Capture 3/RAP_Master_Context_v6.md
Old Context Files/RAP Master Context/Archive 2/RAP_Master_Context_v5.md
```

- Differently named replacements retain their exact source filename. Nested sources retain their original parent structure beneath Old Context Files.
- In-place edits and missing-source fallback copies use a Capture N directory. Capture 3 is the local snapshot sequence, not a claim that the author's v6 document is v3.
- Existing files and reserved operation destinations are preserved. Collisions choose Archive 2, Archive 3 and subsequent directories without changing the filename or overwriting anything.
- Original filenames are not truncated. Readable family directory names retain the existing sanitization and length bound, with protection for Windows reserved device names.
- Archive review and automatic replacement archival share the same destination planner. A changed destination invalidates an earlier review token. Execution still verifies hashes, file identity, guarded directories and no-overwrite renames.
- Restoring moved files uses their recorded original paths. Snapshot copies remain archived and never replace the active working file.
- Existing legacy archive receipts keep their stored paths so interrupted operations and later restores remain valid. This change does not bulk rename existing archives or modify project data. Internal journal IDs and temporary staging filenames remain UUID based.

Validation: all 200 native tests passed, with one pre-existing ignored test. All-target Clippy with warnings denied and rustfmt checks passed. Coverage includes exact names and decimal labels, nested folders, long filenames, Windows reserved names, disk and journal collisions, snapshot copies, legacy UUID recovery and archive/restore round trips. Tests use temporary fixtures; no live project files or application databases were changed for this implementation.

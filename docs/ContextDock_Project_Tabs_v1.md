# ContextDock project workspace tabs

Version 0.9.0 keeps the current project name visible in a sticky header beside Files & tracking, Cloud sync and Ask AI. The header wraps at smaller window widths. Arrow keys, Home and End move tab focus; Enter or Space activates the focused tab.

Files & tracking begins with a full-width overview showing saved selected files, successfully captured selected document families, cloud indexed/expected versions and local missing-file/review/operation issues. A family can contain several selected versioned exports, so selected-file and tracked-document counts can differ. Saved history is retained while tracking is off. Cloud fractions use the configured current/history policy. Counts do not preview unapplied checkbox edits. Red indicates failures, amber indicates indexing or waiting, green indicates completed indexing and neutral indicates cloud off, paused or unavailable status. Text accompanies every color.

The folder selector precedes tracking/archival controls, document families/history and the discovered document list. Cloud sync contains connection/indexing status and cloud settings. Ask AI contains saved chats and questions. Panels stay mounted during tab changes; no mutations, uploads, paid questions or consent changes are triggered by changing tabs. Existing folder-choice save/discard prompts also protect tab navigation. A scan refreshes saved selection coverage without replacing an unsaved draft. Folder loss remains actionable across tabs.

Validation: 136 frontend tests across 16 files and 200 native tests passed, with one existing intentionally ignored native child fixture. TypeScript, ESLint, rustfmt, all-target Clippy with warnings denied and production frontend compilation passed. New coverage checks tab separation, active project heading, keyboard activation, save-before-switch, retained checkbox state, retained typed AI questions, single cloud subscription, status colors/counts and refresh protection for unsaved choices.

Browser fixture verification used Edge with isolated in-memory registry/cloud/chat responses at 1180x780 and 780x560. The header remained at y=0 after scrolling; tab panels and minimum-width layout passed without page errors or horizontal overflow. Files and Cloud screenshots were inspected. This does not claim installed upgrade acceptance or authenticated cloud/API testing. No real project files, local application databases or cloud resources were modified by these checks.

Manual acceptance after installation: open a project; scroll and switch all three tabs; check saved-selection and indexing counts; edit a folder checkbox and choose Keep editing, Save changes and continue, and Discard changes; type an AI question and switch tabs without sending it; verify paused/off/error statuses; check both themes and a narrow window; confirm readable archive naming after a reviewed archive operation.

Schema 010 and application identity are unchanged. The installer also includes the previously validated unified folder selection and readable archive naming changes already on this branch.

Release: feature commit `4acd2ba` was pushed and merged into main as `b3f56c5`. The 0.9.0 x64 NSIS installer was then built successfully from that merged tree and its version, size and SHA-256 were verified. See [the release verification record](verification.md). Installation was left to the user.

# ContextDock document and resource pilot

Step 5 development evidence separates local/native acceptance from the still-pending app-owned Google grant. The installed app, its database and its credential entry were not used.

## Isolated native app

The `pilot` Cargo feature and `tauri.pilot.conf.json` produce a distinct app identity and Drive credential target. The feature refuses to start under the production app identifier or without an explicitly marked absolute pilot directory. It opens only that directory's Data profile. Startup registration is disabled in this build. The ignored `profile::seed_isolated_pilot` test creates disposable Markdown and valid DOCX packages and refuses to overwrite an existing Data directory.

Build with `npx tauri build --no-bundle --features pilot --config src-tauri/tauri.pilot.conf.json --ci`. Set `CONTEXTDOCK_PILOT_DIR` only for the pilot process, with a `ContextDock_Pilot_v1.txt` marker at its root. This configuration is for development acceptance, not a production installer.

Observed in the actual release-mode native interface: four simultaneous Markdown/Word v1/v2 source files produced two latest-document rows, both v2. Adding Markdown v3 updated the existing row automatically. Closing to the tray, adding v4 and moving Word v2 into a subfolder preserved two rows and displayed the new location on reopening. Exiting and restarting retained the v4, Word location and monitoring setting. Reopening after memory conservation restored a usable latest-document interface.

## Publication and recovery

A new combined native fixture uses valid DOCX and Markdown, simultaneous numbered versions, moves, renames, an offline upload and a registry restart. Both families retain their cloud IDs, exact readback hashes and one cloud file per family in the provider simulator. Existing tests separately cover a lost response after the remote write and pending recovery under the same ID.

Three loopback HTTP tests exercise the real Drive adapter: resumable POST/create and PATCH/update initiation, raw PUT bytes for Markdown and DOCX, conditional update headers, exact downloaded hashes, provider retry/auth/conflict/missing classification, and rejection of an untrusted upload-session URL. The loopback override is compiled only for tests; production endpoints remain fixed Google HTTPS URLs. These tests do not establish Google's live behavior or authorization.

Stable monitoring keeps metadata rather than immutable snapshot copies. Upload staging and verification use bounded buffers. An unsuccessful staging operation now cleans up its newly created private temporary copy before it can become a journal reference; journaled copies remain recoverable. Settings polling is limited to an active sign-in. Hidden UI subscriptions defer their reads until the window is visible, while native monitoring and publication continue.

## Resource observations

The ARM64 Windows laptop has 12 logical processors and about 15.61 GiB RAM. Each sample spans about 31 seconds and includes the native process plus its six WebView2 descendants. CPU is normalized across all 12 processors; private bytes are summed across the same process tree. Summed working sets include shared pages and must not be treated as unique physical RAM.

| Condition | Total machine CPU | Ending private memory | Ending summed working sets |
| --- | ---: | ---: | ---: |
| Visible latest list, two families/four sources, Drive disconnected | 0.147% | 201.82 MiB | 476.11 MiB |
| Hidden in tray, two families/five sources, Drive disconnected | 0.025% | 199.23 MiB | 475.71 MiB |
| Minimized with low-memory request, two families/six sources, Drive disconnected | 0.038% | 189.92 MiB | 90.45 MiB |

These short samples differ in source count, warm-up and UI lifecycle and do not establish a percentage improvement against the installed version. The last run observed a stable process set. `scripts/ContextDock_Resource_Probe_v1.ps1` provides a repeatable pilot-only probe and preserves previous JSON reports. Raw observations remain in the local development artifact folder.

Windows WebView2's low-memory policy is requested when hidden/minimized and restored to normal on show/focus. It is best effort on older runtimes; native workers continue. Microsoft's [memory-target documentation](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2.memoryusagetargetlevel) describes this as a target, not a guaranteed memory limit. Lower working sets can involve paging; this is not a measured reduction in committed memory of the same magnitude.

The laptop reported external power, with charge rising from 78% to 80% across the pilot. Battery discharge, watt-hours, thermal behavior and long-duration savings were not measured. Battery-saver policy tests confirm that the default policy pauses new publication writes while local tracking continues, respects the persisted override and manual pause, and resumes eligibility without discarding pending work. Actual battery-saver transition with a live upload remains part of live acceptance.

## Checks and remaining acceptance

186 frontend tests and 312 Rust tests passed, with six default opt-in integrations excluded. The disposable Windows native tray/tracking/reopen/exit test was also run explicitly and passed. The isolated seed test passed. ESLint, production frontend build, formatting, and Clippy with warnings denied passed; the pilot feature also passed Clippy.

No ContextDock Desktop OAuth client exists yet. Live sign-in, the native folder picker, folder reuse visibility under `drive.file`, actual app-owned Markdown/Word transfer, disconnect/restart recovery and a fresh ChatGPT connected-Drive readback are still required. The prior connected-tool test is recorded separately in ContextDock_Drive_Test_Report_v1.md. Cleanup remains off during this pilot. Step 6 must keep recycling opt-in and must not interpret these local tests as live-cloud acceptance.

//! Local, on-demand Windows diagnostics. No registry database lock or polling thread.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    protocol: String,
    address: String,
    port: u16,
    scope: u32,
}
impl Endpoint {
    fn preview_url(&self, secure: bool) -> Result<String, String> {
        let ip: std::net::IpAddr = self.address.parse().map_err(|_| "Invalid address.")?;
        if self.protocol != "TCP" || self.port == 0 || !(ip.is_loopback() || ip.is_unspecified()) {
            return Err("Preview is available only for local TCP listeners.".into());
        }
        let host = match ip {
            std::net::IpAddr::V4(ip) => {
                if ip.is_unspecified() {
                    "127.0.0.1".into()
                } else {
                    ip.to_string()
                }
            }
            std::net::IpAddr::V6(ip) => {
                if ip.is_unspecified() {
                    "[::1]".into()
                } else {
                    format!("[{ip}]")
                }
            }
        };
        Ok(format!(
            "{}://{host}:{}/",
            if secure { "https" } else { "http" },
            self.port
        ))
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Process {
    pid: u32,
    parent_pid: u32,
    name: String,
    path: String,
    started: String,
    started_at: Option<String>,
    can_stop: bool,
    protection: String,
    endpoints: Vec<Endpoint>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    processes: Vec<Process>,
    warnings: Vec<String>,
    scanned_at: String,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pid: u32,
    started: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StopResult {
    pid: u32,
    stopped: bool,
    message: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupItem {
    name: String,
    source: String,
}

#[tauri::command]
pub async fn process_inventory() -> Result<Inventory, String> {
    tauri::async_runtime::spawn_blocking(platform::inventory)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn stop_processes(targets: Vec<Target>) -> Result<Vec<StopResult>, String> {
    if targets.is_empty() || targets.len() > 256 {
        return Err("Select between 1 and 256 processes.".into());
    }
    tauri::async_runtime::spawn_blocking(move || platform::stop(targets))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn preview_process(
    target: Target,
    endpoint: Endpoint,
    secure: bool,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = endpoint.preview_url(secure)?;
        let inventory = platform::inventory()?;
        if !inventory.processes.iter().any(|p| {
            p.pid == target.pid && p.started == target.started && p.endpoints.contains(&endpoint)
        }) {
            return Err("This listener changed or closed. Refresh before previewing.".into());
        }
        platform::open(&url)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn process_startup_items() -> Result<Vec<StartupItem>, String> {
    tauri::async_runtime::spawn_blocking(platform::startup)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn open_startup_settings() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| platform::open("ms-settings:startupapps"))
        .await
        .map_err(|e| e.to_string())?
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::{
        collections::{HashMap, HashSet},
        mem::{offset_of, size_of},
        ptr::{null, null_mut},
    };
    use windows_sys::Win32::{
        Foundation::*,
        NetworkManagement::IpHelper::*,
        Networking::WinSock::{AF_INET, AF_INET6},
        Security::{EqualSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER},
        System::{
            Diagnostics::ToolHelp::*, Registry::*, RemoteDesktop::ProcessIdToSessionId,
            Threading::*,
        },
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    };
    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    fn err() -> String {
        std::io::Error::last_os_error().to_string()
    }
    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    fn string(s: &[u16]) -> String {
        String::from_utf16_lossy(&s[..s.iter().position(|c| *c == 0).unwrap_or(s.len())])
    }
    fn process_handle(pid: u32, access: u32) -> Result<Handle, String> {
        let h = unsafe { OpenProcess(access, 0, pid) };
        if h.is_null() {
            Err(err())
        } else {
            Ok(Handle(h))
        }
    }
    fn birth(h: HANDLE) -> Result<u64, String> {
        let mut c = FILETIME::default();
        let mut e = c;
        let mut k = c;
        let mut u = c;
        if unsafe { GetProcessTimes(h, &mut c, &mut e, &mut k, &mut u) } == 0 {
            return Err(err());
        }
        Ok(((c.dwHighDateTime as u64) << 32) | c.dwLowDateTime as u64)
    }
    fn token(h: HANDLE) -> Result<Vec<usize>, String> {
        let mut t = null_mut();
        if unsafe { OpenProcessToken(h, TOKEN_QUERY, &mut t) } == 0 {
            return Err(err());
        }
        let t = Handle(t);
        let mut needed = 0;
        unsafe {
            GetTokenInformation(t.0, TokenUser, null_mut(), 0, &mut needed);
        }
        if needed == 0 {
            return Err(err());
        }
        let mut data = vec![0usize; (needed as usize).div_ceil(size_of::<usize>())];
        if unsafe {
            GetTokenInformation(
                t.0,
                TokenUser,
                data.as_mut_ptr().cast(),
                needed,
                &mut needed,
            )
        } == 0
        {
            return Err(err());
        }
        Ok(data)
    }
    fn same_user(h: HANDLE) -> bool {
        let (Ok(a), Ok(b)) = (token(h), token(unsafe { GetCurrentProcess() })) else {
            return false;
        };
        unsafe {
            EqualSid(
                (*(a.as_ptr() as *const TOKEN_USER)).User.Sid,
                (*(b.as_ptr() as *const TOKEN_USER)).User.Sid,
            ) != 0
        }
    }
    fn entries() -> Result<Vec<PROCESSENTRY32W>, String> {
        let h = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if h == INVALID_HANDLE_VALUE {
            return Err(err());
        }
        let h = Handle(h);
        let mut row = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut rows = Vec::new();
        if unsafe { Process32FirstW(h.0, &mut row) } == 0 {
            return Err(err());
        }
        loop {
            rows.push(row);
            if unsafe { Process32NextW(h.0, &mut row) } == 0 {
                if unsafe { GetLastError() } != ERROR_NO_MORE_FILES {
                    return Err(err());
                }
                break;
            }
        }
        Ok(rows)
    }
    // Protect the app's ancestor chain and descendants, including WebView helpers.
    fn protected(rows: &[PROCESSENTRY32W]) -> HashSet<u32> {
        let parents: HashMap<_, _> = rows
            .iter()
            .map(|p| (p.th32ProcessID, p.th32ParentProcessID))
            .collect();
        let me = std::process::id();
        let mut result = HashSet::from([0, 4, me]);
        let mut cursor = me;
        while let Some(parent) = parents.get(&cursor) {
            if !result.insert(*parent) {
                break;
            }
            cursor = *parent;
        }
        let mut children = HashSet::from([me]);
        loop {
            let previous = children.len();
            for (pid, parent) in &parents {
                if children.contains(parent) {
                    children.insert(*pid);
                }
            }
            if children.len() == previous {
                break;
            }
        }
        result.extend(children);
        result
    }
    fn describe(row: &PROCESSENTRY32W, guarded: &HashSet<u32>) -> Process {
        let pid = row.th32ProcessID;
        let mut p = Process {
            pid,
            parent_pid: row.th32ParentProcessID,
            name: string(&row.szExeFile),
            path: String::new(),
            started: String::new(),
            started_at: None,
            can_stop: false,
            protection: "Process exited or Windows denied access.".into(),
            endpoints: Vec::new(),
        };
        let Ok(h) = process_handle(pid, PROCESS_QUERY_LIMITED_INFORMATION) else {
            return p;
        };
        let Ok(started) = birth(h.0) else {
            return p;
        };
        p.started = started.to_string();
        let unix_ms = (started / 10_000) as i64 - 11_644_473_600_000;
        p.started_at = chrono::DateTime::from_timestamp_millis(unix_ms).map(|d| d.to_rfc3339());
        let mut image = vec![0u16; 32768];
        let mut length = image.len() as u32;
        if unsafe { QueryFullProcessImageNameW(h.0, 0, image.as_mut_ptr(), &mut length) } != 0 {
            p.path = String::from_utf16_lossy(&image[..length as usize]);
        }
        let mut session = 0;
        let mut current_session = 0;
        let mut critical = 1;
        let session_ok = unsafe {
            ProcessIdToSessionId(pid, &mut session) != 0
                && ProcessIdToSessionId(std::process::id(), &mut current_session) != 0
                && session != 0
                && session == current_session
        };
        let critical_ok = unsafe { IsProcessCritical(h.0, &mut critical) != 0 && critical == 0 };
        let windows = std::env::var("SystemRoot")
            .unwrap_or_else(|_| "C:\\Windows".into())
            .to_lowercase();
        p.protection = if guarded.contains(&pid) {
            "ContextDock, its host, or a related helper is protected."
        } else if !same_user(h.0) || !session_ok {
            "Windows services and other user sessions are protected."
        } else if !critical_ok
            || p.path.is_empty()
            || p.path.to_lowercase().starts_with(&(windows + "\\"))
        {
            "Windows system processes are protected."
        } else {
            ""
        }
        .into();
        p.can_stop = p.protection.is_empty();
        p
    }
    // Use aligned allocations, retry a growing table, and validate row count before reading.
    fn table<T: Copy>(tcp: bool, family: u16, offset: usize) -> Result<Vec<T>, String> {
        let mut size = 0u32;
        let fetch = |ptr, size: &mut u32| unsafe {
            if tcp {
                GetExtendedTcpTable(ptr, size, 0, family as u32, TCP_TABLE_OWNER_PID_LISTENER, 0)
            } else {
                GetExtendedUdpTable(ptr, size, 0, family as u32, UDP_TABLE_OWNER_PID, 0)
            }
        };
        let mut status = fetch(null_mut(), &mut size);
        for _ in 0..5 {
            if status != ERROR_INSUFFICIENT_BUFFER {
                return Err(format!("Port table unavailable (Windows {status})."));
            }
            if size as usize > 64 * 1024 * 1024 {
                return Err("Port table exceeded the safety limit.".into());
            }
            let mut data = vec![0usize; (size as usize).div_ceil(size_of::<usize>())];
            status = fetch(data.as_mut_ptr().cast(), &mut size);
            if status == 0 {
                let count = data.first().copied().unwrap_or(0) as u32 as usize;
                if offset.saturating_add(count.saturating_mul(size_of::<T>())) > size as usize {
                    return Err("Windows returned an incomplete port table.".into());
                }
                let bytes = data.as_ptr().cast::<u8>();
                return Ok((0..count)
                    .map(|i| unsafe {
                        std::ptr::read_unaligned(bytes.add(offset + i * size_of::<T>()).cast::<T>())
                    })
                    .collect());
            }
        }
        Err("Port table kept changing. Refresh to retry.".into())
    }
    fn listeners() -> (HashMap<u32, Vec<Endpoint>>, Vec<String>) {
        let mut all: HashMap<u32, Vec<Endpoint>> = HashMap::new();
        let mut warnings = Vec::new();
        let mut add = |pid, protocol: &str, address, port, scope| {
            all.entry(pid).or_default().push(Endpoint {
                protocol: protocol.into(),
                address,
                port: u16::from_be(port as u16),
                scope,
            });
        };
        macro_rules! rows { ($ty:ty, $table:ty, $tcp:expr, $af:expr, $label:expr, $row:ident, $body:block) => {
            match table::<$ty>($tcp, $af, offset_of!($table, table)) { Ok(rows) => for $row in rows $body, Err(e) => warnings.push(format!("{}: {e}", $label)) }
        }; }
        rows!(
            MIB_TCPROW_OWNER_PID,
            MIB_TCPTABLE_OWNER_PID,
            true,
            AF_INET,
            "TCP IPv4",
            r,
            {
                add(
                    r.dwOwningPid,
                    "TCP",
                    std::net::Ipv4Addr::from(r.dwLocalAddr.to_ne_bytes()).to_string(),
                    r.dwLocalPort,
                    0,
                );
            }
        );
        rows!(
            MIB_TCP6ROW_OWNER_PID,
            MIB_TCP6TABLE_OWNER_PID,
            true,
            AF_INET6,
            "TCP IPv6",
            r,
            {
                add(
                    r.dwOwningPid,
                    "TCP",
                    std::net::Ipv6Addr::from(r.ucLocalAddr).to_string(),
                    r.dwLocalPort,
                    r.dwLocalScopeId,
                );
            }
        );
        rows!(
            MIB_UDPROW_OWNER_PID,
            MIB_UDPTABLE_OWNER_PID,
            false,
            AF_INET,
            "UDP IPv4",
            r,
            {
                add(
                    r.dwOwningPid,
                    "UDP",
                    std::net::Ipv4Addr::from(r.dwLocalAddr.to_ne_bytes()).to_string(),
                    r.dwLocalPort,
                    0,
                );
            }
        );
        rows!(
            MIB_UDP6ROW_OWNER_PID,
            MIB_UDP6TABLE_OWNER_PID,
            false,
            AF_INET6,
            "UDP IPv6",
            r,
            {
                add(
                    r.dwOwningPid,
                    "UDP",
                    std::net::Ipv6Addr::from(r.ucLocalAddr).to_string(),
                    r.dwLocalPort,
                    r.dwLocalScopeId,
                );
            }
        );
        for endpoints in all.values_mut() {
            endpoints.sort();
            endpoints.dedup();
        }
        (all, warnings)
    }
    pub fn inventory() -> Result<Inventory, String> {
        let rows = entries()?;
        let guarded = protected(&rows);
        let (mut ports, warnings) = listeners();
        let mut processes = Vec::new();
        for row in rows {
            let mut p = describe(&row, &guarded);
            p.endpoints = ports.remove(&p.pid).unwrap_or_default();
            // Include all stoppable user apps as well as every listener owner.
            if p.can_stop || !p.endpoints.is_empty() {
                processes.push(p);
            }
        }
        for (pid, endpoints) in ports {
            processes.push(Process {
                pid,
                parent_pid: 0,
                name: "Unavailable process".into(),
                path: String::new(),
                started: String::new(),
                started_at: None,
                can_stop: false,
                protection: "Owner exited or its identity is unavailable. Refresh to retry.".into(),
                endpoints,
            });
        }
        processes.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then(a.pid.cmp(&b.pid))
        });
        Ok(Inventory {
            processes,
            warnings,
            scanned_at: chrono::Utc::now().to_rfc3339(),
        })
    }
    fn stop_one(
        t: &Target,
        rows: &[PROCESSENTRY32W],
        guarded: &HashSet<u32>,
    ) -> Result<(), String> {
        let row = rows
            .iter()
            .find(|r| r.th32ProcessID == t.pid)
            .ok_or("Already exited. Refresh the list.")?;
        // Retain one handle across identity validation, policy checks and termination.
        let h = process_handle(
            t.pid,
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | SYNCHRONIZATION_SYNCHRONIZE,
        )?;
        if birth(h.0)?.to_string() != t.started || t.started.is_empty() {
            return Err(
                "Process identity changed. Nothing was stopped; refresh and review again.".into(),
            );
        }
        let p = describe(row, guarded);
        if !p.can_stop || p.started != t.started {
            return Err(if p.protection.is_empty() {
                "Process identity changed.".into()
            } else {
                p.protection
            });
        }
        if unsafe { TerminateProcess(h.0, 1) } == 0 {
            return Err(err());
        }
        if unsafe { WaitForSingleObject(h.0, 2000) } != WAIT_OBJECT_0 {
            return Err(
                "Termination requested, but exit is not confirmed yet. Refresh to check.".into(),
            );
        }
        Ok(())
    }
    pub fn stop(targets: Vec<Target>) -> Result<Vec<StopResult>, String> {
        let rows = entries()?;
        let guarded = protected(&rows);
        let mut seen = HashSet::new();
        Ok(targets
            .into_iter()
            .filter(|t| seen.insert(t.pid))
            .map(|t| {
                let result = stop_one(&t, &rows, &guarded);
                StopResult {
                    pid: t.pid,
                    stopped: result.is_ok(),
                    message: result.err().unwrap_or_else(|| "Stopped.".into()),
                }
            })
            .collect())
    }
    pub fn open(url: &str) -> Result<(), String> {
        // Callers construct the entire URI; never accept arbitrary shell arguments.
        let result = unsafe {
            ShellExecuteW(
                null_mut(),
                wide("open").as_ptr(),
                wide(url).as_ptr(),
                null(),
                null(),
                SW_SHOWNORMAL,
            )
        } as isize;
        if result <= 32 {
            Err(format!(
                "Windows could not open the browser or settings (code {result})."
            ))
        } else {
            Ok(())
        }
    }
    pub fn startup() -> Result<Vec<StartupItem>, String> {
        let mut items = Vec::new();
        for (root, source) in [
            (HKEY_CURRENT_USER, "Your account"),
            (HKEY_LOCAL_MACHINE, "All users"),
        ] {
            for (view, label) in [(KEY_WOW64_64KEY, "64-bit"), (KEY_WOW64_32KEY, "32-bit")] {
                for key in ["Run", "RunOnce"] {
                    let name = wide(&format!(
                        "Software\\Microsoft\\Windows\\CurrentVersion\\{key}"
                    ));
                    let mut h = null_mut();
                    let status = unsafe {
                        RegOpenKeyExW(root, name.as_ptr(), 0, KEY_QUERY_VALUE | view, &mut h)
                    };
                    if status == ERROR_FILE_NOT_FOUND {
                        continue;
                    }
                    if status != 0 {
                        return Err(format!("Could not read {source} {key} ({status})."));
                    }
                    let result = (|| {
                        for index in 0..4096 {
                            let mut name = vec![0u16; 16384];
                            let mut length = name.len() as u32;
                            let code = unsafe {
                                RegEnumValueW(
                                    h,
                                    index,
                                    name.as_mut_ptr(),
                                    &mut length,
                                    null(),
                                    null_mut(),
                                    null_mut(),
                                    null_mut(),
                                )
                            };
                            if code == ERROR_NO_MORE_ITEMS {
                                return Ok(());
                            }
                            if code != 0 {
                                return Err(format!("Could not read a startup entry ({code})."));
                            }
                            items.push(StartupItem {
                                name: String::from_utf16_lossy(&name[..length as usize]),
                                source: format!("{source} · {key} · {label}"),
                            });
                        }
                        Err("Too many startup entries; inspection was incomplete.".into())
                    })();
                    unsafe {
                        RegCloseKey(h);
                    }
                    result?;
                }
            }
        }
        for (env, suffix, source) in [
            (
                "APPDATA",
                "Microsoft\\Windows\\Start Menu\\Programs\\Startup",
                "Your Startup folder",
            ),
            (
                "ProgramData",
                "Microsoft\\Windows\\Start Menu\\Programs\\Startup",
                "All users Startup folder",
            ),
        ] {
            let base = std::env::var_os(env).ok_or("Windows startup folder is unavailable.")?;
            let path = std::path::PathBuf::from(base).join(suffix);
            match std::fs::read_dir(path) {
                Ok(entries) => {
                    for entry in entries {
                        let entry = entry.map_err(|e| e.to_string())?;
                        if entry.file_name() != "desktop.ini" {
                            items.push(StartupItem {
                                name: entry.file_name().to_string_lossy().into_owned(),
                                source: source.into(),
                            });
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("Cannot read {source}: {e}")),
            }
        }
        items.sort_by(|a, b| a.name.cmp(&b.name).then(a.source.cmp(&b.source)));
        Ok(items)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn enumerates_all_four_tables_and_protects_self() {
            let tcp4 = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let tcp6 = std::net::TcpListener::bind("[::1]:0").unwrap();
            let udp4 = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
            let udp6 = std::net::UdpSocket::bind("[::1]:0").unwrap();
            let inv = inventory().unwrap();
            assert!(inv.warnings.is_empty(), "{:?}", inv.warnings);
            let me = inv
                .processes
                .iter()
                .find(|p| p.pid == std::process::id())
                .unwrap();
            assert!(!me.can_stop);
            for (protocol, address) in [
                ("TCP", tcp4.local_addr().unwrap()),
                ("TCP", tcp6.local_addr().unwrap()),
                ("UDP", udp4.local_addr().unwrap()),
                ("UDP", udp6.local_addr().unwrap()),
            ] {
                assert!(me.endpoints.iter().any(|e| e.protocol == protocol
                    && e.port == address.port()
                    && e.address == address.ip().to_string()));
            }
            let results = stop(vec![Target {
                pid: me.pid,
                started: me.started.clone(),
            }])
            .unwrap();
            assert!(!results[0].stopped);
        }
        #[test]
        fn protection_includes_ancestors_and_only_own_descendants() {
            let me = std::process::id();
            let row = |pid, parent| PROCESSENTRY32W {
                th32ProcessID: pid,
                th32ParentProcessID: parent,
                ..Default::default()
            };
            let guarded = protected(&[
                row(me, 11),
                row(11, 12),
                row(12, 0),
                row(21, me),
                row(22, 21),
                row(31, 11),
            ]);
            for pid in [0, 4, me, 11, 12, 21, 22] {
                assert!(guarded.contains(&pid));
            }
            assert!(!guarded.contains(&31));
        }
        #[test]
        #[ignore = "Invoked only by the disposable child-process integration test"]
        fn owned_listener_fixture() {
            let path = std::env::var_os("CONTEXTDOCK_PROCESS_TEST_READY")
                .expect("Fixture requires its temporary readiness path");
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            std::fs::write(path, listener.local_addr().unwrap().port().to_string()).unwrap();
            loop {
                std::thread::park_timeout(std::time::Duration::from_secs(30));
            }
        }
        #[test]
        fn rejects_reused_identity_and_confirms_exit_of_owned_fixture_only() {
            let dir = tempfile::tempdir().unwrap();
            let ready = dir.path().join("ready");
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "processes::platform::tests::owned_listener_fixture",
                    "--ignored",
                ])
                .env("CONTEXTDOCK_PROCESS_TEST_READY", &ready)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            struct Child(std::process::Child);
            impl Drop for Child {
                fn drop(&mut self) {
                    let _ = self.0.kill();
                    let _ = self.0.wait();
                }
            }
            let mut child = Child(child);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            while !ready.exists() {
                assert!(
                    std::time::Instant::now() < deadline,
                    "Disposable fixture did not start"
                );
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            let pid = child.0.id();
            let rows = entries().unwrap();
            let guarded = protected(&rows);
            let p = describe(
                rows.iter().find(|p| p.th32ProcessID == pid).unwrap(),
                &HashSet::new(),
            );
            assert!(p.can_stop);
            assert!(stop_one(
                &Target {
                    pid,
                    started: "stale".into()
                },
                &rows,
                &HashSet::new()
            )
            .is_err());
            assert!(stop_one(
                &Target {
                    pid,
                    started: p.started.clone()
                },
                &rows,
                &guarded
            )
            .is_err());
            assert!(child.0.try_wait().unwrap().is_none());
            // Only this test-owned child bypasses the app-descendant guard. All other checks are production code.
            stop_one(
                &Target {
                    pid,
                    started: p.started,
                },
                &rows,
                &HashSet::new(),
            )
            .unwrap();
            assert!(child.0.wait().unwrap().code().is_some());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_is_strictly_local_and_constructed_without_shell_input() {
        let endpoint = |address: &str| Endpoint {
            protocol: "TCP".into(),
            address: address.into(),
            port: 5173,
            scope: 0,
        };
        for (address, host) in [
            ("0.0.0.0", "127.0.0.1"),
            ("127.1.2.3", "127.1.2.3"),
            ("::", "[::1]"),
            ("::1", "[::1]"),
        ] {
            assert_eq!(
                endpoint(address).preview_url(false).unwrap(),
                format!("http://{host}:5173/")
            );
            assert_eq!(
                endpoint(address).preview_url(true).unwrap(),
                format!("https://{host}:5173/")
            );
        }
        for address in [
            "192.168.1.2",
            "example.com",
            "127.0.0.1/;calc.exe",
            "fe80::1",
        ] {
            assert!(endpoint(address).preview_url(false).is_err());
        }
        let mut e = endpoint("127.0.0.1");
        e.protocol = "UDP".into();
        assert!(e.preview_url(false).is_err());
        e.protocol = "TCP".into();
        e.port = 0;
        assert!(e.preview_url(false).is_err());
    }
}

#[cfg(not(windows))]
mod platform {
    use super::*;
    pub fn inventory() -> Result<Inventory, String> {
        Err("Process diagnostics are available on Windows.".into())
    }
    pub fn stop(_: Vec<Target>) -> Result<Vec<StopResult>, String> {
        Err("Process controls are available on Windows.".into())
    }
    pub fn open(_: &str) -> Result<(), String> {
        Err("Available on Windows.".into())
    }
    pub fn startup() -> Result<Vec<StartupItem>, String> {
        Err("Startup inspection is available on Windows.".into())
    }
}

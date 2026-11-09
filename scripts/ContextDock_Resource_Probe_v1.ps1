param(
    [Parameter(Mandatory)][int]$RootProcessId,
    [Parameter(Mandatory)][string]$Output,
    [Parameter(Mandatory)][string]$Mode
)
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath $Output) { throw 'Choose a new report filename; existing reports are preserved.' }
$rootProcess = Get-Process -Id $RootProcessId
if ($rootProcess.ProcessName -notlike 'ContextDock_Pilot_v*') { throw 'This probe is restricted to an isolated ContextDock pilot.' }
$logicalProcessors = (Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
$samples = @()
$sets = @()
for ($sampleIndex = 0; $sampleIndex -lt 7; $sampleIndex++) {
    $procInfo = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId
    $ids = [System.Collections.Generic.HashSet[int]]::new()
    [void]$ids.Add($RootProcessId)
    do {
        $before = $ids.Count
        foreach ($entry in $procInfo) {
            if ($ids.Contains([int]$entry.ParentProcessId)) { [void]$ids.Add([int]$entry.ProcessId) }
        }
    } while ($ids.Count -gt $before)
    $sets += (($ids | Sort-Object) -join ',')
    $procs = @(foreach ($taskProcessId in $ids) { Get-Process -Id $taskProcessId -ErrorAction SilentlyContinue })
    if ((Get-Process -Id $RootProcessId).StartTime -ne $rootProcess.StartTime) { throw 'The pilot restarted during measurement.' }
    $samples += [pscustomobject]@{
        Utc = [DateTime]::UtcNow.ToString('o')
        Processes = $procs.Count
        CpuSeconds = ($procs | Measure-Object -Property CPU -Sum).Sum
        PrivateBytes = ($procs | Measure-Object -Property PrivateMemorySize64 -Sum).Sum
        WorkingSetBytes = ($procs | Measure-Object -Property WorkingSet64 -Sum).Sum
        NativeCpuSeconds = (Get-Process -Id $RootProcessId).CPU
    }
    if ($sampleIndex -lt 6) { Start-Sleep -Seconds 5 }
}
$elapsed = ([datetime]$samples[-1].Utc - [datetime]$samples[0].Utc).TotalSeconds
$stable = @($sets | Select-Object -Unique).Count -eq 1
$cpuSeconds = if ($stable) { $samples[-1].CpuSeconds - $samples[0].CpuSeconds } else { $null }
$batteries = @(Get-CimInstance Win32_Battery | Select-Object BatteryStatus, EstimatedChargeRemaining)
$result = [pscustomobject]@{
    Mode = $Mode
    LogicalProcessors = $logicalProcessors
    StableProcessSet = $stable
    ElapsedSeconds = $elapsed
    CpuSeconds = $cpuSeconds
    OneCorePercent = if ($stable) { 100 * $cpuSeconds / $elapsed } else { $null }
    MachinePercent = if ($stable) { 100 * $cpuSeconds / $elapsed / $logicalProcessors } else { $null }
    Batteries = $batteries
    Samples = $samples
}
$result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $Output
$result | Select-Object Mode, StableProcessSet, ElapsedSeconds, CpuSeconds, OneCorePercent, MachinePercent, Batteries | ConvertTo-Json -Depth 3 -Compress

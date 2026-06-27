param(
  [string]$client = "codex",
  [string]$stateDir = "",
  [Parameter(ValueFromRemainingArguments = $true)]
  [string[]]$remainingArgs = @()
)

$ErrorActionPreference = "Stop"
$MAX_FOCUS_ATTEMPTS = 3
$FOCUS_RETRY_DELAY_MS = 500
$FOCUS_SETTLE_DELAY_MS = 350
$CLIPBOARD_SETTLE_DELAY_MS = 180
$ENTER_AFTER_PASTE_DELAY_MS = 320
$DELAYED_SEND_WAIT_MS = 700
$KEY_UP_FLAG = 0x0002
$VK_CONTROL = 0x11
$VK_V = 0x56
$VK_ENTER = 0x0D

function Write-PromptFlowLog {
  param([string]$message)
  try {
    if (-not $script:stateDir) { return }
    $logPath = Join-Path $script:stateDir "flow-hook.log"
    $line = "$(Get-Date -Format o) [$client] $message"
    Add-Content -LiteralPath $logPath -Value $line -Encoding UTF8
  } catch {}
}

function Read-JsonFile {
  param([string]$path)
  if (-not (Test-Path -LiteralPath $path)) { return $null }
  $raw = Get-Content -LiteralPath $path -Raw -Encoding UTF8
  if (-not $raw.Trim()) { return $null }
  return $raw | ConvertFrom-Json
}

function Save-JsonFile {
  param([string]$path, [object]$value)
  $json = $value | ConvertTo-Json -Depth 100
  Set-Content -LiteralPath $path -Value $json -Encoding UTF8
}

function Get-Prop {
  param([object]$value, [string[]]$names)
  if ($null -eq $value) { return $null }
  foreach ($name in $names) {
    if ($value.PSObject.Properties.Name -contains $name) {
      return $value.$name
    }
  }
  return $null
}

function Set-Prop {
  param([object]$value, [string]$name, [object]$newValue)
  if ($value.PSObject.Properties.Name -contains $name) {
    $value.$name = $newValue
  } else {
    $value | Add-Member -NotePropertyName $name -NotePropertyValue $newValue -Force
  }
}

Add-Type @"
using System;
using System.Runtime.InteropServices;

public static class PromptFlowWin32 {
  [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, uint dwFlags, UIntPtr dwExtraInfo);
}
"@

function Stop-FlowWithError {
  param([object]$run, [string]$activePath, [string]$message)
  $run.status = "error"
  Set-Prop $run "error" $message
  Set-Prop $run "updated_at" (Get-Date -Format o)
  Save-JsonFile $activePath $run
  Write-PromptFlowLog $message
  "{}"
  exit 0
}

function Pause-Flow {
  param([object]$run, [string]$activePath, [string]$message)
  $run.status = "paused"
  Set-Prop $run "error" $message
  Set-Prop $run "updated_at" (Get-Date -Format o)
  Save-JsonFile $activePath $run
  Write-PromptFlowLog $message
  "{}"
  exit 0
}

function Focus-TargetWindow {
  param([Int64]$hwndValue)

  if ($hwndValue -le 0) {
    return "missing"
  }

  $hwnd = [IntPtr]$hwndValue
  if (-not [PromptFlowWin32]::IsWindow($hwnd)) {
    return "closed"
  }

  if ([PromptFlowWin32]::IsIconic($hwnd)) {
    [void][PromptFlowWin32]::ShowWindow($hwnd, 9)
  }

  [void][PromptFlowWin32]::BringWindowToTop($hwnd)
  [void][PromptFlowWin32]::SetForegroundWindow($hwnd)
  Start-Sleep -Milliseconds $FOCUS_SETTLE_DELAY_MS

  $focused = [PromptFlowWin32]::GetForegroundWindow()
  if ($focused.ToInt64() -ne $hwnd.ToInt64()) {
    return "blocked"
  }
  return "focused"
}

function Send-PromptToTarget {
  param([Int64]$targetHwnd, [string]$prompt)

  # Same idea as cockpit-tools: resolve/focus the saved window, but keep retries local and short.
  for ($attempt = 1; $attempt -le $MAX_FOCUS_ATTEMPTS; $attempt++) {
    $focusResult = Focus-TargetWindow $targetHwnd
    if ($focusResult -eq "focused") { break }
    if ($focusResult -in @("missing", "closed")) { return $focusResult }
    if ($attempt -lt $MAX_FOCUS_ATTEMPTS) { Start-Sleep -Milliseconds $FOCUS_RETRY_DELAY_MS }
  }
  if ($focusResult -ne "focused") { return "focus_failed" }

  Set-Clipboard -Value $prompt
  Start-Sleep -Milliseconds $CLIPBOARD_SETTLE_DELAY_MS

  [PromptFlowWin32]::keybd_event($VK_CONTROL, 0, 0, [UIntPtr]::Zero)
  [PromptFlowWin32]::keybd_event($VK_V, 0, 0, [UIntPtr]::Zero)
  [PromptFlowWin32]::keybd_event($VK_V, 0, $KEY_UP_FLAG, [UIntPtr]::Zero)
  [PromptFlowWin32]::keybd_event($VK_CONTROL, 0, $KEY_UP_FLAG, [UIntPtr]::Zero)

  Start-Sleep -Milliseconds $ENTER_AFTER_PASTE_DELAY_MS
  [PromptFlowWin32]::keybd_event($VK_ENTER, 0, 0, [UIntPtr]::Zero)
  [PromptFlowWin32]::keybd_event($VK_ENTER, 0, $KEY_UP_FLAG, [UIntPtr]::Zero)
  return "sent"
}

function Read-LegacyArgs {
  param([string[]]$values)
  for ($i = 0; $i -lt $values.Count; $i++) {
    if ($values[$i] -eq "--client" -and ($i + 1) -lt $values.Count) {
      $script:client = [string]$values[$i + 1]
      $i++
      continue
    }
    if ($values[$i] -eq "--state-dir" -and ($i + 1) -lt $values.Count) {
      $script:stateDir = [string]$values[$i + 1]
      $i++
    }
  }
}

function Has-LegacyFlag {
  param([string[]]$values, [string]$flag)
  return $values -contains $flag
}

function Current-ScriptPath {
  if ($PSCommandPath) { return $PSCommandPath }
  return $MyInvocation.MyCommand.Path
}

function Start-DelayedSender {
  param([object]$run, [string]$activePath)

  $scriptPath = Current-ScriptPath
  if (-not $scriptPath) {
    Stop-FlowWithError $run $activePath "Could not find hook script path for delayed sender."
  }

  $args = @(
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    $scriptPath,
    "--send-pending",
    "--client",
    $client,
    "--state-dir",
    $script:stateDir
  )
  # Return from the Stop hook quickly; a detached process pastes the next step after the CLI is idle.
  Start-Process -FilePath "powershell.exe" -ArgumentList $args -WindowStyle Hidden | Out-Null
}

function Send-PendingStep {
  param([string]$activePath)

  # Let the CLI finish closing the Stop hook frame before we paste the next prompt.
  Start-Sleep -Milliseconds $DELAYED_SEND_WAIT_MS
  $run = Read-JsonFile $activePath
  if ($null -eq $run -or $run.status -ne "active") {
    "{}"
    exit 0
  }

  try {
    $currentStep = [int]$run.current_step
    $stepCount = @($run.steps).Count
    if ($currentStep -lt 0 -or $currentStep -ge $stepCount) {
      throw "Saved flow step is out of range."
    }

    $targetHwnd = [Int64](Get-Prop $run @("target_hwnd", "targetHwnd"))
    $prompt = [string]$run.steps[$currentStep]
    $sendResult = Send-PromptToTarget $targetHwnd $prompt
    if ($sendResult -eq "missing") {
      Stop-FlowWithError $run $activePath "Could not send pending flow step: no target window was saved."
    }
    if ($sendResult -eq "closed") {
      Stop-FlowWithError $run $activePath "Could not send pending flow step: target window was closed."
    }
    if ($sendResult -ne "sent") {
      Pause-Flow $run $activePath "Flow paused: target window could not be focused after 3 attempts."
    }
    Write-PromptFlowLog "Sent step $($currentStep + 1)/$stepCount to target window."
  } catch {
    Stop-FlowWithError $run $activePath "Could not send pending flow step: $($_.Exception.Message)"
  }

  "{}"
  exit 0
}

function Continue-Flow {
  param([object]$run, [string]$activePath)

  $nextStep = [int]$run.current_step + 1
  $stepCount = @($run.steps).Count

  if ($nextStep -ge $stepCount) {
    $run.status = "completed"
    Set-Prop $run "updated_at" (Get-Date -Format o)
    Save-JsonFile $activePath $run
    Write-PromptFlowLog "Completed flow $($run.title) / $($run.run_id)."
    "{}"
    exit 0
  }

  $nextPrompt = [string]$run.steps[$nextStep]
  if (-not $nextPrompt.Trim()) {
    $run.status = "completed"
    Set-Prop $run "updated_at" (Get-Date -Format o)
    Save-JsonFile $activePath $run
    "{}"
    exit 0
  }

  $run.current_step = $nextStep
  Set-Prop $run "updated_at" (Get-Date -Format o)
  Save-JsonFile $activePath $run

  Write-PromptFlowLog "Continuing flow $($run.title) with step $($nextStep + 1)/$stepCount."
  try {
    Start-DelayedSender $run $activePath
  } catch {
    Stop-FlowWithError $run $activePath "Could not launch delayed flow sender: $($_.Exception.Message)"
  }

  "{}"
  exit 0
}

try {
  # PowerShell binds --client as -client, but leaves --state-dir in remaining args.
  Read-LegacyArgs $remainingArgs

  if (-not $stateDir) {
    $stateDir = Join-Path $env:APPDATA "dev.promptflow.desktop"
  }
  $script:stateDir = $stateDir

  $activePath = Join-Path (Join-Path $stateDir "flow-runs") "active.json"
  if (Has-LegacyFlag $remainingArgs "--send-pending") {
    Send-PendingStep $activePath
  }

  $run = Read-JsonFile $activePath
  if ($null -eq $run -or $run.status -ne "active") {
    "{}"
    exit 0
  }

  $stdin = [Console]::In.ReadToEnd()
  $payload = $null
  if ($stdin.Trim()) {
    try { $payload = $stdin | ConvertFrom-Json } catch {}
  }

  $sessionId = [string](Get-Prop $payload @("session_id", "sessionId"))
  $transcriptPath = [string](Get-Prop $payload @("transcript_path", "transcriptPath"))
  $boundSession = [string]$run.session_id
  $boundTranscript = [string]$run.transcript_path

  $matchesRun =
    ($boundSession -and $sessionId -and $boundSession -eq $sessionId) -or
    ($boundTranscript -and $transcriptPath -and $boundTranscript -eq $transcriptPath) -or
    (-not $boundSession -and -not $boundTranscript)

  # Ignore Stop hooks from other Codex/Claude sessions once this flow is bound to one session.
  if (-not $matchesRun) {
    Write-PromptFlowLog "Ignored Stop hook; active run was not found in this session."
    "{}"
    exit 0
  }

  if (-not [string]$run.session_id -and $sessionId) {
    Set-Prop $run "session_id" $sessionId
  }
  if (-not [string]$run.transcript_path -and $transcriptPath) {
    Set-Prop $run "transcript_path" $transcriptPath
  }

  Continue-Flow $run $activePath
} catch {
  Write-PromptFlowLog "Hook failed: $($_.Exception.Message)"
  "{}"
  exit 0
}

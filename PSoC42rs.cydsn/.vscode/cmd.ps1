$PPCLIExecutable = "C:\Program Files (x86)\Cypress\Programmer\ppcli.exe"
# We will generate a temporary .cli file in the system temp directory
$TempCliPath = "J:\Projects\Psoc4Vscode\P4200\EncoderTest.cydsn\.vscode\temp.cli"

$commands = @"
OpenPort "MiniProg4 (CMSIS-DAP/BULK/0C07062303210400)" "C:\Program Files (x86)\Cypress\Programmer"
SetPowerVoltage 3.3
PowerOn
SetAcquireMode "Reset"
SetProtocol 8
SetProtocolClock 152
DAP_Acquire
GetRowsPerArrayInFlash
ClosePort
Quit
"@


# $Output = $commands | & "C:\Program Files (x86)\Cypress\Programmer\ppcli.exe"
# $Output | Out-Host

function Invoke-PPCLI-Script {
    param(
        [Parameter(Mandatory=$true)]
        [string]$CliContent
    )

    Write-Host "Executing PPCLI interactively..." -ForegroundColor Yellow
    $Output = $CliContent | & $PPCLIExecutable 2>&1
    $ExitCode = $LASTEXITCODE

    if ($ExitCode -ne 0) {
        Write-Error "🚨 PPCLI failed with Exit Code: $ExitCode"
        return $null
    }

    return $Output
}

& $PPCLIExecutable --runfile "$TempCliPath" --outputfile "$env:TEMP\ppcli_log.txt" --quit

param(
    [Parameter(Mandatory=$true)]
    [string]$HexFilePath
)

# --- Configuration ---
$PPCLIExecutable = "C:\Program Files (x86)\Cypress\Programmer\ppcli.exe"
# We will generate a temporary .cli file in the system temp directory
$TempCliPath = [System.IO.Path]::GetTempFileName() + ".cli"

# --- Dynamic CLI Content ---
# NOTE: Ensure forward slashes are used for the HEX file path inside the CLI command
$ForwardSlashHexPath = $HexFilePath -replace '\\', '/'
# J:\Projects\Psoc4Vscode\P4200\EncoderTest.cydsn\CortexM0\ARM_GCC_541\Debug\EncoderTest.hex
$CliContent = @(
"OpenPort `"MiniProg4 (CMSIS-DAP/BULK/0C07062303210400)`" `"C:\Program Files (x86)\Cypress\Programmer`""
"HEX_ReadFile `"$ForwardSlashHexPath`""
"SetPowerVoltage 3.3"
"PowerOn"
"SetAcquireMode `“Reset`”"
"SetProtocol 8"#swd
"SetProtocolClock 152"
"DAP_Acquire"
"Calibrate"
"EraseAll"
"Program"
"Verify"
"CheckSum 0"
"HEX_ReadChecksum"
"ClosePort"
# "SetAcquireMode `"Reset`""
# 
# "DAP_Acquire"
# "PSoC4_EraseAll"
# "PSoC4_ProgramAllFromHex"
# "DAP_ReleaseChip"
# "ClosePort"
"quit"
)

# Parametric CLI OpenPort "MiniProg4 (CMSIS-DAP/BULK/0C07062303210400)" "C:\Program Files (x86)\Cypress\Programmer" 
#HEX_ReadFile "J:/Projects/Psoc4Vscode/P4200/EncoderTest.cydsn/CortexM0/ARM_GCC_541/Debug/EncoderTest.hex"
# SetPowerVoltage 3.3 SetAcquireMode â€œResetâ€ Acquire Calibrate EraseAll Program Verify CheckSum 0 HEX_ReadChecksum ClosePort quit
Write-Host "Parametric CLI" $CliContent
# Write content to the temporary CLI file
$CliContent | Out-File $TempCliPath -Encoding UTF8

Write-Host "Starting PSoC programming using temporary script: $TempCliPath"

# --- Execution ---
$Arguments = "--runfile", "`"$TempCliPath`"", "--quit"
$Output = & $PPCLIExecutable $Arguments 2>&1

# --- Cleanup ---
Remove-Item $TempCliPath -Force

# --- Result Analysis ---
if ($LASTEXITCODE -ne 0) {
    # ... (Error handling as before) ...
    Write-Error "🚨 PPCLI Programming FAILED! Exit Code: $LASTEXITCODE"
    $Output | Out-Host
    exit 1
}
# ... (Success handling as before) ...
Write-Host "✅ PPCLI Programming completed successfully."
$Output | Out-Host
param(
    [Parameter(Mandatory=$true)]
    [string]$HexFilePath
)

# --- Configuration ---
$PPCLIExecutable = "C:\Program Files (x86)\Cypress\Programmer\ppcli.exe"
$ToolDir = "C:/Program Files (x86)/Cypress/Programmer" # Tool path for OpenPort command
$ProgrammerID = "MiniProg4 (CMSIS-DAP/BULK/0C07062303210400)" # REPLACE with your actual ID from GetPorts

# Convert path for ppcli.exe (must use forward slashes for file operations)
$ForwardSlashHexPath = $HexFilePath -replace '\\', '/'

# --- Function to Execute PPCLI with a Script and Get Output ---
function Invoke-PPCLI-Script {
    param(
        [Parameter(Mandatory=$true)]
        [string]$CliContent
    )
    $TempCliPath = [System.IO.Path]::GetTempFileName() + ".cli"
    
    # Write the content to a temporary file
    $CliContent | Out-File $TempCliPath -Encoding ASCII 

    # Execute the tool
    Write-Host "Executing PPCLI via temporary script: $TempCliPath" -ForegroundColor Yellow
    $Arguments = "--runfile", "`"$TempCliPath`"", "--quit"

    $Output = & $PPCLIExecutable $Arguments 2>&1
    $Output | Out-Host
    $ExitCode = $LASTEXITCODE

    # Cleanup
    Remove-Item $TempCliPath -Force

    # Check for failure in the tool's native exit code
    if ($ExitCode -ne 0) {
        Write-Error "🚨 PPCLI failed with Exit Code: $ExitCode"
        Write-Host "--- PPCLI Output (Error) ---"
        $Output | Out-Host
        return $null # Return null on failure
    }
    
    return $Output
}

## ------------------------------------------------------------------
## 1. GET ROW COUNT (UPDATED for GetRowsPerArrayInFlash)
# ------------------------------------------------------------------

Write-Host "`n--- STEP 1: Getting Row Count ---"

# 1. Define the CLI content using the new command
$CliCountContent = @(
    # Assumes OpenPort and HEX_ReadFile are necessary to initialize the device/state.
    # If this command doesn't rely on a specific HEX file, you might remove HEX_ReadFile.
    "OpenPort `"$ProgrammerID`" `"$ToolDir`""
    "SetPowerVoltage 3.3"
    "PowerOn"
    "SetAcquireMode `“Reset`”"
    "SetProtocol 8"#swd
    "SetProtocolClock 152"
    "DAP_Acquire"    
    "GetRowsPerArrayInFlash" # <-- The new command
    "ClosePort"
    "Quit"
)

# Execute the script
$CountOutput = Invoke-PPCLI-Script -CliContent ($CliCountContent -join "`n")

if (-not $CountOutput) {
    Write-Error "Failed to run the count script. Aborting. "
    exit 1
}

# 2. Search for the output line containing the command name
$RowCountLine = $CountOutput | Select-String -Pattern "GetRowsPerArrayInFlash"

if ($RowCountLine) {
    # 3. Use Regex to reliably extract the hexadecimal value (e.g., 0x00000100)
    # This looks for "0x" followed by 1 or more hex digits (A-F, 0-9)
    $Match = $RowCountLine -match '(0x[0-9a-fA-F]+)'
    
    if ($Match) {
        $HexValue = $matches[1]
        
        # 4. Convert the extracted hexadecimal string to a decimal integer
        # The [int] operator with the "0x" prefix automatically performs the conversion
        try {
            $Rows = [int]$HexValue 
            Write-Host "Detected $Rows rows per array ($HexValue). " -ForegroundColor Green
        }
        catch {
             Write-Error "Could not convert the hex value '$HexValue' to a number. "
             $Rows = 0
        }
    } else {
        Write-Error " Could not extract the hexadecimal row count (0x...) from output."
        $Rows = 0
    }
} else {
    Write-Error " Could not find the GetRowsPerArrayInFlash line in PPCLI output."
    $Rows = 0
}

if ($Rows -eq 0) {
    exit 1
}

# ------------------------------------------------------------------
## 2. DYNAMICALLY GENERATE PROGRAMMING SCRIPT
# ------------------------------------------------------------------

if ($Rows -eq 0) {
    Write-Host "No rows detected. Exiting."
    exit 0
}

Write-Host "--- STEP 2: Generating Program Script ---"

# Base commands for the start and end of the programming cycle
$CliProgramContent = @(
    "OpenPort `"$ProgrammerID`" `"$ToolDir`""
    "HEX_ReadFile `"$ForwardSlashHexPath`""
    "SetAcquireMode `"Reset`""
    "SetProtocol 8"
    "SetPowerVoltage 5.0" # Adjust voltage if needed
    "DAP_Acquire"
    "PSoC4_EraseAll"
)

# Loop to generate the commands for each row
for ($i = 0; $i -lt $Rows; $i++) {
    $CliProgramContent += "PSoC4_ProgramRowFromHex $i"
    $CliProgramContent += "PSoC4_VerifyRowFromHex $i"
}

# Base commands for the end of the programming cycle
$CliProgramContent += @(
    "DAP_ReleaseChip"
    "ClosePort"
    "Quit"
)

# ------------------------------------------------------------------
## 3. RUN PROGRAMMING SCRIPT
# ------------------------------------------------------------------

Write-Host "`n--- STEP 3: Programming Device (Total Commands: $($CliProgramContent.Count)) ---"

$ProgramOutput = Invoke-PPCLI-Script -CliContent ($CliProgramContent -join "n")`

# This is the IF block starting near Line 120
if (-not $ProgramOutput) {
    Write-Error " Device programming failed. See errors above."
    exit 1
} 
# <--- CRITICAL: Ensure this closing brace is present!

Write-Host "Programming and Verification COMPLETE. " -ForegroundColor Green
Write-Host "--- Full PPCLI Output from Programming Run --- " # Line 126 should be correct now
$ProgramOutput  |Out-Host
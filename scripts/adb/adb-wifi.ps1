param(
    [Parameter(Mandatory=$true)]
    [string]$PhoneIP
)

$adbPath = "$env:LOCALAPPDATA\Android\Sdk\platform-tools\adb.exe"
if (-not (Test-Path $adbPath)) {
    $adbPath = "C:\Users\aryan\AppData\Local\Android\Sdk\platform-tools\adb.exe"
}

if (-not (Test-Path $adbPath)) {
    Write-Host "ADB not found at $adbPath" -ForegroundColor Red
    Write-Host "Set ANDROID_HOME or provide adb.exe path manually." -ForegroundColor Yellow
    exit 1
}

Write-Host "ADB found at: $adbPath" -ForegroundColor Green

# Kill any existing ADB server
& $adbPath kill-server

# Start ADB server
& $adbPath start-server

# Connect over WiFi (port 5555 is the default ADB port)
Write-Host "Connecting to $PhoneIP:5555 ..." -ForegroundColor Yellow
$output = & $adbPath connect "$PhoneIP`:5555"

if ($output -match "connected") {
    Write-Host "SUCCESS: Connected to $PhoneIP" -ForegroundColor Green
    Write-Host ""
    Write-Host "Devices:" -ForegroundColor Cyan
    & $adbPath devices -l
    Write-Host ""
    Write-Host "Now open apps\mobile\ in Android Studio and click Run (Shift+F10)." -ForegroundColor Cyan
    Write-Host "It will deploy over WiFi. No USB cable needed." -ForegroundColor Cyan
} else {
    Write-Host "FAILED: $output" -ForegroundColor Red
    Write-Host ""
    Write-Host "Troubleshooting:" -ForegroundColor Yellow
    Write-Host "  1. On your phone, enable Developer options and USB debugging" -ForegroundColor Yellow
    Write-Host "  2. Connect via USB once and run: adb tcpip 5555" -ForegroundColor Yellow
    Write-Host "  3. Disconnect USB and run this script again" -ForegroundColor Yellow
    Write-Host "  4. Make sure phone and PC are on the same WiFi network" -ForegroundColor Yellow
}

param(
    [Parameter(Mandatory = $true, Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$InputPath,

    [string]$OutputDir = "",
    [int]$Crf = 18,
    [string]$Preset = "slow",
    [switch]$Overwrite,
    [switch]$OpenRadOnUnsupported
)

$ErrorActionPreference = "Stop"

function Resolve-ToolPath {
    param(
        [string]$Preferred,
        [string]$ExeName
    )

    if ($Preferred -and (Test-Path -LiteralPath $Preferred)) {
        return (Resolve-Path -LiteralPath $Preferred).Path
    }

    $cmd = Get-Command $ExeName -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }

    throw "Could not find $ExeName. Install ffmpeg or edit this script's preferred path."
}

function Get-OutputPath {
    param(
        [System.IO.FileInfo]$InputFile,
        [string]$OutputDir
    )

    $name = [System.IO.Path]::GetFileNameWithoutExtension($InputFile.Name) + ".mp4"
    if ([string]::IsNullOrWhiteSpace($OutputDir)) {
        return [System.IO.Path]::Combine($InputFile.DirectoryName, $name)
    }

    if (!(Test-Path -LiteralPath $OutputDir)) {
        New-Item -ItemType Directory -Path $OutputDir | Out-Null
    }
    return [System.IO.Path]::Combine((Resolve-Path -LiteralPath $OutputDir).Path, $name)
}

function Get-BinkSignature {
    param([string]$Path)

    $stream = [System.IO.File]::OpenRead($Path)
    try {
        $buffer = New-Object byte[] 4
        $read = $stream.Read($buffer, 0, 4)
        if ($read -lt 4) {
            return ""
        }
        return [System.Text.Encoding]::ASCII.GetString($buffer)
    }
    finally {
        $stream.Dispose()
    }
}

function Resolve-OptionalToolPath {
    param(
        [string]$Preferred,
        [string]$ExeName
    )

    if ($Preferred -and (Test-Path -LiteralPath $Preferred)) {
        return (Resolve-Path -LiteralPath $Preferred).Path
    }

    $cmd = Get-Command $ExeName -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }

    return $null
}

$preferredFfmpeg = "G:\ffmpeg\ffmpeg-8.0-essentials_build\bin\ffmpeg.exe"
$ffmpeg = Resolve-ToolPath -Preferred $preferredFfmpeg -ExeName "ffmpeg.exe"
$radvideo = Resolve-OptionalToolPath `
    -Preferred "G:\Modification\FSModify\Sound\CG修改\RADVideo\radvideo64.exe" `
    -ExeName "radvideo64.exe"

$converted = 0
$skipped = 0
foreach ($rawPath in $InputPath) {
    if ([string]::IsNullOrWhiteSpace($rawPath)) {
        continue
    }

    $resolved = Resolve-Path -LiteralPath $rawPath -ErrorAction Stop
    foreach ($path in $resolved) {
        $file = Get-Item -LiteralPath $path.Path
        if ($file.PSIsContainer) {
            Write-Warning "Skipping directory: $($file.FullName)"
            continue
        }
        if ($file.Extension.ToLowerInvariant() -ne ".bk2") {
            Write-Warning "Skipping non-BK2 file: $($file.FullName)"
            $skipped += 1
            continue
        }

        $signature = Get-BinkSignature -Path $file.FullName
        if ($signature.StartsWith("KB2")) {
            Write-Warning "Unsupported by ffmpeg: $($file.FullName)"
            Write-Warning "Detected Bink 2 signature '$signature'. ffmpeg usually cannot decode KB2* BK2 files."
            if ($OpenRadOnUnsupported -and $radvideo) {
                Write-Host "Opening RADVideo for manual export..." -ForegroundColor Yellow
                Start-Process -FilePath $radvideo -ArgumentList @($file.FullName)
            }
            elseif ($radvideo) {
                Write-Host "RADVideo is available here:" -ForegroundColor Yellow
                Write-Host "  $radvideo"
                Write-Host "Run again with -OpenRadOnUnsupported to open it automatically."
            }
            $skipped += 1
            continue
        }
        if (!$signature.StartsWith("BIK")) {
            Write-Warning "Skipping unknown Bink signature '$signature': $($file.FullName)"
            $skipped += 1
            continue
        }

        $out = Get-OutputPath -InputFile $file -OutputDir $OutputDir
        if ((Test-Path -LiteralPath $out) -and !$Overwrite) {
            Write-Warning "Output exists, skipped: $out"
            $skipped += 1
            continue
        }

        Write-Host "Converting:" -ForegroundColor Cyan
        Write-Host "  Input : $($file.FullName)"
        Write-Host "  Output: $out"

        $args = @(
            "-hide_banner",
            "-y",
            "-i", $file.FullName,
            "-map", "0:v:0",
            "-map", "0:a?",
            "-c:v", "libx264",
            "-pix_fmt", "yuv420p",
            "-crf", [string]$Crf,
            "-preset", $Preset,
            "-c:a", "aac",
            "-b:a", "192k",
            "-movflags", "+faststart",
            $out
        )

        & $ffmpeg @args
        if ($LASTEXITCODE -ne 0) {
            throw "ffmpeg failed for $($file.FullName) with exit code $LASTEXITCODE"
        }
        $converted += 1
    }
}

Write-Host "Done. Converted $converted file(s), skipped $skipped file(s)." -ForegroundColor Green

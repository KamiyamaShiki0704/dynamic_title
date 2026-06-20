# BK2 to MP4

Small helper for converting `.bk2` files to `.mp4` with ffmpeg.

## Drag and Drop

Drag one or more `.bk2` files onto:

```text
BK2ToMP4.bat
```

The output `.mp4` files are written next to the source files.

## PowerShell

```powershell
.\BK2ToMP4.ps1 "F:\path\movie.bk2"
```

Convert multiple files:

```powershell
.\BK2ToMP4.ps1 "F:\a.bk2" "F:\b.bk2"
```

Write output to a folder:

```powershell
.\BK2ToMP4.ps1 "F:\movie.bk2" -OutputDir "F:\converted"
```

Overwrite existing MP4 files:

```powershell
.\BK2ToMP4.ps1 "F:\movie.bk2" -Overwrite
```

Open RADVideo automatically when the file is Bink 2 (`KB2*`) and ffmpeg cannot decode it:

```powershell
.\BK2ToMP4.ps1 "F:\movie.bk2" -OpenRadOnUnsupported
```

Tune quality:

```powershell
.\BK2ToMP4.ps1 "F:\movie.bk2" -Crf 20 -Preset medium
```

Lower `Crf` means better quality and larger files. The default is `18`.

## Tool Detection

The script first looks for:

```text
G:\ffmpeg\ffmpeg-8.0-essentials_build\bin\ffmpeg.exe
```

If that is missing, it falls back to `ffmpeg.exe` from `PATH`.

## Bink 2 Note

Some `.bk2` files start with `KB2*` such as `KB2n`. These are Bink 2 files. ffmpeg often cannot decode them and reports:

```text
Invalid data found when processing input
```

For those files, use RADVideo manually. The script will detect this case and print a clearer message instead of a PowerShell error.

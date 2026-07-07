# CLAUDE.md

Guidance for Claude Code (and humans) working in this repository.

## What this is

**DontTouchMeBro** is a tiny Windows system-tray utility that enables/disables a
single hardware device (originally a laptop touch screen) by its Device Instance
Path. Clicking the tray icon toggles the device; the icon reflects the current
state. Because changing device state requires elevation, the app runs as
Administrator (declared in `app.manifest`).

- **Language / stack:** C#, WinForms, targeting `net10.0-windows` (Windows-only).
- **Elevation:** requires Administrator (`requestedExecutionLevel level="requireAdministrator"`).
- **Device control:** WMI (`System.Management`, `Win32_PnPEntity`), invoking the
  `Enable` / `Disable` methods on the matching device.

## Build & run

```powershell
# Build (Debug)
dotnet build DontTouchMeBro.sln

# Run — must be launched elevated (the manifest forces a UAC prompt).
# From an elevated shell:
dotnet run --project DontTouchMeBro/DontTouchMeBro.csproj

# Publish a distributable build (self-contained single-file x64 by default,
# output in .\dist\win-x64). See the script header for parameters.
.\build.ps1
```

`build.ps1` wraps `dotnet publish`. It defaults to a self-contained,
single-file build so end users don't need the .NET runtime installed, and it
strips the dev-only `device-id.txt` from the output. Single-file publishing is
why app-directory lookups must use `AppContext.BaseDirectory`, not
`Assembly.Location` (which returns an empty string in single-file apps).

- Toolchain confirmed working: .NET SDK 10.0.x.
- There are **no tests** in this repo.
- Config lives at `%APPDATA%\DontTouchMeBro\device-id.txt` and holds the target
  Device Instance Path (e.g. `HID\ELAN2D25&COL01\5&2B77D6B&0&0000`). It is
  created at runtime — either by editing the file directly or via the tray menu
  **Configure** dialog (`AboutWindow`), which lists HID devices and writes the
  chosen ID back. `Program.MigrateLegacyConfig` copies a pre-existing
  `device-id.txt` from next to the exe (older layout) on first run.

## Packaging

- `make-installer.ps1` is the one-step entry point: it runs `build.ps1` then
  compiles the installer with ISCC (auto-detected in the per-user and machine
  Inno Setup locations). `-SkipBuild` recompiles without republishing;
  `-Version` flows through to the setup filename.
- `installer/DontTouchMeBro.iss` is the Inno Setup script producing a **per-user**
  installer (`PrivilegesRequired=lowest`, installs to `{autopf}` →
  `%LocalAppData%\Programs`) with a Start Menu shortcut. It packages the
  single-file exe from `dist\win-x64`. Compiled installers land in
  `installer\Output\` (gitignored). The app version is read from the exe
  (`GetVersionNumbersString`) so it tracks `AssemblyInfo.cs`.

## Architecture / where things live

| File | Responsibility |
|------|----------------|
| `Program.cs` | Entry point. Single-instance `Mutex`, global exception handlers, config file read/write, wires up static event handlers used by the tray menu. Holds global `CurrentDevice` state. |
| `MainForm.cs` | Hidden top-level form that owns the `NotifyIcon` (tray icon). Handles click-to-toggle, a watchdog timer that re-shows the icon, and `WM_TASKBARCREATED` recovery when Explorer restarts. |
| `DeviceManager.cs` | All WMI access. Enumerates HID devices, looks up a device by ID, enables/disables it, and maps `ConfigManagerErrorCode` values. |
| `AboutWindow.cs` / `.Designer.cs` | "Configure" dialog: lists devices in a `ListView`, lets the user pick the target device and save it. |
| `NativeMethods.cs` | P/Invoke: `RegisterWindowMessage`, `ChangeWindowMessageFilter`, and the `TaskbarCreated` message plumbing. |
| `ErrorLogger.cs` | Writes to the Windows Event Log (source `DontTouchMeBroApp`, `Application` log). Creating the event source requires elevation (fine here since the app is elevated). |
| `app.manifest` | Forces Administrator elevation and declares Windows 10 support. |

## Key concepts

- **Device state is read from `ConfigManagerErrorCode`:** `"0"` = enabled/OK,
  `"22"` = disabled. Any other value is treated as an error/unknown state and
  shows the error icon. See the constants in `DeviceManager.ConfigManagerErrorCode`.
- **Tray icon resilience:** Explorer can restart (or the icon can vanish). The
  app defends against this three ways — a 30s watchdog `Timer`, handling the
  `WM_TASKBARCREATED` broadcast in `WndProc`, and `RestoreNotifyIcon` /
  `RecreateNotifyIcon`.
- **Global mutable state:** `Program.CurrentDevice` is a public static that
  `MainForm` and `DeviceManager` read and write directly. This coupling is known
  and called out in code comments; keep it in mind when changing state flow.

## Conventions & gotchas

- **Windows-only, elevated-only.** Anything touching devices or the Event Log
  needs to run as Administrator. `CA1416` (platform-compatibility) warnings are
  suppressed in the `.csproj` on purpose.
- `Nullable` and `ImplicitUsings` are **disabled** — match the existing explicit
  `using` style and don't assume nullable reference types.
- Build artifacts (`bin/`, `obj/`), `dist/`, and `installer/Output/` are
  gitignored. `device-id.txt` is **not** tracked (it's per-user config now living
  in `%APPDATA%`); `device-id.txt.example` documents the format.
- The tray menu wires directly to `static` handlers on `Program`
  (`OnShowSettings`, `OnShowAbout`, `OnExit`). New menu items follow the same
  pattern today.

## Related docs

- `README.md` — user-facing explanation and how to find a Device Instance Path.
- `REVIEW.md` — performance / security / readability review (findings not yet
  applied to the code).

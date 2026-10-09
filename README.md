# DontTouchMeBro

## What is this thing.
Very simple C# Application that enables and disables a device by its Device Instance Path.

## Why
Well I purchased a new Dell XPS 15 Laptop. and at the time to get the OLED screen you had to get a touch screen. I had no real need for the touch screen do I did a little digging and found that I could disable it by entering the command:
`pnputil /disable-device "HID\ELAN2D25&COL01\5&2B77D6B&0&0000"`

and re-enable it with:
`pnputil /enable-device "HID\ELAN2D25&COL01\5&2B77D6B&0&0000"`

For a while I had two shortcuts that ran batch scripts as Administrator (since you need admin access to enable or disable devices).

Then I got a little tired of that and made this really simple application that runs the above two commands based on the state of an icon in the Window Notification bar.

Job done...

## What's Next

Well Maybe I'll flush it out to ask the user for a Device instance path and store it in a file or registry, but to be honest it works for me at this point so that might take a while.

## Installing

Build the installer in one step with [Inno Setup](https://jrsoftware.org/isinfo.php)
installed:

```powershell
./make-installer.ps1
```

This publishes a self-contained exe (no .NET runtime needed) and compiles the
installer. The two steps can also be run individually — `./build.ps1` to
publish, then `ISCC.exe installer\DontTouchMeBro.iss` to compile.

The resulting `installer\Output\DontTouchMeBro-Setup-*.exe` installs **for all
users into `C:\Program Files\DontTouchMeBro`** and needs Administrator to
install. It adds an all-users Start Menu shortcut.

Why per-machine: the app always runs as Administrator (it needs that to toggle
devices). If its exe lived in a folder your normal, non-elevated account can
write to, any program running as you could replace it and have the replacement
run as admin the next time you start it. Program Files is only writable by
administrators, so that isn't possible.

### Start automatically when I sign in

The installer has a **Start automatically when I sign in** option (checked by
default). It creates a Scheduled Task named `DontTouchMeBro` that starts the
app when you sign in, with highest privileges, so there is **no UAC prompt at
each sign-in**. (The usual `Run` registry key can't be used: Windows silently
skips apps that require Administrator there.)

The task:

- runs only for the account that approved the installer's UAC prompt (see
  [Known limitations](#known-limitations)), only when that account signs in
  interactively;
- has no run-time limit and isn't blocked or stopped on battery power (the
  Task Scheduler defaults would otherwise kill the tray app after 72 hours);
- is removed when you uninstall, or when you re-run the installer with the
  option unchecked.

You can see or disable it in Task Scheduler (`taskschd.msc`, top-level
library), or from an elevated prompt:
`schtasks /Query /TN DontTouchMeBro` / `schtasks /Delete /TN DontTouchMeBro /F`.

### Upgrading from an older (per-user) install

Earlier installers put the app in `%LocalAppData%\Programs\DontTouchMeBro` for
the current user only. When the new installer finds that copy (in the profile
of the account running setup) it offers to remove it first; this is
recommended. Your device setting in `%APPDATA%\DontTouchMeBro\device-id.txt`
is not touched.

If the old copy wasn't found (see below) or you chose to keep it, remove it
yourself from **Settings > Apps > Installed apps** (it's the entry installed
under your user, not the Program Files one), or run
`%LocalAppData%\Programs\DontTouchMeBro\unins000.exe`.

### Uninstalling

Uninstall from **Settings > Apps**. The uninstaller stops the app if it's
running, deletes the sign-in task, and removes the program files. Your
`%APPDATA%\DontTouchMeBro` config folder is left in place.

### Known limitations

These apply when you sign in as a **standard (non-admin) user** and someone
types an *administrator's* credentials into the UAC prompt ("over-the-shoulder"
elevation). The app and the installer then run as that administrator account,
not as you:

- **Config location:** the app's config goes to the *administrator's*
  `%APPDATA%\DontTouchMeBro\device-id.txt`, not yours.
- **Start at sign-in:** the task is created for the administrator account (it
  starts when *they* sign in), not for you. This is unavoidable: a standard
  account's "highest privileges" are still non-admin, so a task can never start
  this app elevated for it. Standard users start the app from the Start Menu
  and approve the UAC prompt each time.
- **Upgrade cleanup:** the installer looks for an old per-user copy in the
  administrator's profile, so your own old copy isn't found; remove it manually
  as described above.

If your everyday account is an administrator (the normal case, where UAC just
asks you to click **Yes**), none of this applies.

## How do I make it work my Device or Touch Screen

The easiest way is to launch the app and pick your device from the tray icon's
**Configure** dialog. Your choice is saved to
`%APPDATA%\DontTouchMeBro\device-id.txt`.

You can also create that file yourself: put your specific Device Instance Path
in `%APPDATA%\DontTouchMeBro\device-id.txt`. (Older versions kept this file next
to the executable; on first run the app migrates it to the new location
automatically.)


## How do I find my Device Instance Path?

There are a few ways to do this but the simplest seems to be:

* Hit Ctrl + X to and Choose `Device Manager`
* Find the device you are looking for, if it's a Touch Screen try in the Human Interface Devices
* Once you have found the device go it it's properties.
* Once in the properties, do to the Details Tab and under the Property choose Device instance path:
* In the value Section, copy this text and use it for the instanceID variable.

## Why does the application always popup a UAC (How User Account Control) Dialog?
Well in order for an application to make changes to any devices on the computer it needs to be an administrator. So I had a choice, make the application require administrator access to run, or make each press of the button require admin access, the former seemed less annoying to me.

## How can I get this app to run at startup without asking for UAC each time

Tick **Start automatically when I sign in** in the installer (it's on by
default). It sets up a Scheduled Task that starts the app elevated at sign-in
without a prompt; see [Start automatically when I sign in](#start-automatically-when-i-sign-in)
above. If you run the exe without the installer, you can create a similar task
yourself in Task Scheduler ("Run with highest privileges", trigger "At log on",
and clear "Stop the task if it runs longer than").

## Hey, so if this asks for app ask for Admin rights can I trust it?
In a short, No; never blindly trust someone you don't know. I tried to do the right thing but you trust you.

//! The tray process: a hidden window that owns the notification icon,
//! toggles the device on left-click and shows the menu on right-click.
//!
//! Plain Win32 rather than a tray crate so the app can show balloon
//! notifications and re-add its icon when Explorer restarts. The window is a
//! hidden top-level window (not message-only) because message-only windows
//! don't receive the `TaskbarCreated` broadcast.

use std::cell::RefCell;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HINSTANCE, HWND, LPARAM, LRESULT,
    POINT, WPARAM,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex};
use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_ERROR, NIIF_INFO, NIIF_WARNING, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NOTIFY_ICON_DATA_FLAGS, NOTIFY_ICON_MESSAGE, NOTIFYICONDATAW,
    Shell_NotifyIconW, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, ChangeWindowMessageFilterEx, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
    DestroyMenu, DestroyWindow, DispatchMessageW, EnumWindows, FindWindowW, GetCursorPos,
    GetMessageW, GetSystemMetrics, GetWindowThreadProcessId, HICON, IMAGE_ICON, IsIconic,
    IsWindowVisible, LR_DEFAULTCOLOR, LoadImageW, MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG,
    MSGFLT_ALLOW, PostMessageW, PostQuitMessage, RegisterClassExW, RegisterWindowMessageW,
    SM_CXSMICON, SM_CYSMICON, SW_RESTORE, SW_SHOWNORMAL, SetForegroundWindow, SetMenuDefaultItem,
    SetTimer, ShowWindow, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu,
    TranslateMessage, WM_APP, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP,
    WM_TIMER, WNDCLASSEXW, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};
use windows::core::{BOOL, PCWSTR, w};

use crate::config::{self, Config};
use crate::device::SetupApi;
use crate::status::{DeviceState, IconKind};
use crate::toggle::{self, Notice, NoticeKind};
use crate::win::{self, copy_to_buffer, wide};
use crate::{CONFIGURE_ARG, logging};

/// Callback message for tray icon mouse events.
const WM_TRAY: u32 = WM_APP + 1;
/// Posted by the watcher thread when the Configure process exits.
const WM_CONFIGURE_CLOSED: u32 = WM_APP + 2;
/// Posted by a second instance so the running one can say it's already here.
const WM_ALREADY_RUNNING: u32 = WM_APP + 3;

const WINDOW_CLASS: PCWSTR = w!("DontTouchMeBroTrayWindow");
/// Same name the C# version used, so old and new builds never run together.
const SINGLE_INSTANCE_MUTEX: PCWSTR = w!("DontTouchMeBro!");
const TRAY_ICON_ID: u32 = 1;

/// Re-check the device this often so the icon follows changes made
/// elsewhere (Device Manager, unplugging, sleep).
const POLL_TIMER_ID: usize = 1;
const POLL_INTERVAL_MS: u32 = 10_000;

const MENU_REVEAL: usize = 1;
const MENU_CONFIGURE: usize = 2;
const MENU_EXIT: usize = 3;
const MENU_TOGGLE: usize = 4;

/// Icon resource IDs, embedded by build.rs.
const ICON_ENABLED: u16 = 101;
const ICON_DISABLED: u16 = 102;
const ICON_ERROR: u16 = 103;

/// `TaskbarCreated` is a registered message, so its ID is only known at runtime.
static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static TRAY: RefCell<Option<Tray>> = const { RefCell::new(None) };
}

/// Run the tray until the user chooses Exit.
pub fn run() -> Result<(), String> {
    let Some(_instance) = SingleInstance::acquire() else {
        notify_running_instance();
        return Ok(());
    };

    let config =
        Config::new(config::default_dir().ok_or("The APPDATA environment variable isn't set.")?);
    if let Some(exe_dir) = win::exe_dir() {
        match config.migrate_legacy(&exe_dir) {
            Ok(true) => logging::info(&format!(
                "Migrated legacy config from {}",
                exe_dir.display()
            )),
            Ok(false) => {}
            Err(err) => logging::error(&format!("Config migration failed: {err}")),
        }
    }
    let configured = read_config_reporting_errors(&config);

    let instance: HINSTANCE = unsafe { GetModuleHandleW(None) }
        .map_err(|err| format!("GetModuleHandle failed: {err}"))?
        .into();
    let hwnd = create_window(instance)?;

    let taskbar_created = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    TASKBAR_CREATED.store(taskbar_created, Ordering::Relaxed);
    if taskbar_created != 0 {
        // The app is elevated; let the (unelevated) Explorer's broadcast
        // through UIPI, to this window only.
        if let Err(err) =
            unsafe { ChangeWindowMessageFilterEx(hwnd, taskbar_created, MSGFLT_ALLOW, None) }
        {
            logging::error(&format!("ChangeWindowMessageFilterEx failed: {err}"));
        }
    }

    let backend = SetupApi;
    let state = toggle::query_state(&backend, configured.as_deref()).unwrap_or_else(|err| {
        logging::error(&format!("Querying device: {err}"));
        DeviceState::NotFound
    });

    let tray = Tray {
        hwnd,
        icons: Icons::load(instance)?,
        config,
        configured,
        state,
        backend,
        configure_pid: None,
        last_toggle: None,
    };
    tray.add_icon();
    if tray.state == DeviceState::NotConfigured {
        tray.show_notice(&Notice {
            kind: NoticeKind::Info,
            title: "No device configured".into(),
            text: "Right-click the tray icon and choose Configure to pick the device to toggle."
                .into(),
        });
    }
    TRAY.with(|cell| *cell.borrow_mut() = Some(tray));

    unsafe { SetTimer(Some(hwnd), POLL_TIMER_ID, POLL_INTERVAL_MS, None) };

    let mut msg = MSG::default();
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    TRAY.with(|cell| cell.borrow_mut().take());
    Ok(())
}

struct Tray {
    hwnd: HWND,
    icons: Icons,
    config: Config,
    configured: Option<String>,
    state: DeviceState,
    backend: SetupApi,
    /// Process ID of the open Configure window, if any.
    configure_pid: Option<u32>,
    /// When the last click-to-toggle finished; see [`Tray::toggle`].
    last_toggle: Option<Instant>,
}

impl Tray {
    fn icon_data(&self, flags: NOTIFY_ICON_DATA_FLAGS) -> NOTIFYICONDATAW {
        NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            uFlags: flags,
            ..Default::default()
        }
    }

    fn icon_with_state(&self, flags: NOTIFY_ICON_DATA_FLAGS) -> NOTIFYICONDATAW {
        let mut data = self.icon_data(flags | NIF_ICON | NIF_TIP);
        data.hIcon = self.icons.get(self.state.icon());
        copy_to_buffer(&mut data.szTip, &self.state.tooltip());
        data
    }

    /// Add the icon (also used to re-add it after Explorer restarts).
    fn add_icon(&self) {
        let mut data = self.icon_with_state(NIF_MESSAGE);
        data.uCallbackMessage = WM_TRAY;
        if !notify(NIM_ADD, &data) {
            // Already present (e.g. a spurious TaskbarCreated): just update it.
            self.update_icon();
        }
    }

    fn update_icon(&self) {
        if !notify(NIM_MODIFY, &self.icon_with_state(NOTIFY_ICON_DATA_FLAGS(0))) {
            // The icon was lost without a TaskbarCreated broadcast; add it back.
            let mut data = self.icon_with_state(NIF_MESSAGE);
            data.uCallbackMessage = WM_TRAY;
            notify(NIM_ADD, &data);
        }
    }

    fn remove_icon(&self) {
        notify(NIM_DELETE, &self.icon_data(NOTIFY_ICON_DATA_FLAGS(0)));
    }

    fn set_state(&mut self, state: DeviceState) {
        if state != self.state {
            self.state = state;
            self.update_icon();
        }
    }

    fn show_notice(&self, notice: &Notice) {
        let mut data = self.icon_data(NIF_INFO);
        copy_to_buffer(&mut data.szInfoTitle, &notice.title);
        copy_to_buffer(
            &mut data.szInfo,
            if notice.text.is_empty() {
                &notice.title
            } else {
                &notice.text
            },
        );
        data.dwInfoFlags = match notice.kind {
            NoticeKind::Info => NIIF_INFO,
            NoticeKind::Warning => NIIF_WARNING,
            NoticeKind::Error => NIIF_ERROR,
        };
        notify(NIM_MODIFY, &data);
    }

    /// Left-click on the icon.
    fn on_click(&mut self) {
        // A double-click delivers two button-up messages. Treat a click that
        // lands within the double-click time of the last toggle as part of
        // the same gesture, so a double-click doesn't flip the device off
        // and straight back on.
        let double_click = Duration::from_millis(unsafe { GetDoubleClickTime() }.into());
        if self
            .last_toggle
            .is_some_and(|at| at.elapsed() < double_click)
        {
            return;
        }
        self.toggle();
    }

    fn toggle(&mut self) {
        let report = toggle::toggle(&self.backend, self.configured.as_deref());
        self.last_toggle = Some(Instant::now());
        if let Some(message) = &report.log {
            logging::error(message);
        }
        if let Some(message) = &report.done {
            logging::info(message);
        }
        if let Some(state) = report.state {
            self.set_state(state);
        }
        if let Some(notice) = &report.notice {
            self.show_notice(notice);
        }
    }

    /// Timer tick: pick up config edits and device changes made elsewhere.
    fn refresh(&mut self) {
        if let Ok(configured) = self.config.read() {
            self.configured = configured;
        }
        match toggle::query_state(&self.backend, self.configured.as_deref()) {
            Ok(state) => self.set_state(state),
            // Don't fill the log every 10 seconds; transient failures happen.
            Err(err) => logging::debug(&format!("State poll failed: {err}")),
        }
    }

    fn reload_config(&mut self) {
        self.configured = read_config_reporting_errors(&self.config);
        self.refresh();
    }

    fn reveal_config_folder(&self) {
        let dir = self.config.dir();
        if let Err(err) = std::fs::create_dir_all(dir) {
            logging::error(&format!("Creating {}: {err}", dir.display()));
        }
        open_folder(self.hwnd, dir);
    }

    fn open_configure(&mut self) {
        if let Some(pid) = self.configure_pid {
            bring_process_to_front(pid);
            return;
        }

        let child = std::env::current_exe()
            .and_then(|exe| std::process::Command::new(exe).arg(CONFIGURE_ARG).spawn());
        let mut child = match child {
            Ok(child) => child,
            Err(err) => {
                logging::error(&format!("Starting the Configure window: {err}"));
                self.show_notice(&Notice {
                    kind: NoticeKind::Error,
                    title: "Couldn't open Configure".into(),
                    text: err.to_string(),
                });
                return;
            }
        };
        self.configure_pid = Some(child.id());

        // HWND isn't Send; pass the raw value to the watcher thread.
        let hwnd = self.hwnd.0 as isize;
        std::thread::spawn(move || {
            let _ = child.wait();
            let hwnd = HWND(hwnd as *mut _);
            let _ = unsafe { PostMessageW(Some(hwnd), WM_CONFIGURE_CLOSED, WPARAM(0), LPARAM(0)) };
        });
    }
}

/// Run `f` against the tray state. Returns `None` if called re-entrantly
/// (e.g. a message dispatched while a device call is in progress), in which
/// case the event is dropped rather than panicking on a double borrow.
fn with_tray<R>(f: impl FnOnce(&mut Tray) -> R) -> Option<R> {
    TRAY.with(|cell| cell.try_borrow_mut().ok()?.as_mut().map(f))
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TRAY => {
            // Without NOTIFYICON_VERSION_4, lParam is the mouse message.
            match lparam.0 as u32 & 0xFFFF {
                WM_LBUTTONUP => {
                    with_tray(Tray::on_click);
                }
                WM_RBUTTONUP | WM_CONTEXTMENU => show_menu(hwnd),
                _ => {}
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == POLL_TIMER_ID => {
            with_tray(Tray::refresh);
            LRESULT(0)
        }
        WM_CONFIGURE_CLOSED => {
            with_tray(|tray| {
                tray.configure_pid = None;
                tray.reload_config();
            });
            LRESULT(0)
        }
        WM_ALREADY_RUNNING => {
            with_tray(|tray| {
                tray.show_notice(&Notice {
                    kind: NoticeKind::Info,
                    title: "Already running".into(),
                    text: "Don't Touch Me Bro is already in the notification area.".into(),
                })
            });
            LRESULT(0)
        }
        WM_DESTROY => {
            with_tray(|tray| tray.remove_icon());
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ if msg != 0 && msg == TASKBAR_CREATED.load(Ordering::Relaxed) => {
            // Explorer restarted: the old icon is gone, add it again.
            with_tray(|tray| tray.add_icon());
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Show the tray menu at the cursor and run the chosen command. Holds no
/// borrow of the tray state while the menu's modal loop runs.
fn show_menu(hwnd: HWND) {
    // Re-read the device first so the toggle item matches its real state.
    let Some((toggle_text, can_toggle)) = with_tray(|tray| {
        tray.refresh();
        toggle::menu_item(&tray.state)
    }) else {
        return;
    };
    // '&' marks a keyboard shortcut in menu text; "&&" is a literal one.
    let toggle_text = wide(&toggle_text.replace('&', "&&"));

    let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
        return;
    };
    unsafe {
        // Enable / Disable <device> first, in bold (the same as clicking
        // the icon), or greyed out saying why it can't be toggled.
        let flags = if can_toggle {
            MF_STRING
        } else {
            MF_STRING | MF_GRAYED
        };
        let _ = AppendMenuW(menu, flags, MENU_TOGGLE, PCWSTR(toggle_text.as_ptr()));
        if can_toggle {
            let _ = SetMenuDefaultItem(menu, MENU_TOGGLE as u32, 0);
        }
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, MENU_CONFIGURE, w!("Configure..."));
        let _ = AppendMenuW(menu, MF_STRING, MENU_REVEAL, w!("Reveal in File Explorer"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, MENU_EXIT, w!("Exit"));
    }

    let mut point = POINT::default();
    let command = unsafe {
        let _ = GetCursorPos(&mut point);
        // Required so the menu closes when the user clicks elsewhere.
        let _ = SetForegroundWindow(hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            None,
            hwnd,
            None,
        );
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        command.0 as usize
    };

    match command {
        MENU_TOGGLE => {
            with_tray(Tray::toggle);
        }
        MENU_REVEAL => {
            with_tray(|tray| tray.reveal_config_folder());
        }
        MENU_CONFIGURE => {
            with_tray(Tray::open_configure);
        }
        MENU_EXIT => {
            let _ = unsafe { DestroyWindow(hwnd) };
        }
        _ => {}
    }
}

fn notify(message: NOTIFY_ICON_MESSAGE, data: &NOTIFYICONDATAW) -> bool {
    unsafe { Shell_NotifyIconW(message, data) }.as_bool()
}

/// Read the config, telling the user about real errors (permissions, a
/// locked file). Missing or blank means "not configured" and isn't an error.
fn read_config_reporting_errors(config: &Config) -> Option<String> {
    match config.read() {
        Ok(configured) => configured,
        Err(err) => {
            let path = config.path();
            logging::error(&format!("Reading config {}: {err}", path.display()));
            win::error_box(
                "Could not read config",
                &format!(
                    "Could not read the device configuration at\n{}\n\n{err}",
                    path.display()
                ),
            );
            None
        }
    }
}

fn create_window(instance: HINSTANCE) -> Result<HWND, String> {
    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(window_proc),
        hInstance: instance,
        lpszClassName: WINDOW_CLASS,
        ..Default::default()
    };
    if unsafe { RegisterClassExW(&class) } == 0 {
        return Err(format!(
            "RegisterClassEx failed: {}",
            windows::core::Error::from_thread()
        ));
    }
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            WINDOW_CLASS,
            w!("Don't Touch Me Bro"),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )
    }
    .map_err(|err| format!("CreateWindowEx failed: {err}"))
}

struct Icons {
    enabled: HICON,
    disabled: HICON,
    error: HICON,
}

impl Icons {
    fn load(instance: HINSTANCE) -> Result<Self, String> {
        let (cx, cy) = unsafe { (GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON)) };
        let load = |id: u16| -> Result<HICON, String> {
            unsafe {
                LoadImageW(
                    Some(instance),
                    PCWSTR(id as usize as *const u16),
                    IMAGE_ICON,
                    cx,
                    cy,
                    LR_DEFAULTCOLOR,
                )
            }
            .map(|handle| HICON(handle.0))
            .map_err(|err| format!("Loading icon {id}: {err}"))
        };
        Ok(Icons {
            enabled: load(ICON_ENABLED)?,
            disabled: load(ICON_DISABLED)?,
            error: load(ICON_ERROR)?,
        })
    }

    fn get(&self, kind: IconKind) -> HICON {
        match kind {
            IconKind::Enabled => self.enabled,
            IconKind::Disabled => self.disabled,
            IconKind::Error => self.error,
        }
    }
}

fn open_folder(hwnd: HWND, dir: &Path) {
    let dir = wide(&dir.to_string_lossy());
    let result = unsafe {
        ShellExecuteW(
            Some(hwnd),
            w!("open"),
            PCWSTR(dir.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecute returns a value > 32 on success.
    if result.0 as usize <= 32 {
        logging::error(&format!(
            "Opening the config folder failed (ShellExecute returned {})",
            result.0 as usize
        ));
    }
}

/// Bring the visible top-level window of process `pid` to the foreground.
fn bring_process_to_front(pid: u32) {
    struct Search {
        pid: u32,
        found: Option<HWND>,
    }

    extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = unsafe { &mut *(lparam.0 as *mut Search) };
        let mut window_pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut window_pid)) };
        if window_pid == search.pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
            search.found = Some(hwnd);
            return BOOL(0);
        }
        BOOL(1)
    }

    let mut search = Search { pid, found: None };
    let _ = unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) };
    if let Some(hwnd) = search.found {
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

/// Tell an already-running tray to show an "already running" balloon.
fn notify_running_instance() {
    if let Ok(hwnd) = unsafe { FindWindowW(WINDOW_CLASS, PCWSTR::null()) } {
        let _ = unsafe { PostMessageW(Some(hwnd), WM_ALREADY_RUNNING, WPARAM(0), LPARAM(0)) };
    }
}

/// Holds the named mutex that keeps a second tray from starting.
struct SingleInstance(HANDLE);

impl SingleInstance {
    fn acquire() -> Option<Self> {
        let handle = unsafe { CreateMutexW(None, true, SINGLE_INSTANCE_MUTEX) }.ok()?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            let _ = unsafe { CloseHandle(handle) };
            return None;
        }
        Some(SingleInstance(handle))
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.0);
            let _ = CloseHandle(self.0);
        }
    }
}

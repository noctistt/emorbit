//! Windows proof of concept. All native callbacks run on the message thread.
//! The boxed App outlives its HWND; GWLP_USERDATA is cleared before it is freed.
use emorbit::{
    catalog::{self, CATALOG},
    favorites::Favorites,
    motion::Motion,
    unicode_units,
    wheel::{self, Hit, Point},
};
use std::{
    cell::{Cell, RefCell},
    mem::size_of,
    sync::atomic::{AtomicPtr, Ordering},
    time::Instant,
};
use windows::{
    core::{w, Error, Result, BOOL, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::{Common::*, *},
            DirectWrite::*,
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Gdi::*,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Accessibility::*,
            Controls::{EM_SETLIMITTEXT, WM_MOUSELEAVE},
            HiDpi::*,
            Input::KeyboardAndMouse::*,
            Shell::*,
            WindowsAndMessaging::*,
        },
    },
};
use windows_numerics::{Matrix3x2, Vector2};

const HOTKEY: i32 = 1;
const ESCAPE_KEY: i32 = 2;
const TICK: usize = 1;
const ANIMATE: usize = 3;
const VERIFY: usize = 4;
thread_local! { static PRESENTING: Cell<bool> = const { Cell::new(false) }; }
const TRAY: u32 = WM_APP + 1;
const FOREGROUND: u32 = WM_APP + 2;
const SHOW: u32 = WM_APP + 3;
const SEARCH_ENTER: u32 = WM_APP + 4;
const SEARCH_ESCAPE: u32 = WM_APP + 5;
const REFRESH_SIZE: u32 = WM_APP + 6;
const EXIT: u32 = WM_APP + 7;
const RENDER: u32 = WM_APP + 8;
static WINDOW: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

// Keep a small local diagnostic log without recording searches or inserted text.
fn diagnostic(message: &str) {
    use std::io::Write;
    let Some(local) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let directory = std::path::PathBuf::from(local).join("EmOrbit");
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let path = directory.join("emorbit.log");
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > 262_144) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let _ = writeln!(file, "{now} {message}");
    }
}

#[derive(Clone, Copy)]
struct Destination {
    window: HWND,
    focus: HWND,
}

impl Destination {
    unsafe fn capture(own: HWND) -> Option<Self> {
        let window = GetForegroundWindow();
        if window.0.is_null() || window == own {
            return None;
        }
        let thread = GetWindowThreadProcessId(window, None);
        let mut info = GUITHREADINFO {
            cbSize: size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        GetGUIThreadInfo(thread, &mut info).ok()?;
        if info.hwndFocus.0.is_null() {
            return None;
        }
        Some(Self {
            window,
            focus: info.hwndFocus,
        })
    }

    unsafe fn still_focused(self) -> bool {
        if !IsWindow(Some(self.window)).as_bool()
            || !IsWindow(Some(self.focus)).as_bool()
            || GetForegroundWindow() != self.window
        {
            return false;
        }
        Self::capture(HWND::default()).is_some_and(|current| current.focus == self.focus)
    }
}

struct Pending {
    text: String,
    target: Destination,
    started: Instant,
}

struct App {
    hwnd: HWND,
    search_host: HWND,
    edit: HWND,
    render_queued: Cell<bool>,
    graphics: Option<Graphics>,
    visible: bool,
    pinned: bool,
    transparent: bool,
    choices: Vec<catalog::Suggestion>,
    smooth: bool,
    mouse_origin: Option<Hit>,
    searching: bool,
    adding_favorite: bool,
    favorites_mode: bool,
    favorites: Favorites,
    edge_pager: wheel::EdgePager,
    gesture_paged: bool,
    destination: Option<Destination>,
    results: Vec<usize>,
    page: usize,
    hover: Hit,
    context: Option<usize>,
    motion: Motion,
    motion_running: bool,
    motion_enabled: bool,
    last_frame: Instant,
    status: String,
    pending: Option<Pending>,
    dpi: u32,
    tray: NOTIFYICONDATAW,
}

impl App {
    fn new() -> Self {
        Self {
            hwnd: HWND::default(),
            search_host: HWND::default(),
            edit: HWND::default(),
            render_queued: Cell::new(false),
            graphics: None,
            visible: false,
            pinned: false,
            transparent: false,
            choices: Vec::new(),
            smooth: true,
            mouse_origin: None,
            searching: false,
            adding_favorite: false,
            favorites_mode: false,
            favorites: Favorites::default(),
            edge_pager: wheel::EdgePager::default(),
            gesture_paged: false,
            destination: None,
            results: catalog::search(""),
            page: 0,
            hover: Hit::None,
            context: None,
            motion: Motion::default(),
            motion_running: false,
            motion_enabled: true,
            last_frame: Instant::now(),
            status: "Tap Ctrl + Alt + Space to open".into(),
            pending: None,
            dpi: 96,
            tray: NOTIFYICONDATAW::default(),
        }
    }

    fn items(&self) -> &[usize] {
        catalog::page(&self.results, self.page)
    }
    fn item_text(&self, index: usize) -> &str {
        if let Some(emoji) = CATALOG.get(index) {
            emoji.text
        } else {
            self.favorites
                .items
                .get(index - CATALOG.len())
                .map_or("", String::as_str)
        }
    }
    fn search_results(&self, query: &str) -> Vec<usize> {
        if !self.favorites_mode {
            return catalog::search(query);
        }
        let query = query.trim().to_lowercase();
        self.favorites
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, text)| {
                let words = CATALOG.iter().find(|e| e.text == text).map_or_else(
                    || text.clone(),
                    |e| format!("{} {} {}", text, e.name, e.keywords),
                );
                query
                    .split_whitespace()
                    .all(|q| words.to_lowercase().contains(q))
                    .then_some(CATALOG.len() + i)
            })
            .collect()
    }
    fn favorites_path() -> Option<std::path::PathBuf> {
        std::env::var_os("LOCALAPPDATA").map(|p| {
            std::path::PathBuf::from(p)
                .join("EmOrbit")
                .join("favorites.txt")
        })
    }
    unsafe fn toggle_favorite(&mut self, text: &str, remove: bool) {
        let previous = self.favorites.clone();
        if remove {
            self.favorites.remove(text);
        } else if let Err(error) = self.favorites.add(text) {
            self.status = error.into();
            self.invalidate();
            return;
        }
        let result = Self::favorites_path()
            .ok_or_else(|| std::io::Error::other("Local app data is unavailable"))
            .and_then(|path| self.favorites.save(&path));
        if let Err(error) = result {
            self.favorites = previous;
            self.status = format!("Could not save favorites: {error}");
        } else {
            self.status = if remove {
                "Favorite removed"
            } else {
                "Favorite saved"
            }
            .into();
        }
        if self.favorites_mode {
            self.results = self.search_results("");
            self.page = self.page.min(catalog::page_count(&self.results) - 1);
        }
        self.set_context(None);
        self.hover = Hit::None;
        self.invalidate();
    }
    unsafe fn favorite_menu(&mut self) {
        self.mouse_origin = None;
        if self.pending.is_none() {
            let _ = KillTimer(Some(self.hwnd), TICK);
        }
        self.refresh_pointer();
        let Some(text) = self.selection(self.hover) else {
            return;
        };
        let Ok(menu) = CreatePopupMenu() else {
            return;
        };
        self.mouse_origin = None;
        let target = Destination::capture(self.hwnd).or(self.destination);
        let remove = self.favorites.contains(&text);
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            1,
            if remove {
                w!("Remove from favorites")
            } else {
                w!("Save to favorites")
            },
        );
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let _ = SetForegroundWindow(self.hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            cursor.x,
            cursor.y,
            Some(0),
            self.hwnd,
            None,
        )
        .0;
        let _ = DestroyMenu(menu);
        self.render_queued.set(false);
        self.invalidate();
        if let Some(target) = target {
            let _ = SetForegroundWindow(target.window);
        }
        if command == 1 {
            self.toggle_favorite(&text, remove);
        }
    }
    unsafe fn begin_favorite(&mut self) {
        if self.pending.is_some() {
            return;
        }
        self.begin_search();
        self.adding_favorite = true;
        self.status = "Paste emoji or a combination · Enter saves".into();
        self.position_edit();
        self.invalidate();
    }
    unsafe fn save_favorite_entry(&mut self) {
        let mut buffer = [0u16; 256];
        let count = GetWindowTextW(self.edit, &mut buffer).max(0) as usize;
        let text = String::from_utf16_lossy(&buffer[..count]);
        let mut probe = self.favorites.clone();
        if let Err(error) = probe.add(&text) {
            self.status = error.into();
            self.invalidate();
            return;
        }
        self.end_search();
        self.favorites_mode = true;
        self.toggle_favorite(text.trim(), false);
        if let Some(index) = self.favorites.items.iter().position(|s| s == text.trim()) {
            self.page = index / emorbit::PAGE_SIZE;
        }
        self.invalidate();
    }

    fn variants(&self) -> &[catalog::Suggestion] {
        &self.choices
    }
    fn context_layout(&self) -> Option<wheel::ContextLayout> {
        let index = self.context?;
        Some(wheel::ContextLayout {
            anchor: self
                .items()
                .iter()
                .position(|item| self.item_text(*item) == CATALOG[index].text)?,
            count: self.variants().len(),
            progress: self.motion.expansion.value,
        })
    }

    unsafe fn animate(&mut self) {
        let anchor = self.context_layout().map(|layout| layout.anchor);
        for (index, item) in self.motion.items.iter_mut().enumerate() {
            let target = if self.hover == Hit::Item(index) {
                1.0
            } else if anchor == Some(index) {
                0.55
            } else {
                0.0
            };
            item.target(target, 0.14);
        }
        for (index, variant) in self.motion.variants.iter_mut().enumerate() {
            variant.target(
                if self.hover == Hit::Variant(index) {
                    1.0
                } else {
                    0.0
                },
                0.12,
            );
        }
        self.motion
            .search
            .target(if self.hover == Hit::Search { 1.0 } else { 0.0 }, 0.14);
        if !self.motion_enabled {
            self.motion.snap();
            let _ = KillTimer(Some(self.hwnd), ANIMATE);
            self.motion_running = false;
        } else if self.motion.active() && !self.motion_running && self.visible {
            self.last_frame = Instant::now();
            self.motion_running = SetTimer(Some(self.hwnd), ANIMATE, 16, None) != 0;
            if !self.motion_running {
                self.motion.snap();
            }
        }
        self.invalidate();
    }

    unsafe fn animation_frame(&mut self) {
        let now = Instant::now();
        self.motion
            .advance(now.duration_since(self.last_frame).as_secs_f32());
        self.last_frame = now;
        if !self.motion.active() || !self.visible {
            let _ = KillTimer(Some(self.hwnd), ANIMATE);
            self.motion_running = false;
        }
        // Recompute animated hit regions during an active drag only. Idle
        // mouse hover is event driven, so a settled wheel does not poll.
        if self.mouse_origin.is_some() {
            self.refresh_pointer();
        }
        self.invalidate();
    }

    unsafe fn set_context(&mut self, index: Option<usize>) {
        if self.context == index {
            return;
        }
        self.context = index;
        self.choices = index.map_or_else(Vec::new, |i| catalog::suggestions(&CATALOG[i]));
        self.motion.expansion.reset(0.0);
        for variant in &mut self.motion.variants {
            variant.reset(0.0);
        }
        if index.is_some() {
            self.motion.expansion.target(1.0, 0.18);
        }
        self.animate();
    }

    unsafe fn motion_preference(&mut self) {
        let mut enabled = BOOL(1);
        if SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&mut enabled as *mut BOOL).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
        {
            self.motion_enabled = enabled.as_bool();
        }
        self.animate();
    }
    fn selection(&self, hit: Hit) -> Option<String> {
        match hit {
            Hit::Item(i) => self
                .items()
                .get(i)
                .map(|index| self.item_text(*index).to_owned()),
            Hit::Variant(i) => self.variants().get(i).map(|s| s.text.to_owned()),
            _ => None,
        }
    }
    unsafe fn invalidate(&self) {
        // Layered surfaces are presented explicitly, not by a continuous paint
        // loop. Coalesce requests from pointer events and finite animations.
        if !self.hwnd.0.is_null()
            && !self.render_queued.replace(true)
            && PostMessageW(Some(self.hwnd), RENDER, WPARAM(0), LPARAM(0)).is_err()
        {
            self.render_queued.set(false);
        }
    }

    unsafe fn set_transparency(&mut self, enabled: bool) {
        self.transparent = enabled;
        let style = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) as u32 & !WS_EX_LAYERED.0;
        // Clear the previous layered mode before switching APIs. Mixing
        // SetLayeredWindowAttributes and UpdateLayeredWindow is unsupported.
        SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, style as isize);
        if enabled {
            SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, (style | WS_EX_LAYERED.0) as isize);
            if !self.smooth {
                if let Err(error) =
                    SetLayeredWindowAttributes(self.hwnd, COLORREF(0x1e1312), 255, LWA_COLORKEY)
                {
                    diagnostic(&format!("Color key setup failed: {error}"));
                    self.transparent = false;
                    SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, style as isize);
                }
            }
        }
        self.graphics = None;
        diagnostic(if !self.transparent {
            "Using opaque GDI presentation"
        } else if self.smooth {
            "Using smooth alpha presentation with explicit geometry"
        } else {
            "Using compatibility color key presentation"
        });
        self.invalidate();
    }

    unsafe fn verify_presentation(&mut self) {
        let _ = KillTimer(Some(self.hwnd), VERIFY);
        if !self.visible || !self.transparent || !self.smooth || self.searching {
            return;
        }
        // Read one pixel at the center of our own solid search button, without
        // capturing or saving the desktop. This guards silent composition failure.
        let scale = self.dpi as f32 / 96.0;
        let mut point = POINT {
            x: (wheel::SEARCH_CENTER.x * scale) as i32,
            y: (wheel::SEARCH_CENTER.y * scale) as i32,
        };
        if !ClientToScreen(self.hwnd, &mut point).as_bool() {
            return;
        }
        let dc = GetDC(None);
        if dc.0.is_null() {
            return;
        }
        let pixel = GetPixel(dc, point.x, point.y);
        let _ = ReleaseDC(None, dc);
        if pixel.0 == 0xffffffff {
            diagnostic("Display pixel check unavailable");
            return;
        }
        let expected = blend(0x373049, 0x66519d, self.motion.search.value);
        let observed = ((pixel.0 & 255) << 16) | (pixel.0 & 0xff00) | ((pixel.0 >> 16) & 255);
        let matches = [0, 8, 16].iter().all(|shift| {
            (((observed >> shift) & 255) as i32 - ((expected >> shift) & 255) as i32).abs() <= 24
        });
        diagnostic(&format!(
            "Own button display check: {observed:06x}, expected {expected:06x}, match {matches}"
        ));
        if !matches {
            self.smooth = false;
            self.set_transparency(true);
            self.status = "Display compatibility mode · smooth edges available in tray".into();
            self.invalidate();
        }
    }

    unsafe fn show(&mut self) {
        diagnostic("Opening wheel");
        self.destination = Destination::capture(self.hwnd);
        self.visible = true;
        self.mouse_origin = None;
        self.searching = false;
        self.hover = Hit::None;
        self.context = None;
        self.choices.clear();
        self.motion = Motion::default();
        self.page = 0;
        self.results = self.search_results("");
        self.status = "Hold mouse · move · release to select".into();
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = GetMonitorInfoW(monitor, &mut info);
        self.dpi = GetDpiForWindow(self.hwnd).max(96);
        let scale = self.dpi as f32 / 96.0;
        let width = (wheel::WIDTH * scale) as i32;
        let height = (wheel::HEIGHT * scale) as i32;
        let (x, y) = wheel::clamp_origin(
            cursor.x - width / 2,
            cursor.y - (wheel::SEARCH_CENTER.y * scale) as i32,
            width,
            height,
            [
                info.rcWork.left,
                info.rcWork.top,
                info.rcWork.right,
                info.rcWork.bottom,
            ],
        );
        let positioned = SetWindowPos(
            self.hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        if let Err(error) = positioned {
            diagnostic(&format!("Window positioning failed: {error}"));
        }
        // Escape is registered only while visible and is released on dismissal.
        if RegisterHotKey(
            Some(self.hwnd),
            ESCAPE_KEY,
            MOD_NOREPEAT,
            VK_ESCAPE.0 as u32,
        )
        .is_err()
        {
            self.status = "Escape unavailable; use the close button".into();
        }
        self.invalidate();
    }

    #[track_caller]
    unsafe fn hide(&mut self) {
        let _ = KillTimer(Some(self.hwnd), VERIFY);
        diagnostic(&format!(
            "Hiding wheel at {}",
            std::panic::Location::caller()
        ));
        self.visible = false;
        self.mouse_origin = None;
        self.context = None;
        let _ = KillTimer(Some(self.hwnd), ANIMATE);
        self.motion_running = false;
        if self.pending.is_none() {
            let _ = KillTimer(Some(self.hwnd), TICK);
        }
        let _ = UnregisterHotKey(Some(self.hwnd), ESCAPE_KEY);
        let _ = ShowWindow(self.edit, SW_HIDE);
        let _ = ShowWindow(self.search_host, SW_HIDE);
        self.searching = false;
        let _ = ShowWindow(self.hwnd, SW_HIDE);
        // Release device resources when hidden; factories are recreated on demand.
        self.graphics = None;
    }

    unsafe fn begin_search(&mut self) {
        self.mouse_origin = None;
        if self.pending.is_some() {
            return;
        }
        let _ = KillTimer(Some(self.hwnd), TICK);
        self.searching = true;
        self.adding_favorite = false;
        self.results = self.search_results("");
        self.page = 0;
        self.set_context(None);
        self.status = "Type to search · Enter inserts first result".into();
        self.position_edit();
        let _ = SetWindowTextW(self.edit, w!(""));
        let _ = ShowWindow(self.edit, SW_SHOW);
        let _ = ShowWindow(self.search_host, SW_SHOW);
        let _ = SetForegroundWindow(self.search_host);
        let _ = SetFocus(Some(self.edit));
        self.invalidate();
    }

    unsafe fn position_edit(&self) {
        let s = self.dpi as f32 / 96.0;
        let width = if self.adding_favorite { 240.0 } else { 94.0 };
        let mut point = POINT {
            x: ((wheel::SEARCH_CENTER.x - width / 2.0) * s) as i32,
            y: ((wheel::SEARCH_CENTER.y - 15.0) * s) as i32,
        };
        let _ = ClientToScreen(self.hwnd, &mut point);
        let _ = SetWindowPos(
            self.search_host,
            Some(HWND_TOPMOST),
            point.x,
            point.y,
            (width * s) as i32,
            (30.0 * s) as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let _ = SetWindowPos(
            self.edit,
            None,
            0,
            0,
            (width * s) as i32,
            (30.0 * s) as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }

    unsafe fn owns_search_focus(&self) -> bool {
        let foreground = GetForegroundWindow();
        foreground == self.hwnd || foreground == self.search_host
    }

    unsafe fn end_search(&mut self) {
        let restore = self.owns_search_focus();
        self.searching = false;
        self.adding_favorite = false;
        let _ = ShowWindow(self.edit, SW_HIDE);
        let _ = ShowWindow(self.search_host, SW_HIDE);
        if restore {
            if let Some(target) = self.destination {
                if IsWindow(Some(target.window)).as_bool() {
                    let _ = SetForegroundWindow(target.window);
                }
            }
        }
    }

    unsafe fn changed_search(&mut self) {
        if self.adding_favorite {
            return;
        }
        let mut text = [0u16; 256];
        let count = GetWindowTextW(self.edit, &mut text);
        self.results =
            self.search_results(&String::from_utf16_lossy(&text[..count.max(0) as usize]));
        self.page = 0;
        self.set_context(None);
        self.hover = Hit::None;
        self.animate();
    }

    unsafe fn choose(&mut self, text: &str) {
        if self.pending.is_some() {
            return;
        }
        self.mouse_origin = None;
        if !self.searching {
            self.destination = Destination::capture(self.hwnd);
        } else if !self.owns_search_focus() {
            self.end_search();
            self.status = "Search cancelled: focus changed; select again".into();
            self.invalidate();
            return;
        }
        let target = self.destination;
        if self.searching {
            self.end_search();
        }
        let Some(target) = target else {
            self.status = "Focus a text field, then reopen with the shortcut".into();
            self.invalidate();
            return;
        };
        self.pending = Some(Pending {
            text: text.to_owned(),
            target,
            started: Instant::now(),
        });
        self.status = "Release shortcut keys to insert".into();
        SetTimer(Some(self.hwnd), TICK, 16, None);
        self.invalidate();
    }

    unsafe fn tick(&mut self) {
        if let Some(pending) = &self.pending {
            let timed_out = pending.started.elapsed().as_secs() >= 2;
            let held = [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN, VK_SPACE]
                .iter()
                .any(|key| GetAsyncKeyState(key.0 as i32) < 0);
            if held && !timed_out {
                return;
            }
            let pending = self.pending.take().unwrap();
            let _ = KillTimer(Some(self.hwnd), TICK);
            if timed_out || !pending.target.still_focused() {
                self.status = "Cancelled: destination changed or keys stayed held".into();
            } else {
                let inputs: Vec<INPUT> = unicode_units(&pending.text)
                    .into_iter()
                    .flat_map(|unit| {
                        [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP].map(|flags| {
                            INPUT {
                                r#type: INPUT_KEYBOARD,
                                Anonymous: INPUT_0 {
                                    ki: KEYBDINPUT {
                                        wVk: VIRTUAL_KEY(0),
                                        wScan: unit,
                                        dwFlags: flags,
                                        time: 0,
                                        dwExtraInfo: 0,
                                    },
                                },
                            }
                        })
                    })
                    .collect();
                let sent = SendInput(&inputs, size_of::<INPUT>() as i32);
                if sent == inputs.len() as u32 {
                    self.status = format!("Inserted {}", pending.text);
                    if !self.pinned {
                        self.hide();
                    }
                } else {
                    // A partial SendInput can end on a Unicode key down. Release
                    // that packet without retrying any text or duplicating it.
                    if sent > 0 && sent % 2 == 1 {
                        let mut release = inputs[sent as usize - 1];
                        release.Anonymous.ki.dwFlags |= KEYEVENTF_KEYUP;
                        SendInput(&[release], size_of::<INPUT>() as i32);
                    }
                    self.status = "Input failed or was partial; check the target app".into();
                }
            }
            self.invalidate();
        } else if self.mouse_origin.is_some() {
            // No global mouse hook or capture: the nonactivating picker leaves
            // the text field focused. Poll only during a press that began here,
            // including when WM_MOUSEMOVE/UP stop arriving outside the window.
            self.refresh_pointer();
            let primary = if GetSystemMetrics(SM_SWAPBUTTON) != 0 {
                VK_RBUTTON
            } else {
                VK_LBUTTON
            };
            if GetAsyncKeyState(primary.0 as i32) >= 0 {
                self.finish_mouse();
            }
        }
    }

    unsafe fn refresh_pointer(&mut self) {
        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor).is_ok() && ScreenToClient(self.hwnd, &mut cursor).as_bool() {
            let scale = self.dpi as f32 / 96.0;
            self.pointer(Point {
                x: cursor.x as f32 / scale,
                y: cursor.y as f32 / scale,
            });
        } else {
            self.hover = Hit::None;
        }
    }

    unsafe fn begin_mouse(&mut self) {
        self.edge_pager = wheel::EdgePager::default();
        self.gesture_paged = false;
        if self.pending.is_some() {
            return;
        }
        if !self.searching {
            self.destination = Destination::capture(self.hwnd);
        }
        self.refresh_pointer();
        self.mouse_origin = Some(self.hover);
        SetTimer(Some(self.hwnd), TICK, 16, None);
    }

    unsafe fn finish_mouse(&mut self) {
        let Some(origin) = self.mouse_origin.take() else {
            return;
        };
        let _ = KillTimer(Some(self.hwnd), TICK);
        self.refresh_pointer();
        if let Some(target) = wheel::release_target(origin, self.hover) {
            if matches!(target, Hit::Item(_) | Hit::Variant(_))
                && !self.searching
                && !self
                    .destination
                    .is_some_and(|destination| destination.still_focused())
            {
                self.status = "Cancelled: focus changed during the mouse gesture".into();
                self.invalidate();
                return;
            }
            self.hover = target;
            self.click();
        } else if !self.pinned && !self.searching && !self.gesture_paged {
            self.hide();
        } else {
            self.status = "Cancelled · hold mouse and move to choose".into();
            self.invalidate();
        }
    }

    unsafe fn pointer(&mut self, point: Point) {
        if matches!(
            self.mouse_origin,
            Some(Hit::Search | Hit::Item(_) | Hit::Variant(_) | Hit::None)
        ) {
            let primary = if GetSystemMetrics(SM_SWAPBUTTON) != 0 {
                VK_RBUTTON
            } else {
                VK_LBUTTON
            };
            if GetAsyncKeyState(primary.0 as i32) < 0 {
                if let Some(direction) = self.edge_pager.update(point) {
                    self.gesture_paged = true;
                    self.change_page(direction);
                }
            }
        }
        let hit = wheel::hit_test(point, self.items().len(), self.context_layout());
        if hit == self.hover {
            return;
        }
        self.hover = hit;
        if let Hit::Item(index) = hit {
            let context = self
                .items()
                .get(index)
                .and_then(|i| CATALOG.iter().position(|e| e.text == self.item_text(*i)))
                .filter(|i| {
                    !CATALOG[*i].variants.is_empty()
                        || catalog::intensity(CATALOG[*i].text).is_some()
                });
            self.set_context(context);
        }
        if hit == Hit::Search {
            self.set_context(None);
        }
        if let Some(text) = self.selection(hit) {
            let label = if let Hit::Variant(slot) = hit {
                self.choices.get(slot).and_then(|s| s.label)
            } else {
                None
            };
            self.status = format!("{} {text}", label.unwrap_or("Select"));
        }
        self.animate();
    }

    unsafe fn click(&mut self) {
        if self.adding_favorite && matches!(self.hover, Hit::Item(_) | Hit::Variant(_)) {
            return;
        }
        if let Some(text) = self.selection(self.hover) {
            self.choose(&text);
            return;
        }
        match self.hover {
            Hit::Search => self.begin_search(),
            Hit::Favorites => {
                self.end_search();
                self.favorites_mode = !self.favorites_mode;
                self.results = self.search_results("");
                self.page = 0;
                self.set_context(None);
                self.hover = Hit::None;
                self.invalidate();
            }
            Hit::AddFavorite => self.begin_favorite(),
            Hit::Pin => {
                self.pinned = !self.pinned;
                self.mouse_origin = None;
                if self.pending.is_none() {
                    let _ = KillTimer(Some(self.hwnd), TICK);
                }
                self.status = if self.pinned {
                    "Pinned · click or drag to select"
                } else {
                    "Unpinned · next selection closes"
                }
                .into();
                self.invalidate();
            }
            Hit::Close => {
                self.end_search();
                self.pending = None;
                self.hide();
            }
            Hit::Previous => self.change_page(-1),
            Hit::Next => self.change_page(1),
            _ => (),
        }
    }

    unsafe fn change_page(&mut self, delta: i32) {
        let count = catalog::page_count(&self.results) as i32;
        self.page = (self.page as i32 + delta).rem_euclid(count) as usize;
        self.set_context(None);
        self.hover = Hit::None;
        self.animate();
    }

    unsafe fn paint(&mut self) -> Result<()> {
        let first_frame = self.graphics.is_none();
        if first_frame {
            self.graphics = Some(Graphics::new(self.hwnd, self.dpi)?);
        }
        let graphics = self.graphics.as_ref().unwrap();
        if let Err(error) = graphics.draw(self) {
            if self.transparent && self.smooth {
                diagnostic(&format!(
                    "Smooth presentation failed, restoring compatibility: {error}"
                ));
                self.smooth = false;
                self.set_transparency(true);
                return self.paint();
            }
            return Err(error);
        }
        if first_frame && self.transparent && self.smooth {
            let _ = SetTimer(Some(self.hwnd), VERIFY, 160, None);
        }
        if first_frame {
            diagnostic(&format!(
                "Presented wheel: {}x{}, DPI {}, window visible {}, transparent {}, center alpha {}",
                graphics.surface.width,
                graphics.surface.height,
                self.dpi,
                IsWindowVisible(self.hwnd).as_bool(),
                self.transparent,
                graphics.surface.alpha(graphics.surface.width as usize / 2,
                    (wheel::CENTER.y * self.dpi as f32 / 96.0) as usize)
            ));
        }
        Ok(())
    }
}

struct Graphics {
    target: ID2D1DCRenderTarget,
    text: IDWriteTextFormat,
    small: IDWriteTextFormat,
    emoji: IDWriteTextFormat,
    write: IDWriteFactory,
    brush: ID2D1SolidColorBrush,
    // Drop the COM target and brush before releasing their bound GDI surface.
    surface: Surface,
}

/// Premultiplied BGRA backing store for a per pixel alpha layered window.
struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pixels: *mut u8,
    width: i32,
    height: i32,
}

impl Surface {
    unsafe fn new(width: i32, height: i32) -> Result<Self> {
        let dc = CreateCompatibleDC(None);
        if dc.0.is_null() {
            return Err(Error::from_thread());
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut pixels, None, 0) {
            Ok(bitmap) => bitmap,
            Err(error) => {
                let _ = DeleteDC(dc);
                return Err(error);
            }
        };
        let previous = SelectObject(dc, HGDIOBJ(bitmap.0));
        if pixels.is_null() || previous.0.is_null() || previous.0 as isize == -1 {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(dc);
            return Err(Error::from_thread());
        }
        Ok(Self {
            dc,
            bitmap,
            previous,
            pixels: pixels.cast(),
            width,
            height,
        })
    }

    unsafe fn present(&self, hwnd: HWND, smooth: bool) -> Result<()> {
        if smooth {
            let mut bounds = RECT::default();
            GetWindowRect(hwnd, &mut bounds)?;
            let position = POINT {
                x: bounds.left,
                y: bounds.top,
            };
            let size = SIZE {
                cx: self.width,
                cy: self.height,
            };
            let origin = POINT::default();
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let _ = GdiFlush();
            // Supply the actual surface extent even on the first frame. A
            // successful API return with omitted geometry proved insufficient.
            PRESENTING.set(true);
            let result = UpdateLayeredWindow(
                hwnd,
                None,
                Some(&position),
                Some(&size),
                Some(self.dc),
                Some(&origin),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );
            PRESENTING.set(false);
            return result;
        }
        let destination = GetDC(Some(hwnd));
        if destination.0.is_null() {
            return Err(Error::from_thread());
        }
        let result = BitBlt(
            destination,
            0,
            0,
            self.width,
            self.height,
            Some(self.dc),
            0,
            0,
            SRCCOPY,
        );
        let _ = ReleaseDC(Some(hwnd), destination);
        result
    }

    unsafe fn alpha(&self, x: usize, y: usize) -> u8 {
        assert!(x < self.width as usize && y < self.height as usize);
        *self.pixels.add((y * self.width as usize + x) * 4 + 3)
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            let _ = SelectObject(self.dc, self.previous);
            let _ = DeleteObject(HGDIOBJ(self.bitmap.0));
            let _ = DeleteDC(self.dc);
        }
    }
}

fn color(rgb: u32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: ((rgb >> 16) & 255) as f32 / 255.0,
        g: ((rgb >> 8) & 255) as f32 / 255.0,
        b: (rgb & 255) as f32 / 255.0,
        a: 1.0,
    }
}
fn blend(from: u32, to: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    [16, 8, 0].into_iter().fold(0, |rgb, shift| {
        let a = ((from >> shift) & 255) as f32;
        let b = ((to >> shift) & 255) as f32;
        rgb | (((a + (b - a) * t).round() as u32) << shift)
    })
}
fn rect(x: f32, y: f32, width: f32, height: f32) -> D2D_RECT_F {
    D2D_RECT_F {
        left: x,
        top: y,
        right: x + width,
        bottom: y + height,
    }
}

impl Graphics {
    unsafe fn new(hwnd: HWND, dpi: u32) -> Result<Self> {
        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let write: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let mut bounds = RECT::default();
        GetClientRect(hwnd, &mut bounds)?;
        let surface = Surface::new(bounds.right.max(1), bounds.bottom.max(1))?;
        let target = factory.CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: dpi as f32,
            dpiY: dpi as f32,
            ..Default::default()
        })?;
        target.BindDC(surface.dc, &bounds)?;
        target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        let format = |font: PCWSTR, size: f32| -> Result<IDWriteTextFormat> {
            let format = write.CreateTextFormat(
                font,
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                w!("en-US"),
            )?;
            format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            Ok(format)
        };
        Ok(Self {
            brush: target.CreateSolidColorBrush(&color(0xf4f3ff), None)?,
            target,
            text: format(w!("Segoe UI"), 15.0)?,
            small: format(w!("Segoe UI"), 11.0)?,
            emoji: format(w!("Segoe UI Emoji"), 23.0)?,
            write,
            surface,
        })
    }

    unsafe fn label(&self, text: &str, bounds: D2D_RECT_F, format: &IDWriteTextFormat, rgb: u32) {
        self.label_alpha(text, bounds, format, rgb, 1.0);
    }

    unsafe fn label_alpha(
        &self,
        text: &str,
        bounds: D2D_RECT_F,
        format: &IDWriteTextFormat,
        rgb: u32,
        alpha: f32,
    ) {
        self.brush.SetColor(&color(rgb));
        self.brush.SetOpacity(alpha);
        self.target.DrawText(
            &unicode_units(text),
            format,
            &bounds,
            &self.brush,
            D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT | D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }

    /// Fit actual shaped glyphs, including ZWJ emojis and combinations, to a
    /// smaller target without guessing cluster counts or clipping the output.
    unsafe fn fitted_emoji(&self, text: &str, point: Point, radius: f32) -> Result<()> {
        let layout =
            self.write
                .CreateTextLayout(&unicode_units(text), &self.emoji, 8192.0, 128.0)?;
        layout.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_LEADING)?;
        layout.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_NEAR)?;
        let mut metrics = DWRITE_TEXT_METRICS::default();
        layout.GetMetrics(&mut metrics)?;
        let available = 2.0 * radius - 6.0;
        let scale = (available / metrics.width.max(1.0))
            .min(available / metrics.height.max(1.0))
            .min(1.0);
        self.brush.SetColor(&color(0xffffff));
        self.brush.SetOpacity(1.0);
        self.target.SetTransform(&Matrix3x2 {
            M11: scale,
            M12: 0.0,
            M21: 0.0,
            M22: scale,
            M31: point.x - (metrics.left + metrics.width / 2.0) * scale,
            M32: point.y - (metrics.top + metrics.height / 2.0) * scale,
        });
        self.target.DrawTextLayout(
            Vector2 { X: 0.0, Y: 0.0 },
            &layout,
            &self.brush,
            D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
        );
        self.target.SetTransform(&Matrix3x2::identity());
        Ok(())
    }

    unsafe fn circle(&self, point: Point, radius: f32, rgb: u32) {
        self.circle_alpha(point, radius, rgb, 1.0);
    }

    unsafe fn circle_alpha(&self, point: Point, radius: f32, rgb: u32, alpha: f32) {
        self.brush.SetColor(&color(rgb));
        self.brush.SetOpacity(alpha);
        self.target.FillEllipse(
            &D2D1_ELLIPSE {
                point: Vector2 {
                    X: point.x,
                    Y: point.y,
                },
                radiusX: radius,
                radiusY: radius,
            },
            &self.brush,
        );
    }

    unsafe fn draw(&self, app: &App) -> Result<()> {
        self.target.BeginDraw();
        self.target.Clear(Some(&if app.transparent && app.smooth {
            D2D1_COLOR_F::default()
        } else {
            color(0x12131e)
        }));
        // The title, close, and utility controls live in the permanent hub.
        // Outward suggestions never share this space, so hovering cannot hide it.
        self.pill(rect(166.0, 226.0, 108.0, 20.0));
        self.label(
            if app.favorites_mode {
                "Favorites"
            } else {
                "EmOrbit"
            },
            rect(168.0, 226.0, 78.0, 20.0),
            &self.small,
            0xf3f2ff,
        );
        self.label("×", rect(246.0, 226.0, 28.0, 20.0), &self.small, 0xaaaabb);
        if matches!(app.hover, Hit::Variant(_))
            || app.adding_favorite
            || (app.favorites_mode && matches!(app.hover, Hit::Item(_)))
        {
            self.pill(rect(90.0, 46.0, 260.0, 26.0));
            self.label(
                &app.status,
                rect(94.0, 46.0, 252.0, 26.0),
                &self.small,
                0xe4d9ff,
            );
        }
        // A subtle orbit joins the individual emoji targets.
        self.brush.SetColor(&color(0x303345));
        self.brush.SetOpacity(1.0);
        self.target.DrawEllipse(
            &D2D1_ELLIPSE {
                point: Vector2 {
                    X: wheel::CENTER.x,
                    Y: wheel::CENTER.y,
                },
                radiusX: wheel::ORBIT_RADIUS,
                radiusY: wheel::ORBIT_RADIUS,
            },
            &self.brush,
            1.5,
            None,
        );
        for (slot, index) in app.items().iter().enumerate() {
            let p = wheel::item_position(slot);
            let highlight = app.motion.items[slot].value;
            let radius = wheel::ITEM_RADIUS + 2.0 * highlight;
            self.circle_alpha(p, radius + 7.0, 0xa084fa, 0.11 * highlight);
            self.circle(p, radius, blend(0x292c3e, 0x8060d8, highlight));
            self.fitted_emoji(app.item_text(*index), p, wheel::ITEM_RADIUS)?;
            if app
                .context
                .is_some_and(|anchor| CATALOG[anchor].text != app.item_text(*index))
            {
                self.circle_alpha(p, radius + 1.0, 0x13151f, 0.58 * app.motion.expansion.value);
            }
        }
        let variants = app.variants();
        if let Some(layout) = app.context_layout() {
            let parent = wheel::item_position(layout.anchor);
            let base = CATALOG[app.context.unwrap()].text;
            for (slot, suggestion) in variants.iter().enumerate() {
                let value = suggestion.text;
                let p = wheel::variant_position(layout, slot);
                let highlight = app.motion.variants[slot].value;
                let expansion = app.motion.expansion.value;
                let radius = wheel::VARIANT_RADIUS * (0.75 + 0.25 * expansion) + 2.0 * highlight;
                self.brush.SetColor(&color(0x766296));
                self.brush.SetOpacity(0.20 * expansion);
                let distance = (p.x - parent.x).hypot(p.y - parent.y);
                let dx = (p.x - parent.x) / distance;
                let dy = (p.y - parent.y) / distance;
                self.target.DrawLine(
                    Vector2 {
                        X: parent.x + dx * (wheel::ITEM_RADIUS + 5.0),
                        Y: parent.y + dy * (wheel::ITEM_RADIUS + 5.0),
                    },
                    Vector2 {
                        X: p.x - dx * (radius + 2.0),
                        Y: p.y - dy * (radius + 2.0),
                    },
                    &self.brush,
                    1.0,
                    None,
                );
                self.circle_alpha(p, radius + 5.0, 0xb49aff, 0.12 * highlight * expansion);
                self.circle_alpha(p, radius, blend(0x353049, 0x8962e6, highlight), expansion);
                if expansion > 0.08 {
                    // Repeated output is shown as one emoji and a compact badge,
                    // so it stays inside the bubble. The header previews the
                    // actual complete sequence when selected.
                    let repeated = (2..=3).find(|count| value == base.repeat(*count));
                    self.label_alpha(
                        suggestion
                            .label
                            .unwrap_or(if repeated.is_some() { base } else { value }),
                        rect(p.x - 22.0, p.y - 22.0, 44.0, 44.0),
                        if suggestion.label.is_some() {
                            &self.small
                        } else {
                            &self.emoji
                        },
                        0xffffff,
                        expansion,
                    );
                    if let Some(count) = repeated {
                        let badge = Point {
                            x: p.x + 16.0,
                            y: p.y + 16.0,
                        };
                        self.circle_alpha(badge, 11.0, 0x1d192c, expansion);
                        self.label_alpha(
                            &format!("×{count}"),
                            rect(badge.x - 12.0, badge.y - 10.0, 24.0, 20.0),
                            &self.small,
                            0xe2d5ff,
                            expansion,
                        );
                    }
                }
            }
        }
        self.circle(
            wheel::SEARCH_CENTER,
            wheel::SEARCH_RADIUS,
            blend(0x373049, 0x66519d, app.motion.search.value),
        );
        if !app.searching {
            self.brush.SetOpacity(1.0);
            self.brush.SetColor(&color(0xe4d9ff));
            self.target.DrawEllipse(
                &D2D1_ELLIPSE {
                    point: Vector2 {
                        X: wheel::SEARCH_CENTER.x - 3.0,
                        Y: wheel::SEARCH_CENTER.y - 3.0,
                    },
                    radiusX: 10.0,
                    radiusY: 10.0,
                },
                &self.brush,
                2.5,
                None,
            );
            self.target.DrawLine(
                Vector2 {
                    X: wheel::SEARCH_CENTER.x + 4.0,
                    Y: wheel::SEARCH_CENTER.y + 4.0,
                },
                Vector2 {
                    X: wheel::SEARCH_CENTER.x + 15.0,
                    Y: wheel::SEARCH_CENTER.y + 15.0,
                },
                &self.brush,
                3.0,
                None,
            );
        }
        for (point, radius, hit, label, active) in [
            (
                wheel::FAVORITES_CENTER,
                14.0,
                Hit::Favorites,
                "★",
                app.favorites_mode,
            ),
            (wheel::ADD_CENTER, 12.0, Hit::AddFavorite, "+", false),
            (
                wheel::PIN_CENTER,
                14.0,
                Hit::Pin,
                if app.pinned { "Unpin" } else { "Pin" },
                app.pinned,
            ),
        ] {
            self.circle(
                point,
                radius,
                if active || app.hover == hit {
                    0x66519d
                } else {
                    0x292c3e
                },
            );
            self.label(
                label,
                rect(
                    point.x - radius,
                    point.y - radius,
                    radius * 2.0,
                    radius * 2.0,
                ),
                if hit == Hit::Pin {
                    &self.small
                } else {
                    &self.text
                },
                0xe4d9ff,
            );
        }
        if app.results.is_empty() {
            self.label(
                if app.favorites_mode {
                    "No favorites found · use + to add"
                } else {
                    "No matches in the prototype catalog"
                },
                rect(50.0, 350.0, 340.0, 28.0),
                &self.small,
                0xb0aec0,
            );
        }
        if app.context.is_none() && !app.adding_favorite {
            for (x, text, hit) in [(20.0, "‹", Hit::Previous), (420.0, "›", Hit::Next)] {
                self.circle(
                    Point {
                        x,
                        y: wheel::CENTER.y,
                    },
                    18.0,
                    if app.hover == hit { 0x66519d } else { 0x292c3e },
                );
                self.label(
                    text,
                    rect(x - 18.0, wheel::CENTER.y - 18.0, 36.0, 36.0),
                    &self.text,
                    0xe4d9ff,
                );
            }
            self.pill(rect(188.0, 439.0, 64.0, 24.0));
            self.label(
                &format!("{} / {}", app.page + 1, catalog::page_count(&app.results)),
                rect(188.0, 439.0, 64.0, 24.0),
                &self.small,
                0xa9a6ba,
            );
        }
        if app.status.starts_with("Favorite") || app.status.starts_with("Could not save") {
            self.pill(rect(70.0, 474.0, 300.0, 28.0));
            self.label(
                &app.status,
                rect(74.0, 474.0, 292.0, 28.0),
                &self.small,
                0xe4d9ff,
            );
        }
        self.target.EndDraw(None, None)?;
        let center_x = self.surface.width as usize / 2;
        let center_y = (wheel::CENTER.y * app.dpi as f32 / 96.0) as usize;
        if self.surface.alpha(center_x, center_y) == 0 {
            return Err(Error::new(
                E_FAIL,
                "The rendered wheel has an empty alpha channel",
            ));
        }
        self.surface
            .present(app.hwnd, app.transparent && app.smooth)
    }

    unsafe fn pill(&self, bounds: D2D_RECT_F) {
        self.brush.SetOpacity(0.94);
        self.brush.SetColor(&color(0x171923));
        self.target.FillRoundedRectangle(
            &D2D1_ROUNDED_RECT {
                rect: bounds,
                radiusX: 12.0,
                radiusY: 12.0,
            },
            &self.brush,
        );
    }
}

unsafe extern "system" fn on_foreground(
    _: HWINEVENTHOOK,
    _: u32,
    _: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    let hwnd = HWND(WINDOW.load(Ordering::Relaxed));
    if !hwnd.0.is_null() {
        let _ = PostMessageW(Some(hwnd), FOREGROUND, WPARAM(0), LPARAM(0));
    }
}

unsafe extern "system" fn edit_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    _: usize,
    data: usize,
) -> LRESULT {
    if msg == WM_KEYDOWN {
        let message = match wp.0 as u16 {
            13 => Some(SEARCH_ENTER),
            27 => Some(SEARCH_ESCAPE),
            _ => None,
        };
        if let Some(message) = message {
            let _ = PostMessageW(Some(HWND(data as *mut _)), message, WPARAM(0), LPARAM(0));
            return LRESULT(0);
        }
    }
    // Prevent Enter and Escape generating an EDIT control beep.
    if msg == WM_CHAR && (wp.0 == 13 || wp.0 == 27) {
        return LRESULT(0);
    }
    DefSubclassProc(hwnd, msg, wp, lp)
}

// Keep the native edit in a tiny owned popup. A child EDIT cannot paint itself
// into the bitmap that is copied to the wheel window.
unsafe extern "system" fn search_host_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> LRESULT {
    if let Ok(owner) = GetWindow(hwnd, GW_OWNER) {
        if msg == WM_COMMAND {
            let _ = PostMessageW(Some(owner), WM_COMMAND, wp, lp);
            return LRESULT(0);
        }
        if msg == WM_CLOSE {
            let _ = PostMessageW(Some(owner), SEARCH_ESCAPE, WPARAM(0), LPARAM(0));
            return LRESULT(0);
        }
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    if msg == EXIT {
        let _ = DestroyWindow(hwnd);
        return LRESULT(0);
    }
    if msg == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<App>;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    // Native calls may synchronously reenter this procedure. RefCell prevents
    // aliased mutable references; size work is deferred until the owner returns.
    let Ok(mut state) = (*ptr).try_borrow_mut() else {
        if msg == WM_SIZE && PRESENTING.get() {
            return LRESULT(0);
        }
        if msg == WM_SIZE || msg == WM_DPICHANGED {
            let _ = PostMessageW(Some(hwnd), REFRESH_SIZE, WPARAM(0), LPARAM(0));
            return LRESULT(0);
        }
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    let app = &mut *state;
    match msg {
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => return LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut paint);
            let _ = EndPaint(hwnd, &paint);
            if app.visible {
                app.invalidate();
            }
            return LRESULT(0);
        }
        RENDER => {
            app.render_queued.set(false);
            if app.visible {
                if let Err(error) = app.paint() {
                    app.end_search();
                    app.hide();
                    report_error(&format!("Could not render the transparent wheel: {error}"));
                }
            }
            return LRESULT(0);
        }
        WM_HOTKEY if wp.0 == HOTKEY as usize => {
            if app.visible {
                app.end_search();
                app.pending = None;
                app.hide();
            } else {
                app.show();
            }
        }
        WM_HOTKEY if wp.0 == ESCAPE_KEY as usize => {
            if app.searching {
                app.end_search();
                app.results = app.search_results("");
                app.page = 0;
                app.invalidate();
            } else {
                app.pending = None;
                app.hide();
            }
        }
        WM_MOUSEMOVE => {
            let _ = TrackMouseEvent(&mut TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            });
            let scale = app.dpi as f32 / 96.0;
            app.pointer(Point {
                x: (lp.0 as u16 as i16) as f32 / scale,
                y: ((lp.0 >> 16) as u16 as i16) as f32 / scale,
            });
        }
        WM_MOUSELEAVE if app.mouse_origin.is_none() => {
            // Alpha-zero gaps pass mouse messages to the underlying app. Keep
            // the fan available while crossing those gaps towards a variant.
            let mut cursor = POINT::default();
            let inside = GetCursorPos(&mut cursor).is_ok()
                && ScreenToClient(hwnd, &mut cursor).as_bool()
                && cursor.x >= 0
                && cursor.y >= 0
                && (cursor.x as f32) < wheel::WIDTH * app.dpi as f32 / 96.0
                && (cursor.y as f32) < wheel::HEIGHT * app.dpi as f32 / 96.0;
            if !inside {
                app.set_context(None);
            }
            app.refresh_pointer();
        }
        WM_LBUTTONDOWN if app.hover == Hit::Drag => {
            app.mouse_origin = None;
            if app.pending.is_none() {
                let _ = KillTimer(Some(hwnd), TICK);
            }
            let _ = ReleaseCapture();
            let _ = SendMessageW(
                hwnd,
                WM_NCLBUTTONDOWN,
                Some(WPARAM(HTCAPTION as usize)),
                None,
            );
            if app.searching {
                app.position_edit();
            }
            app.render_queued.set(false);
            app.invalidate();
        }
        WM_LBUTTONDOWN => app.begin_mouse(),
        WM_LBUTTONUP => app.finish_mouse(),
        WM_RBUTTONUP if !app.searching => app.favorite_menu(),
        WM_MOUSEWHEEL => app.change_page(if ((wp.0 >> 16) as u16 as i16) > 0 {
            -1
        } else {
            1
        }),
        WM_TIMER if wp.0 == TICK => app.tick(),
        WM_TIMER if wp.0 == ANIMATE => app.animation_frame(),
        WM_TIMER if wp.0 == VERIFY => app.verify_presentation(),
        WM_SETTINGCHANGE => app.motion_preference(),
        WM_COMMAND if (wp.0 >> 16) as u32 == EN_CHANGE => app.changed_search(),
        SEARCH_ENTER => {
            if app.adding_favorite {
                app.save_favorite_entry();
            } else if let Some(index) = app.items().first() {
                let text = app.item_text(*index).to_owned();
                app.choose(&text);
            }
        }
        SEARCH_ESCAPE => {
            app.end_search();
            app.results = app.search_results("");
            app.page = 0;
            app.invalidate();
        }
        FOREGROUND if app.visible && !app.searching && app.pending.is_none() => {
            if let Some(target) = Destination::capture(hwnd) {
                if app.mouse_origin.is_some() {
                    app.mouse_origin = None;
                    let _ = KillTimer(Some(hwnd), TICK);
                    app.status = "Gesture cancelled: destination changed".into();
                    if !app.pinned {
                        app.hide();
                    } else {
                        app.destination = Some(target);
                        app.invalidate();
                    }
                } else if app.pinned {
                    app.destination = Some(target);
                } else if app
                    .destination
                    .is_some_and(|old| old.window != target.window)
                {
                    app.hide();
                }
            }
        }
        SHOW => {
            if !app.visible {
                app.show();
            }
        }
        WM_DPICHANGED => {
            app.dpi = (wp.0 & 0xffff) as u32;
            let bounds = &*(lp.0 as *const RECT);
            let _ = SetWindowPos(
                hwnd,
                None,
                bounds.left,
                bounds.top,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            app.graphics = None;
            app.position_edit();
            app.invalidate();
        }
        WM_SIZE => {
            app.graphics = None;
            app.invalidate();
        }
        WM_MOVE if app.searching => app.position_edit(),
        REFRESH_SIZE => {
            let dpi = GetDpiForWindow(hwnd).max(96);
            if dpi != app.dpi {
                app.dpi = dpi;
                let scale = dpi as f32 / 96.0;
                let _ = SetWindowPos(
                    hwnd,
                    None,
                    0,
                    0,
                    (wheel::WIDTH * scale) as i32,
                    (wheel::HEIGHT * scale) as i32,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
                app.position_edit();
            }
            app.graphics = None;
            app.invalidate();
        }
        TRAY => {
            if lp.0 as u32 == WM_LBUTTONUP {
                if !app.visible {
                    app.show();
                }
            } else if lp.0 as u32 == WM_RBUTTONUP {
                if let Ok(menu) = CreatePopupMenu() {
                    let _ = AppendMenuW(menu, MF_STRING, 1, w!("Open EmOrbit"));
                    let flags = if app.transparent {
                        MF_STRING | MF_CHECKED
                    } else {
                        MF_STRING
                    };
                    let _ = AppendMenuW(menu, flags, 3, w!("Floating background"));
                    let smooth_flags = if app.smooth {
                        MF_STRING | MF_CHECKED
                    } else {
                        MF_STRING
                    };
                    let _ = AppendMenuW(menu, smooth_flags, 4, w!("Smooth edges"));
                    let _ = AppendMenuW(menu, MF_STRING, 2, w!("Exit"));
                    let mut point = POINT::default();
                    let _ = GetCursorPos(&mut point);
                    let _ = SetForegroundWindow(hwnd);
                    let command = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_RIGHTBUTTON,
                        point.x,
                        point.y,
                        Some(0),
                        hwnd,
                        None,
                    )
                    .0;
                    let _ = DestroyMenu(menu);
                    app.render_queued.set(false);
                    app.invalidate();
                    if command == 2 {
                        let _ = PostMessageW(Some(hwnd), EXIT, WPARAM(0), LPARAM(0));
                    } else if command == 1 {
                        app.show();
                    } else if command == 4 {
                        app.end_search();
                        app.smooth = !app.smooth;
                        app.set_transparency(true);
                        app.show();
                    } else if command == 3 {
                        app.end_search();
                        app.set_transparency(!app.transparent);
                        app.show();
                    }
                }
            }
        }
        WM_CLOSE => {
            app.end_search();
            app.pending = None;
            app.hide();
        }
        WM_DESTROY => {
            let _ = Shell_NotifyIconW(NIM_DELETE, &app.tray);
            PostQuitMessage(0);
        }
        _ => return DefWindowProcW(hwnd, msg, wp, lp),
    }
    LRESULT(0)
}

pub fn run() -> Result<()> {
    diagnostic(concat!("Starting EmOrbit ", env!("CARGO_PKG_VERSION")));
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if let Ok(existing) = FindWindowW(w!("EmOrbit.Prototype"), None) {
            diagnostic("Requesting existing instance to open");
            PostMessageW(Some(existing), SHOW, WPARAM(0), LPARAM(0))?;
            return Ok(());
        }
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            lpszClassName: w!("EmOrbit.Prototype"),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(Error::from_thread());
        }
        let search_class = WNDCLASSW {
            lpfnWndProc: Some(search_host_proc),
            hInstance: instance,
            lpszClassName: w!("EmOrbit.Search"),
            hCursor: LoadCursorW(None, IDC_IBEAM)?,
            ..Default::default()
        };
        if RegisterClassW(&search_class) == 0 {
            return Err(Error::from_thread());
        }
        let mut initial = App::new();
        if let Some(path) = App::favorites_path() {
            match Favorites::load(&path) {
                Ok(store) => initial.favorites = store,
                Err(error) => diagnostic(&format!("Could not load favorites: {error}")),
            }
        }
        let app = Box::new(RefCell::new(initial));
        // Hold the borrow during creation so synchronous setup messages cannot
        // access an incompletely initialized App.
        let mut state = app.borrow_mut();
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class.lpszClassName,
            w!("EmOrbit"),
            WS_POPUP | WS_CLIPCHILDREN,
            0,
            0,
            wheel::WIDTH as i32,
            wheel::HEIGHT as i32,
            None,
            None,
            Some(instance),
            Some((&*app as *const RefCell<App>).cast()),
        )?;
        state.hwnd = hwnd;
        state.set_transparency(true);
        state.motion_preference();
        WINDOW.store(hwnd.0, Ordering::Relaxed);
        // Install callbacks only after every fallible setup step is complete.
        let setup = (|| -> Result<()> {
            let app = &mut *state;
            app.search_host = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                search_class.lpszClassName,
                w!("Search emojis"),
                WS_POPUP | WS_CLIPCHILDREN,
                0,
                0,
                94,
                30,
                Some(hwnd),
                None,
                Some(instance),
                None,
            )?;
            app.edit = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("EDIT"),
                w!(""),
                WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                0,
                0,
                94,
                30,
                Some(app.search_host),
                Some(HMENU(100usize as *mut _)),
                Some(instance),
                None,
            )?;
            if !SetWindowSubclass(app.edit, Some(edit_proc), 1, hwnd.0 as usize).as_bool() {
                return Err(Error::from_thread());
            }
            SendMessageW(app.edit, EM_SETLIMITTEXT, Some(WPARAM(255)), None);
            RegisterHotKey(
                Some(hwnd),
                HOTKEY,
                MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
                VK_SPACE.0 as u32,
            )?;
            app.tray = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
                uCallbackMessage: TRAY,
                hIcon: LoadIconW(None, IDI_APPLICATION)?,
                ..Default::default()
            };
            let tip: Vec<u16> = "EmOrbit · Ctrl + Alt + Space".encode_utf16().collect();
            app.tray.szTip[..tip.len()].copy_from_slice(&tip);
            if !Shell_NotifyIconW(NIM_ADD, &app.tray).as_bool() {
                return Err(Error::from_thread());
            }
            Ok(())
        })();
        drop(state);
        if let Err(error) = setup {
            let _ = UnregisterHotKey(Some(hwnd), HOTKEY);
            let _ = DestroyWindow(hwnd);
            WINDOW.store(std::ptr::null_mut(), Ordering::Relaxed);
            return Err(error);
        }
        let hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(on_foreground),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        if hook.0.is_null() {
            app.borrow_mut().status =
                "Foreground tracking unavailable; reopen for each destination".into();
        }
        // Launching from Setup or the Start menu must visibly open the picker.
        // Posting after setup lets native creation and sizing messages settle.
        diagnostic("Initialized tray and hotkey; opening initial wheel");
        PostMessageW(Some(hwnd), SHOW, WPARAM(0), LPARAM(0))?;
        let mut msg = MSG::default();
        let loop_result = loop {
            let result = GetMessageW(&mut msg, None, 0, 0).0;
            if result == -1 {
                break Err(Error::from_thread());
            }
            if result == 0 {
                break Ok(());
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        };
        if !hook.0.is_null() {
            let _ = UnhookWinEvent(hook);
        }
        let _ = UnregisterHotKey(Some(hwnd), HOTKEY);
        let _ = UnregisterHotKey(Some(hwnd), ESCAPE_KEY);
        if IsWindow(Some(hwnd)).as_bool() {
            let _ = DestroyWindow(hwnd);
        }
        WINDOW.store(std::ptr::null_mut(), Ordering::Relaxed);
        loop_result
    }
}

pub fn report_error(message: &str) {
    diagnostic(&format!("Error: {message}"));
    let message: Vec<u16> = format!(
        "EmOrbit could not start.\n\n{message}\n\nThe shortcut may already be used by another app."
    )
    .encode_utf16()
    .chain(Some(0))
    .collect();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            w!("EmOrbit"),
            MB_OK | MB_ICONERROR,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "system" fn test_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        DefWindowProcW(hwnd, msg, wp, lp)
    }

    struct TestWindow(HWND);
    impl Drop for TestWindow {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }

    #[test]
    fn custom_favorites_are_selectable_and_known_ones_keep_their_variants() {
        unsafe {
            let mut app = App::new();
            app.motion_enabled = false;
            app.favorites.add("🫠").unwrap();
            app.favorites.add("👍").unwrap();
            app.favorites_mode = true;
            app.results = app.search_results("");
            assert_eq!(app.selection(Hit::Item(0)).as_deref(), Some("🫠"));
            app.pointer(wheel::item_position(0));
            assert!(app.context.is_none());
            app.pointer(wheel::item_position(1));
            assert_eq!(app.variants().len(), 6);
            assert_eq!(app.context_layout().unwrap().anchor, 1);
            app.results = app.search_results("thumb");
            assert_eq!(app.selection(Hit::Item(0)).as_deref(), Some("👍"));
        }
    }

    #[test]
    fn real_window_opens_renders_and_settles_after_reopening() {
        unsafe {
            let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance,
                lpszClassName: w!("EmOrbit.LifecycleTest"),
                ..Default::default()
            };
            assert_ne!(RegisterClassW(&class), 0);
            let app = Box::new(RefCell::new(App::new()));
            let window = TestWindow(
                CreateWindowExW(
                    WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                    class.lpszClassName,
                    w!("EmOrbit lifecycle test"),
                    WS_POPUP,
                    0,
                    0,
                    wheel::WIDTH as i32,
                    wheel::HEIGHT as i32,
                    None,
                    None,
                    Some(instance),
                    Some((&*app as *const RefCell<App>).cast()),
                )
                .unwrap(),
            );
            app.borrow_mut().hwnd = window.0;
            for (transparent, smooth) in [(false, true), (true, true), (true, false), (false, true)]
            {
                app.borrow_mut().smooth = smooth;
                app.borrow_mut().set_transparency(transparent);
                if transparent && !smooth {
                    let mut key = COLORREF(0);
                    let mut flags = LAYERED_WINDOW_ATTRIBUTES_FLAGS(0);
                    GetLayeredWindowAttributes(window.0, Some(&mut key), None, Some(&mut flags))
                        .unwrap();
                    assert_eq!(key, COLORREF(0x1e1312));
                    assert_eq!(flags, LWA_COLORKEY);
                }
                SendMessageW(window.0, SHOW, None, None);
                let mut processed = 0;
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, Some(window.0), 0, 0, PM_REMOVE).as_bool() {
                    DispatchMessageW(&msg);
                    processed += 1;
                    assert!(processed < 100, "window never settles after opening");
                }
                let state = app.borrow();
                assert!(state.visible);
                assert!(IsWindowVisible(window.0).as_bool());
                assert!(!state.render_queued.get());
                assert!(
                    state.graphics.is_some(),
                    "opening must present the first frame"
                );
                drop(state);
                app.borrow_mut().hide();
            }
        }
    }

    #[test]
    fn native_surface_has_smooth_edges_solid_controls_and_inline_reactions() {
        unsafe {
            let instance = HINSTANCE(GetModuleHandleW(None).unwrap().0);
            let class = WNDCLASSW {
                lpfnWndProc: Some(test_proc),
                hInstance: instance,
                lpszClassName: w!("EmOrbit.RenderTest"),
                ..Default::default()
            };
            assert_ne!(RegisterClassW(&class), 0);
            let window = TestWindow(
                CreateWindowExW(
                    WS_EX_LAYERED | WS_EX_NOACTIVATE,
                    class.lpszClassName,
                    w!("EmOrbit render test"),
                    WS_POPUP,
                    0,
                    0,
                    wheel::WIDTH as i32,
                    wheel::HEIGHT as i32,
                    None,
                    None,
                    Some(instance),
                    None,
                )
                .unwrap(),
            );
            let mut app = App::new();
            app.hwnd = window.0;
            app.set_transparency(true);
            app.visible = true;
            app.motion_enabled = false;
            // No wait or dwell message is needed to create the parent context.
            app.pointer(wheel::item_position(2));
            assert_eq!(app.context, Some(2));
            assert_eq!(app.motion.expansion.value, 1.0);
            app.set_context(None);
            for item in &mut app.motion.items {
                item.reset(0.0);
            }
            app.motion.items[0].reset(1.0);
            let graphics = Graphics::new(window.0, 96).unwrap();
            graphics.draw(&app).unwrap();
            assert_eq!(graphics.surface.alpha(0, 0), 0);
            assert_eq!(graphics.surface.alpha(5, 200), 0);
            assert!(
                (166..180).any(|x| (226..240).any(|y| {
                    let alpha = graphics.surface.alpha(x, y);
                    alpha > 0 && alpha < 240
                })),
                "toolbar corners must have fractional alpha"
            );
            assert_eq!(graphics.surface.alpha(220, 285), 255);
            assert_eq!(graphics.surface.alpha(220, 185), 255);
            let glow_alpha = graphics.surface.alpha(249, 185);
            assert!(glow_alpha > 0 && glow_alpha < 255);
            let icon = graphics
                .surface
                .pixels
                .add((276 * graphics.surface.width as usize + 227) * 4);
            assert!(
                *icon > 150 && *icon.add(2) > 150,
                "magnifier stroke must be drawn"
            );
            app.pointer(wheel::item_position(3));
            assert!(app
                .variants()
                .iter()
                .any(|s| s.text == "😭💔🥀" && s.label == Some("Saddest")));
            let layout = app.context_layout().unwrap();
            app.pointer(wheel::variant_position(layout, 5));
            assert_eq!(app.selection(app.hover).as_deref(), Some("😭💔🥀"));
            assert!(app.status.starts_with("Saddest"));
            graphics.draw(&app).unwrap();
            for point in [
                wheel::FAVORITES_CENTER,
                wheel::ADD_CENTER,
                wheel::PIN_CENTER,
            ] {
                assert_eq!(
                    graphics.surface.alpha(point.x as usize, point.y as usize),
                    255
                );
            }
        }
    }
}

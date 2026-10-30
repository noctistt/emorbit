//! Windows proof of concept. All native callbacks run on the message thread.
//! The boxed App outlives its HWND; GWLP_USERDATA is cleared before it is freed.
use emorbit::{
    catalog::{self, CATALOG},
    unicode_units,
    wheel::{self, Hit, Point},
};
use std::{
    cell::RefCell,
    mem::size_of,
    sync::atomic::{AtomicPtr, Ordering},
    time::Instant,
};
use windows::{
    core::{w, Error, Result, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::{Common::*, *},
            DirectWrite::*,
            Gdi::*,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Accessibility::*, Controls::EM_SETLIMITTEXT, HiDpi::*, Input::KeyboardAndMouse::*,
            Shell::*, WindowsAndMessaging::*,
        },
    },
};
use windows_numerics::Vector2;

const HOTKEY: i32 = 1;
const ESCAPE_KEY: i32 = 2;
const TICK: usize = 1;
const DWELL: usize = 2;
const TRAY: u32 = WM_APP + 1;
const FOREGROUND: u32 = WM_APP + 2;
const SHOW: u32 = WM_APP + 3;
const SEARCH_ENTER: u32 = WM_APP + 4;
const SEARCH_ESCAPE: u32 = WM_APP + 5;
const REFRESH_SIZE: u32 = WM_APP + 6;
const EXIT: u32 = WM_APP + 7;
static WINDOW: AtomicPtr<std::ffi::c_void> = AtomicPtr::new(std::ptr::null_mut());

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
    edit: HWND,
    graphics: Option<Graphics>,
    visible: bool,
    pinned: bool,
    mouse_origin: Option<Hit>,
    searching: bool,
    destination: Option<Destination>,
    results: Vec<usize>,
    page: usize,
    hover: Hit,
    context: Option<usize>,
    status: String,
    pending: Option<Pending>,
    dpi: u32,
    tray: NOTIFYICONDATAW,
}

impl App {
    fn new() -> Self {
        Self {
            hwnd: HWND::default(),
            edit: HWND::default(),
            graphics: None,
            visible: false,
            pinned: false,
            mouse_origin: None,
            searching: false,
            destination: None,
            results: catalog::search(""),
            page: 0,
            hover: Hit::None,
            context: None,
            status: "Tap Ctrl + Alt + Space to open".into(),
            pending: None,
            dpi: 96,
            tray: NOTIFYICONDATAW::default(),
        }
    }

    fn items(&self) -> &[usize] {
        catalog::page(&self.results, self.page)
    }
    fn variants(&self) -> &'static [&'static str] {
        self.context.map_or(&[], |i| CATALOG[i].variants)
    }
    fn selection(&self, hit: Hit) -> Option<&'static str> {
        match hit {
            Hit::Item(i) => self.items().get(i).map(|index| CATALOG[*index].text),
            Hit::Variant(i) => self.variants().get(i).copied(),
            _ => None,
        }
    }
    unsafe fn invalidate(&self) {
        let _ = InvalidateRect(Some(self.hwnd), None, false);
    }

    unsafe fn show(&mut self) {
        self.destination = Destination::capture(self.hwnd);
        self.visible = true;
        self.mouse_origin = None;
        self.searching = false;
        self.hover = Hit::None;
        self.context = None;
        self.page = 0;
        self.results = catalog::search("");
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
            cursor.y - (wheel::CENTER.y * scale) as i32,
            width,
            height,
            [
                info.rcWork.left,
                info.rcWork.top,
                info.rcWork.right,
                info.rcWork.bottom,
            ],
        );
        let _ = SetWindowPos(
            self.hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
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

    unsafe fn hide(&mut self) {
        self.visible = false;
        self.mouse_origin = None;
        self.context = None;
        let _ = KillTimer(Some(self.hwnd), DWELL);
        if self.pending.is_none() {
            let _ = KillTimer(Some(self.hwnd), TICK);
        }
        let _ = UnregisterHotKey(Some(self.hwnd), ESCAPE_KEY);
        let _ = ShowWindow(self.edit, SW_HIDE);
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
        self.results = catalog::search("");
        self.page = 0;
        self.context = None;
        self.status = "Type to search · Enter inserts first result".into();
        SetWindowLongPtrW(
            self.hwnd,
            GWL_EXSTYLE,
            (WS_EX_TOPMOST | WS_EX_TOOLWINDOW).0 as isize,
        );
        let _ = SetForegroundWindow(self.hwnd);
        self.position_edit();
        let _ = SetWindowTextW(self.edit, w!(""));
        let _ = ShowWindow(self.edit, SW_SHOW);
        let _ = SetFocus(Some(self.edit));
        self.invalidate();
    }

    unsafe fn position_edit(&self) {
        let s = self.dpi as f32 / 96.0;
        let _ = SetWindowPos(
            self.edit,
            None,
            (173.0 * s) as i32,
            (269.0 * s) as i32,
            (94.0 * s) as i32,
            (30.0 * s) as i32,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }

    unsafe fn end_search(&mut self) {
        self.searching = false;
        let _ = ShowWindow(self.edit, SW_HIDE);
        SetWindowLongPtrW(
            self.hwnd,
            GWL_EXSTYLE,
            (WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE).0 as isize,
        );
        if GetForegroundWindow() == self.hwnd {
            if let Some(target) = self.destination {
                if IsWindow(Some(target.window)).as_bool() {
                    let _ = SetForegroundWindow(target.window);
                }
            }
        }
    }

    unsafe fn changed_search(&mut self) {
        let mut text = [0u16; 256];
        let count = GetWindowTextW(self.edit, &mut text);
        self.results = catalog::search(&String::from_utf16_lossy(&text[..count.max(0) as usize]));
        self.page = 0;
        self.context = None;
        self.hover = Hit::None;
        self.invalidate();
    }

    unsafe fn choose(&mut self, text: &str) {
        if self.pending.is_some() {
            return;
        }
        self.mouse_origin = None;
        let _ = KillTimer(Some(self.hwnd), DWELL);
        if !self.searching {
            self.destination = Destination::capture(self.hwnd);
        } else if GetForegroundWindow() != self.hwnd {
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
        let _ = KillTimer(Some(self.hwnd), DWELL);
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
        } else if !self.pinned && !self.searching {
            self.hide();
        } else {
            self.status = "Cancelled · hold mouse and move to choose".into();
            self.invalidate();
        }
    }

    unsafe fn pointer(&mut self, point: Point) {
        let hit = wheel::hit_test(point, self.items().len(), self.variants().len());
        if hit == self.hover {
            return;
        }
        self.hover = hit;
        let _ = KillTimer(Some(self.hwnd), DWELL);
        if let Hit::Item(index) = hit {
            if self
                .items()
                .get(index)
                .is_some_and(|i| !CATALOG[*i].variants.is_empty())
            {
                SetTimer(Some(self.hwnd), DWELL, 350, None);
            } else {
                self.context = None;
            }
        }
        if let Some(text) = self.selection(hit) {
            self.status = format!("Select {text}");
        }
        self.invalidate();
    }

    unsafe fn click(&mut self) {
        if let Some(text) = self.selection(self.hover) {
            self.choose(text);
            return;
        }
        match self.hover {
            Hit::Search => self.begin_search(),
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
        self.context = None;
        self.hover = Hit::None;
        let _ = KillTimer(Some(self.hwnd), DWELL);
        self.invalidate();
    }

    unsafe fn paint(&mut self) -> Result<()> {
        if self.graphics.is_none() {
            self.graphics = Some(Graphics::new(self.hwnd, self.dpi)?);
        }
        let graphics = self.graphics.as_ref().unwrap();
        graphics.draw(self)?;
        Ok(())
    }
}

struct Graphics {
    target: ID2D1HwndRenderTarget,
    text: IDWriteTextFormat,
    small: IDWriteTextFormat,
    emoji: IDWriteTextFormat,
    brush: ID2D1SolidColorBrush,
}

fn color(rgb: u32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: ((rgb >> 16) & 255) as f32 / 255.0,
        g: ((rgb >> 8) & 255) as f32 / 255.0,
        b: (rgb & 255) as f32 / 255.0,
        a: 1.0,
    }
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
        let target = factory.CreateHwndRenderTarget(
            &D2D1_RENDER_TARGET_PROPERTIES {
                dpiX: dpi as f32,
                dpiY: dpi as f32,
                ..Default::default()
            },
            &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd,
                pixelSize: D2D_SIZE_U {
                    width: bounds.right.max(1) as u32,
                    height: bounds.bottom.max(1) as u32,
                },
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            },
        )?;
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
            emoji: format(w!("Segoe UI Emoji"), 27.0)?,
        })
    }

    unsafe fn label(&self, text: &str, bounds: D2D_RECT_F, format: &IDWriteTextFormat, rgb: u32) {
        self.brush.SetColor(&color(rgb));
        self.target.DrawText(
            &unicode_units(text),
            format,
            &bounds,
            &self.brush,
            D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT | D2D1_DRAW_TEXT_OPTIONS_CLIP,
            DWRITE_MEASURING_MODE_NATURAL,
        );
    }

    unsafe fn circle(&self, point: Point, radius: f32, rgb: u32) {
        self.brush.SetColor(&color(rgb));
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
        self.target.Clear(Some(&color(0x13151f)));
        self.label(
            "EmOrbit",
            rect(20.0, 12.0, 130.0, 36.0),
            &self.text,
            0xf3f2ff,
        );
        self.label(
            if app.pinned { "Unpin" } else { "Pin" },
            rect(340.0, 12.0, 40.0, 36.0),
            &self.small,
            0xbca7ff,
        );
        self.label("×", rect(388.0, 12.0, 40.0, 36.0), &self.text, 0xaaaabb);
        self.label(
            "NATIVE PROTOTYPE",
            rect(140.0, 48.0, 160.0, 24.0),
            &self.small,
            0x83869d,
        );
        // A subtle orbit joins the individual emoji targets.
        self.brush.SetColor(&color(0x303345));
        self.target.DrawEllipse(
            &D2D1_ELLIPSE {
                point: Vector2 {
                    X: wheel::CENTER.x,
                    Y: wheel::CENTER.y,
                },
                radiusX: 112.0,
                radiusY: 112.0,
            },
            &self.brush,
            1.5,
            None,
        );
        for (slot, index) in app.items().iter().enumerate() {
            let p = wheel::item_position(slot);
            let selected = app.hover == Hit::Item(slot);
            self.circle(p, 34.0, if selected { 0x8060d8 } else { 0x292c3e });
            self.label(
                CATALOG[*index].text,
                rect(p.x - 32.0, p.y - 32.0, 64.0, 64.0),
                &self.emoji,
                0xffffff,
            );
        }
        let variants = app.variants();
        for (slot, value) in variants.iter().enumerate() {
            let p = wheel::variant_position(slot, variants.len());
            self.circle(
                p,
                29.0,
                if app.hover == Hit::Variant(slot) {
                    0x8060d8
                } else {
                    0x353049
                },
            );
            self.label(
                value,
                rect(p.x - 29.0, p.y - 26.0, 58.0, 52.0),
                if value.chars().count() > 2 {
                    &self.text
                } else {
                    &self.emoji
                },
                0xffffff,
            );
        }
        self.circle(
            wheel::CENTER,
            47.0,
            if app.hover == Hit::Search {
                0x66519d
            } else {
                0x373049
            },
        );
        if !app.searching {
            self.label(
                "Search",
                rect(173.0, 267.0, 94.0, 36.0),
                &self.text,
                0xe4d9ff,
            );
        }
        if app.results.is_empty() {
            self.label(
                "No matches in the prototype catalog",
                rect(50.0, 350.0, 340.0, 28.0),
                &self.small,
                0xb0aec0,
            );
        }
        self.label("‹", rect(100.0, 440.0, 60.0, 38.0), &self.text, 0xc5b8ed);
        self.label(
            &format!("{} / {}", app.page + 1, catalog::page_count(&app.results)),
            rect(160.0, 440.0, 120.0, 38.0),
            &self.small,
            0xa9a6ba,
        );
        self.label("›", rect(280.0, 440.0, 60.0, 38.0), &self.text, 0xc5b8ed);
        self.label(
            &app.status,
            rect(12.0, 484.0, 416.0, 24.0),
            &self.small,
            0xb7afcd,
        );
        self.target.EndDraw(None, None)
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
        if msg == WM_SIZE || msg == WM_DPICHANGED {
            let _ = PostMessageW(Some(hwnd), REFRESH_SIZE, WPARAM(0), LPARAM(0));
            return LRESULT(0);
        }
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    let app = &mut *state;
    match msg {
        WM_MOUSEACTIVATE if !app.searching => return LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => return LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut paint);
            if let Err(error) = app.paint() {
                app.graphics = None;
                app.status = format!("Rendering failed: {error}");
                // No recursive invalidation loop on device loss; the next user
                // interaction recreates resources and requests another paint.
            }
            let _ = EndPaint(hwnd, &paint);
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
                app.results = catalog::search("");
                app.page = 0;
                app.invalidate();
            } else {
                app.pending = None;
                app.hide();
            }
        }
        WM_MOUSEMOVE => {
            let scale = app.dpi as f32 / 96.0;
            app.pointer(Point {
                x: (lp.0 as u16 as i16) as f32 / scale,
                y: ((lp.0 >> 16) as u16 as i16) as f32 / scale,
            });
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
        }
        WM_LBUTTONDOWN => app.begin_mouse(),
        WM_LBUTTONUP => app.finish_mouse(),
        WM_MOUSEWHEEL => app.change_page(if ((wp.0 >> 16) as u16 as i16) > 0 {
            -1
        } else {
            1
        }),
        WM_TIMER if wp.0 == TICK => app.tick(),
        WM_TIMER if wp.0 == DWELL => {
            let _ = KillTimer(Some(hwnd), DWELL);
            if let Hit::Item(i) = app.hover {
                app.context = app.items().get(i).copied();
                app.invalidate();
            }
        }
        WM_COMMAND if (wp.0 >> 16) as u32 == EN_CHANGE => app.changed_search(),
        SEARCH_ENTER => {
            if let Some(index) = app.items().first() {
                app.choose(CATALOG[*index].text);
            }
        }
        SEARCH_ESCAPE => {
            app.end_search();
            app.results = catalog::search("");
            app.page = 0;
            app.invalidate();
        }
        FOREGROUND if app.visible && !app.searching && app.pending.is_none() => {
            if let Some(target) = Destination::capture(hwnd) {
                if app.mouse_origin.is_some() {
                    app.mouse_origin = None;
                    let _ = KillTimer(Some(hwnd), TICK);
                    let _ = KillTimer(Some(hwnd), DWELL);
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
                    if command == 2 {
                        let _ = PostMessageW(Some(hwnd), EXIT, WPARAM(0), LPARAM(0));
                    } else if command == 1 {
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
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        if let Ok(existing) = FindWindowW(w!("EmOrbit.Prototype"), None) {
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
        let app = Box::new(RefCell::new(App::new()));
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
        WINDOW.store(hwnd.0, Ordering::Relaxed);
        // Install callbacks only after every fallible setup step is complete.
        let setup = (|| -> Result<()> {
            let app = &mut *state;
            app.edit = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("EDIT"),
                w!(""),
                WS_CHILD | WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
                0,
                0,
                94,
                30,
                Some(hwnd),
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

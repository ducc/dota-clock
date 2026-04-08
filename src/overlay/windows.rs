use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

use crate::clock::{ClockState, DisplayFrame, DisplayItem, Urgency};
use crate::config::Config;
use crate::events::FlatEvent;
use crate::gsi::GameState;
use crate::icons;
use crate::patches::RecurringTiming;

const CLASS_NAME: PCSTR = s!("DotaClockOverlay");
const TIMER_ID: usize = 1;
const TIMER_MS: u32 = 200;
// Near-black magenta used as the transparent colorkey — won't appear in normal UI
const COLORKEY: COLORREF = COLORREF(0x00010001);
const WM_TRAYICON: u32 = WM_APP + 1;
const IDM_QUIT: u32 = 1000;

struct DecodedIcon {
    width: i32,
    height: i32,
    // Pre-multiplied BGRA pixels (bottom-up for DIB)
    pixels: Vec<u8>,
}

struct OverlayState {
    shared_state: Arc<Mutex<GameState>>,
    clock_state: ClockState,
    events: Arc<Vec<FlatEvent>>,
    recurring: Vec<RecurringTiming>,
    max_icons: usize,
    icon_size: i32,
    vertical: bool,
    icon_cache: HashMap<&'static str, DecodedIcon>,
}

fn decode_icon(png_data: &[u8], target_size: i32) -> DecodedIcon {
    let img = image::load_from_memory_with_format(png_data, image::ImageFormat::Png)
        .expect("Failed to decode PNG icon")
        .resize_exact(
            target_size as u32,
            target_size as u32,
            image::imageops::FilterType::Lanczos3,
        )
        .to_rgba8();

    let (w, h) = (img.width() as i32, img.height() as i32);

    // Convert RGBA top-down to pre-multiplied BGRA bottom-up (DIB format)
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let src = &img.as_raw()[((y * w + x) * 4) as usize..][..4];
            let dst_y = h - 1 - y; // flip vertically for bottom-up DIB
            let dst = &mut pixels[((dst_y * w + x) * 4) as usize..][..4];
            let a = src[3] as u32;
            // Pre-multiply and convert RGB -> BGR
            dst[0] = ((src[2] as u32 * a) / 255) as u8; // B
            dst[1] = ((src[1] as u32 * a) / 255) as u8; // G
            dst[2] = ((src[0] as u32 * a) / 255) as u8; // R
            dst[3] = src[3]; // A
        }
    }

    DecodedIcon {
        width: w,
        height: h,
        pixels,
    }
}

fn build_icon_cache(icon_size: i32) -> HashMap<&'static str, DecodedIcon> {
    let names = [
        "bounty_rune.png",
        "water_rune.png",
        "power_rune.png",
        "lotus_pool.png",
        "wisdom_shrine.png",
        "outpost.png",
        "night.png",
        "day.png",
        "tormentor.png",
        "neutral_item.png",
        "siege_creep.png",
        "roshan.png",
        "pull.png",
        "stack.png",
    ];
    let mut cache = HashMap::new();
    for name in names {
        cache.insert(name, decode_icon(icons::bytes(name), icon_size));
    }
    cache
}

pub fn run(
    config: Config,
    shared_state: Arc<Mutex<GameState>>,
    events: Vec<FlatEvent>,
    recurring: Vec<RecurringTiming>,
) {
    let icon_cache = build_icon_cache(config.icon_size);

    unsafe {
        let instance = HINSTANCE::default();

        let wc = WNDCLASSA {
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            lpszClassName: CLASS_NAME,
            hbrBackground: HBRUSH::default(),
            ..Default::default()
        };
        RegisterClassA(&wc);

        let item_extent = config.icon_size + 16;
        let item_depth = config.icon_size + 60;
        let total_items = recurring.len() + config.max_icons;
        let (window_width, window_height) = if config.vertical {
            (item_depth, (item_extent * total_items as i32).max(200))
        } else {
            ((item_extent * total_items as i32).max(200), item_depth)
        };

        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);

        let (x, y) = match config.anchor.as_str() {
            "bottom-left" => (
                config.margin_left,
                screen_h - window_height - config.margin_bottom,
            ),
            "top-right" => (
                screen_w - window_width - config.margin_right,
                config.margin_top,
            ),
            "top-left" => (config.margin_left, config.margin_top),
            _ => (
                screen_w - window_width - config.margin_right,
                screen_h - window_height - config.margin_bottom,
            ),
        };

        let hwnd = CreateWindowExA(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            CLASS_NAME,
            s!("Dota Clock"),
            WS_POPUP | WS_VISIBLE,
            x,
            y,
            window_width,
            window_height,
            None,
            None,
            Some(instance),
            None,
        )
        .unwrap();

        SetLayeredWindowAttributes(hwnd, COLORKEY, 0, LWA_COLORKEY).unwrap();

        let state = Box::new(OverlayState {
            shared_state,
            clock_state: ClockState::new(),
            events: Arc::new(events),
            recurring,
            max_icons: config.max_icons,
            icon_size: config.icon_size,
            vertical: config.vertical,
            icon_cache,
        });
        SetWindowLongPtrA(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

        SetTimer(Some(hwnd), TIMER_ID, TIMER_MS, None);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);

        // System tray icon
        let mut nid = NOTIFYICONDATAA {
            cbSize: std::mem::size_of::<NOTIFYICONDATAA>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: LoadIconA(None, PCSTR(32512 as *const u8)).unwrap_or_default(),
            ..Default::default()
        };
        let tip: &[i8] = &[
            b'D' as i8, b'o' as i8, b't' as i8, b'a' as i8, b' ' as i8, b'C' as i8, b'l' as i8,
            b'o' as i8, b'c' as i8, b'k' as i8,
        ];
        nid.szTip[..tip.len()].copy_from_slice(tip);
        let _ = Shell_NotifyIconA(NIM_ADD, &nid);

        let mut msg = MSG::default();
        while GetMessageA(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageA(&msg);
        }

        // Remove tray icon on exit
        let _ = Shell_NotifyIconA(NIM_DELETE, &nid);
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER => {
            let ptr = unsafe { GetWindowLongPtrA(hwnd, GWLP_USERDATA) } as *mut OverlayState;
            if !ptr.is_null() {
                let state = unsafe { &mut *ptr };
                let gs = state.shared_state.lock().unwrap().clone();

                if let Some(frame) =
                    state
                        .clock_state
                        .tick(&gs, &state.events, &state.recurring, state.max_icons)
                {
                    if frame.visible {
                        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
                        unsafe { paint(hwnd, &frame, state) };
                    } else {
                        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
                    }
                }
            }
            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut ps) };
            // Fill with colorkey for transparent background
            let brush = unsafe { CreateSolidBrush(COLORKEY) };
            unsafe { FillRect(hdc, &ps.rcPaint, brush) };
            let _ = unsafe { DeleteObject(brush.into()) };
            let _ = unsafe { EndPaint(hwnd, &ps) };
            LRESULT(0)
        }
        WM_TRAYICON => {
            let event = (lparam.0 & 0xFFFF) as u32;
            if event == WM_RBUTTONUP {
                // Show context menu at cursor
                let mut pt = POINT::default();
                let _ = unsafe { GetCursorPos(&mut pt) };
                let hmenu = unsafe { CreatePopupMenu() }.unwrap();
                unsafe {
                    AppendMenuA(hmenu, MENU_ITEM_FLAGS(0), IDM_QUIT as usize, s!("Quit")).unwrap();
                    // Required for the menu to dismiss properly
                    let _ = SetForegroundWindow(hwnd);
                    let _ = TrackPopupMenu(
                        hmenu,
                        TPM_RIGHTALIGN | TPM_BOTTOMALIGN,
                        pt.x,
                        pt.y,
                        Some(0),
                        hwnd,
                        None,
                    );
                    let _ = DestroyMenu(hmenu);
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd = (wparam.0 & 0xFFFF) as u32;
            if cmd == IDM_QUIT {
                unsafe { DestroyWindow(hwnd) }.unwrap();
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let ptr = unsafe { GetWindowLongPtrA(hwnd, GWLP_USERDATA) } as *mut OverlayState;
            if !ptr.is_null() {
                drop(unsafe { Box::from_raw(ptr) });
            }
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcA(hwnd, msg, wparam, lparam) },
    }
}

unsafe fn paint(hwnd: HWND, frame: &DisplayFrame, state: &OverlayState) {
    let hdc = unsafe { GetDC(Some(hwnd)) };
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect) }.unwrap();

    // Create memory DC for double buffering
    let mem_dc = unsafe { CreateCompatibleDC(Some(hdc)) };
    let mem_bmp =
        unsafe { CreateCompatibleBitmap(hdc, rect.right - rect.left, rect.bottom - rect.top) };
    let old_bmp = unsafe { SelectObject(mem_dc, mem_bmp.into()) };

    // Fill with colorkey (transparent)
    let bg_brush = unsafe { CreateSolidBrush(COLORKEY) };
    unsafe { FillRect(mem_dc, &rect, bg_brush) };
    let _ = unsafe { DeleteObject(bg_brush.into()) };

    let icon_size = state.icon_size;
    let item_step = icon_size + 16;
    let mut x = 4;
    let mut y = 4;

    for slot in &frame.recurring {
        if let Some(item) = slot {
            unsafe { draw_item(mem_dc, item, x, y, icon_size, &state.icon_cache) };
            if state.vertical {
                y += item_step;
            } else {
                x += item_step;
            }
        }
    }

    for item in &frame.events {
        unsafe { draw_item(mem_dc, item, x, y, icon_size, &state.icon_cache) };
        if state.vertical {
            y += item_step;
        } else {
            x += item_step;
        }
    }

    // Blit to window
    let _ = unsafe {
        BitBlt(
            hdc,
            0,
            0,
            rect.right - rect.left,
            rect.bottom - rect.top,
            Some(mem_dc),
            0,
            0,
            SRCCOPY,
        )
    };

    unsafe { SelectObject(mem_dc, old_bmp) };
    let _ = unsafe { DeleteObject(mem_bmp.into()) };
    let _ = unsafe { DeleteDC(mem_dc) };
    unsafe { ReleaseDC(Some(hwnd), hdc) };
}

unsafe fn draw_item(
    hdc: HDC,
    item: &DisplayItem,
    x: i32,
    y: i32,
    icon_size: i32,
    icon_cache: &HashMap<&'static str, DecodedIcon>,
) {
    // Draw background
    let bg_color = match item.urgency {
        Urgency::Urgent => COLORREF(0x001E1EB4),
        Urgency::Warning => COLORREF(0x000A648C),
        _ => COLORREF(0x00140A0A),
    };
    let brush = unsafe { CreateSolidBrush(bg_color) };
    let item_rect = RECT {
        left: x,
        top: y,
        right: x + icon_size + 12,
        bottom: y + icon_size + 40,
    };
    unsafe { FillRect(hdc, &item_rect, brush) };
    let _ = unsafe { DeleteObject(brush.into()) };

    // Draw icon
    if let Some(icon) = icon_cache.get(item.icon_file) {
        unsafe { draw_icon(hdc, icon, x + 6, y + 2, icon_size) };
    }

    // Draw countdown text
    let text_color = match item.urgency {
        Urgency::Urgent => COLORREF(0x004444FF),
        Urgency::Warning => COLORREF(0x0000AAFF),
        Urgency::Soon => COLORREF(0x0044CC88),
        Urgency::Passed => COLORREF(0x00666666),
        Urgency::Dimmed => COLORREF(0x00888888),
    };

    // Create a bold font for countdown
    let font = unsafe {
        CreateFontA(
            14,
            0,
            0,
            0,
            700, // bold
            0,
            0,
            0,
            FONT_CHARSET(0),
            FONT_OUTPUT_PRECISION(0),
            FONT_CLIP_PRECISION(0),
            FONT_QUALITY(0),
            0,
            s!("Courier New"),
        )
    };
    let old_font = unsafe { SelectObject(hdc, font.into()) };

    unsafe {
        SetTextColor(hdc, text_color);
        SetBkMode(hdc, TRANSPARENT);
    }

    let mut text_rect = RECT {
        left: x,
        top: y + icon_size + 2,
        right: x + icon_size + 12,
        bottom: y + icon_size + 20,
    };
    let mut text: Vec<u8> = item.text.bytes().collect();
    unsafe { DrawTextA(hdc, &mut text, &mut text_rect, DT_CENTER | DT_SINGLELINE) };

    // Draw name text (smaller, grey)
    let small_font = unsafe {
        CreateFontA(
            10,
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            FONT_CHARSET(0),
            FONT_OUTPUT_PRECISION(0),
            FONT_CLIP_PRECISION(0),
            FONT_QUALITY(0),
            0,
            s!("Courier New"),
        )
    };
    unsafe { SelectObject(hdc, small_font.into()) };
    unsafe { SetTextColor(hdc, COLORREF(0x00AAAAAA)) };

    let mut name_rect = RECT {
        left: x,
        top: y + icon_size + 18,
        right: x + icon_size + 12,
        bottom: y + icon_size + 36,
    };
    let mut name: Vec<u8> = item.name.bytes().collect();
    unsafe { DrawTextA(hdc, &mut name, &mut name_rect, DT_CENTER | DT_SINGLELINE) };

    // Cleanup
    unsafe { SelectObject(hdc, old_font) };
    let _ = unsafe { DeleteObject(font.into()) };
    let _ = unsafe { DeleteObject(small_font.into()) };
}

unsafe fn draw_icon(hdc: HDC, icon: &DecodedIcon, x: i32, y: i32, size: i32) {
    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: icon.width,
            biHeight: icon.height, // positive = bottom-up
            biPlanes: 1,
            biBitCount: 32,
            biCompression: 0, // BI_RGB
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [RGBQUAD::default()],
    };

    // Create a temporary DC with the icon bitmap
    let icon_dc = unsafe { CreateCompatibleDC(Some(hdc)) };
    let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
    let dib =
        unsafe { CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) }.unwrap();

    // Copy pre-multiplied BGRA pixels into the DIB
    unsafe {
        std::ptr::copy_nonoverlapping(icon.pixels.as_ptr(), bits as *mut u8, icon.pixels.len());
    }

    let old = unsafe { SelectObject(icon_dc, dib.into()) };

    // AlphaBlend the icon onto the target DC
    let blend = BLENDFUNCTION {
        BlendOp: AC_SRC_OVER as u8,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: AC_SRC_ALPHA as u8,
    };
    let _ = unsafe {
        GdiAlphaBlend(
            hdc,
            x,
            y,
            size,
            size,
            icon_dc,
            0,
            0,
            icon.width,
            icon.height,
            blend,
        )
    };

    unsafe { SelectObject(icon_dc, old) };
    let _ = unsafe { DeleteObject(dib.into()) };
    let _ = unsafe { DeleteDC(icon_dc) };
}

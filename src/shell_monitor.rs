extern crate winapi;

use std::{borrow::BorrowMut, ffi::c_void};

use chrono::{DateTime, Local};
use log;
use simple_logger::SimpleLogger;
use winapi::shared::{minwindef, windef};
use winapi::um::winuser;

const CLASS_KAKAO_TALK_AD_VIEW: &str = "BannerAdWnd";
const CLASS_KAKAO_TALK_LOCK_VIEW: &str = "EVA_ChildWindow_Dblclk";
const CLASS_KAKAO_TALK_MAIN_VIEW: &str = "EVA_ChildWindow";
const CLASS_KAKAO_TALK_POPUP: &str = "RichPopWnd";

const TITLE_KAKAO_TALK: &str = "카카오톡";
const TITLE_KAKAO_TALK_EDGE: &str = "KakaoTalkEdgeWnd";
const TITLE_KAKAO_TALK_LOCK_VIEW: &str = "LockModeView_";
const TITLE_KAKAO_TALK_MAIN_VIEW: &str = "OnlineMainView";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RemoveAdLayoutTarget {
    parent_view_hwnd: windef::HWND,
    main_view_hwnd: windef::HWND,
    ad_view_hwnd: windef::HWND,
    lock_view_hwnd: Option<windef::HWND>
}

#[derive(Debug, Clone, /* cannot implement Copy,*/ PartialEq, Eq)]
struct WindowDetail {
    hwnd: windef::HWND,
    parent: Option<windef::HWND>,
    class_name: String,
    title: String
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RectSize {
    width: i32,
    height: i32
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diagnostics {
    pub remove_ad_layout_count: u32,
    pub remove_ad_layout_last: Option<DateTime<Local>>,
    pub remove_ad_popup_count: u32,
    pub remove_ad_popup_last: Option<DateTime<Local>>
}

static mut DIAGNOSTICS: Diagnostics = Diagnostics {
    remove_ad_layout_count: 0,
    remove_ad_layout_last: None,
    remove_ad_popup_count: 0,
    remove_ad_popup_last: None
};

pub fn get_diagnostics() -> Diagnostics {
    return unsafe { DIAGNOSTICS };
}

pub struct Config {
    pub debug: bool
}

static mut CONFIG: Config = Config {
    debug: false
};

pub fn set_config(config: Config) {
    unsafe { CONFIG = config };
} 

fn get_window_rect(hwnd: windef::HWND) -> Option<windef::RECT> {
    let mut rect = windef::RECT{
        left: 0, right: 0, top: 0, bottom: 0
    };
    let ret = unsafe { winuser::GetWindowRect(hwnd, rect.borrow_mut()) };
    return if ret == 0 { None } else { Some (rect) };
}

fn rect_to_size(rect: windef::RECT) -> RectSize {
    RectSize { width: rect.right - rect.left, height: rect.bottom - rect.top }
}

fn get_window_detail(hwnd: windef::HWND) -> WindowDetail {
    let mut buf: [u16; 255] = unsafe { std::mem::zeroed()};
    let title_len = unsafe { winuser::GetWindowTextW(hwnd, buf.as_mut_ptr(), 255)};
    let title = String::from_utf16_lossy(buf.to_vec().get(0..title_len as usize).unwrap());

    let class_len = unsafe {winuser::GetClassNameW(hwnd, buf.as_mut_ptr(), 255)};
    let class_name = String::from_utf16_lossy(buf.to_vec().get(0..class_len as usize).unwrap());

    return WindowDetail{ hwnd: hwnd, parent: None, class_name: class_name, title: title };
}

extern "system" fn winevent_callback(
    _foo: *mut windef::HWINEVENTHOOK__, event: u32, hwnd: *mut windef::HWND__, _baz: i32, _spam: i32, _ham: u32, _quaz: u32) {
    let window_detail = get_window_detail(hwnd);
    let rect = get_window_rect(hwnd);
    if rect.is_none() {
        return;
    }
    let size = rect_to_size(rect.unwrap());

    if unsafe { CONFIG.debug } {
        if event == winuser::EVENT_OBJECT_CREATE {
            log::info!("{:?} {:?}", window_detail, size)
        }
    }
    

    match (event, window_detail.title.as_str(), window_detail.class_name.replace("Sandbox:DefaultBox:", "").as_str()) {
        (winuser::EVENT_OBJECT_CREATE, "", CLASS_KAKAO_TALK_POPUP) => {
            unsafe { winuser::SendMessageW(window_detail.hwnd, winuser::WM_CLOSE, 0, 0) };
            unsafe { DIAGNOSTICS = Diagnostics {
                remove_ad_popup_count: DIAGNOSTICS.remove_ad_popup_count + 1,
                remove_ad_popup_last: Some(Local::now()),
                ..DIAGNOSTICS
            } };
        },
        (_, TITLE_KAKAO_TALK_EDGE, _) => {
            remove_ad_layout();
        },
        (_, _, _) => {
            // do nothing
        }
    }
}

fn enum_windows<F>(mut callback: F) where F: FnMut(windef::HWND) -> bool {
    let mut trait_obj: &mut dyn FnMut(windef::HWND) -> bool = &mut callback;
    let closure_pointer_pointer: *mut c_void = unsafe { std::mem::transmute(&mut trait_obj)};
    unsafe { winuser::EnumWindows(Some(enum_windows_callback), closure_pointer_pointer as isize) };
}

fn enum_child_windows<F>(hwnd: windef::HWND, mut callback: F) where F: FnMut(windef::HWND) -> bool {
    let mut trait_obj: &mut dyn FnMut(windef::HWND) -> bool = &mut callback;
    let closure_pointer_pointer: *mut c_void = unsafe { std::mem::transmute(&mut trait_obj)};
    unsafe { winuser::EnumChildWindows(hwnd, Some(enum_windows_callback), closure_pointer_pointer as isize) };
}

unsafe extern "system" fn enum_windows_callback(hwnd: windef::HWND, lparam: isize) -> i32 {
    let closure: &mut &mut dyn FnMut(windef::HWND) -> bool = std::mem::transmute(lparam as *mut c_void);
    return if closure(hwnd) { 1 } else { 0 };
}

pub fn remove_ad_layout() {
    let mut window_details: Vec<WindowDetail> = vec![];

    enum_windows(|hwnd| {
        let window_detail = get_window_detail(hwnd);

        if window_detail.title == String::from(TITLE_KAKAO_TALK) {
            enum_child_windows(hwnd, |child_hwnd| {
                let child_window_detail = get_window_detail(child_hwnd);
                window_details.push(WindowDetail { parent: Some(hwnd), ..child_window_detail });
                return true;
            });
            window_details.push(window_detail);
            return false;
        }
        return true;
    });

    let main_view = window_details.iter().find(|entry| {
        return entry.class_name.replace("Sandbox:DefaultBox:", "") == String::from(CLASS_KAKAO_TALK_MAIN_VIEW) && entry.title.starts_with(&String::from(TITLE_KAKAO_TALK_MAIN_VIEW));
    });
    if main_view.is_none() {
        return;
    }
    let main_view = main_view.unwrap();
    let ad_view = window_details.iter().find(|entry| {
        return entry.class_name.replace("Sandbox:DefaultBox:", "") == String::from(CLASS_KAKAO_TALK_AD_VIEW) && entry.title == String::from("");
    });
    if ad_view.is_none() {
        return;
    }
    let ad_view = ad_view.unwrap();
    if main_view.parent != ad_view.parent {
        return;
    }

    let lock_view = window_details.iter().find(|entry| {
        return entry.class_name.replace("Sandbox:DefaultBox:", "") == String::from(CLASS_KAKAO_TALK_LOCK_VIEW) && entry.title.starts_with(&String::from(TITLE_KAKAO_TALK_LOCK_VIEW));
    });

    let mut remove_ad_layout_target: Vec<RemoveAdLayoutTarget> = vec![];
    remove_ad_layout_target.push(RemoveAdLayoutTarget {
        main_view_hwnd: main_view.hwnd,  ad_view_hwnd: ad_view.hwnd, parent_view_hwnd: main_view.parent.unwrap(),
        lock_view_hwnd: match lock_view { Some(entry) => Some(entry.hwnd), None => None }
    });

    for entry in remove_ad_layout_target {
        let main_view_rect = get_window_rect(entry.main_view_hwnd);
        if main_view_rect.is_none() {
            continue;
        }
        let main_view_size = rect_to_size(main_view_rect.unwrap());

        let ad_view_rect = get_window_rect(entry.ad_view_hwnd);
        if ad_view_rect.is_none() {
            continue;
        }
        let ad_view_size = rect_to_size(ad_view_rect.unwrap());

        let top_view_rect = get_window_rect(entry.parent_view_hwnd);
        if top_view_rect.is_none() {
            continue;
        }
        let top_view_size = rect_to_size(top_view_rect.unwrap());

        if ad_view_size.height != 0 {
            unsafe { winuser::SetWindowPos(entry.ad_view_hwnd, 0 as windef::HWND, 0, 0, 0, 0, 0)};
        }

        if main_view_size.height != top_view_size.height - 31 {
            unsafe { winuser::SetWindowPos(entry.main_view_hwnd, 0 as windef::HWND, 1, 30,
                main_view_size.width, top_view_size.height - 31, 0)};
            unsafe { DIAGNOSTICS = Diagnostics {
                remove_ad_layout_count: DIAGNOSTICS.remove_ad_layout_count + 1,
                remove_ad_layout_last: Some(Local::now()),
                ..DIAGNOSTICS
            } };
        }

        if entry.lock_view_hwnd.is_some() {
            let lock_view_size = rect_to_size(top_view_rect.unwrap());

            if lock_view_size.height != top_view_size.height - 2 {
                unsafe { winuser::SetWindowPos(entry.lock_view_hwnd.unwrap(), 0 as windef::HWND, 1, 1,
                    lock_view_size.width, top_view_size.height - 2, 0)};
                unsafe { DIAGNOSTICS = Diagnostics {
                    remove_ad_layout_count: DIAGNOSTICS.remove_ad_layout_count + 1,
                    remove_ad_layout_last: Some(Local::now()),
                    ..DIAGNOSTICS
                } };
            }
        }
    }
}

pub fn run() -> *mut windef::HWINEVENTHOOK__   {
    SimpleLogger::new().init().unwrap();

    let interested_events = [
        winuser::EVENT_OBJECT_CREATE, winuser::EVENT_SYSTEM_MOVESIZEEND, winuser::EVENT_OBJECT_LOCATIONCHANGE,
        winuser::EVENT_OBJECT_LIVEREGIONCHANGED, winuser::EVENT_OBJECT_CREATE
    ];
    let hook = unsafe { winuser::SetWinEventHook(
         *interested_events.iter().min().unwrap(),
         *interested_events.iter().max().unwrap(),
         0 as minwindef::HMODULE,
         Some(winevent_callback),
         0,
         0,
         winuser::WINEVENT_OUTOFCONTEXT) };
    return hook;
}

pub fn cleanup(hook: *mut windef::HWINEVENTHOOK__) {
    unsafe { winuser::UnhookWinEvent(hook)};
}


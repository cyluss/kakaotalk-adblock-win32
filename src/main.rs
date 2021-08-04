mod shell_monitor;

extern crate chrono;
extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;

use chrono::{DateTime, Local};
use nwd::NwgUi;
use nwg::NativeUi;
use std::env;

const UNKNOWN_ICON: &[u8] = include_bytes!("../assets/question-mark-4-16.ico");
static VERSION: &'static str = include_str!("../assets/version.txt");

static mut UP: Option<DateTime<Local>> = None;

#[derive(Default, NwgUi)]
pub struct BasicApp {
    #[nwg_control]
    window: nwg::MessageWindow,

    #[nwg_resource(source_bin: Some(UNKNOWN_ICON))]
    proxy_unkn: nwg::Icon,

    #[nwg_control(icon: Some(&data.proxy_unkn), tip: Some("KakaoTalk Adblock for Win32"))]
    #[nwg_events(OnMousePress: [BasicApp::show_menu])]
    tray: nwg::TrayNotification,

    #[nwg_control(parent: window, popup: true)]
    tray_menu: nwg::Menu,

    #[nwg_control(parent: tray_menu, text: "About")]
    #[nwg_events(OnMenuItemSelected: [BasicApp::show_about])]
    tray_item_about: nwg::MenuItem,

    #[nwg_control(parent: tray_menu, text: "Exit")]
    #[nwg_events(OnMenuItemSelected: [BasicApp::exit])]
    tray_item_exit: nwg::MenuItem,
}

fn format_date(date: Option<DateTime<Local>>) -> String {
    match date {
        None => { String::from("None") },
        Some(date) => {
            date.format("%Y-%m-%d %H:%M:%S").to_string()
        }
    }
}

impl BasicApp {
    fn show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn show_about(&self) {
        let diag = shell_monitor::get_diagnostics();
        nwg::simple_message("About", &format!(
            "KakaoTalk Adblock for Win32 {}\n\
             \n\
             remove ad layout #: {} ({})\n\
             remove ad popup #: {} ({})\n\
             start: {}
             ",
             VERSION,
             diag.remove_ad_layout_count, format_date(diag.remove_ad_layout_last),
             diag.remove_ad_popup_count, format_date(diag.remove_ad_popup_last),
             format_date(unsafe { UP })
        ));
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

fn main() {
    unsafe { UP = Some(Local::now()) };

    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1].eq("--debug") {
        shell_monitor::set_config(shell_monitor::Config {debug: true});
    }

    let hook = shell_monitor::run();
    shell_monitor::remove_ad_layout();
    nwg::init().expect("Failed to init Native Windows GUI");

    let _app = BasicApp::build_ui(Default::default()).expect("Failed to build UI");

    nwg::dispatch_thread_events();
    shell_monitor::cleanup(hook);
}
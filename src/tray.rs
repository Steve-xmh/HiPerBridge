//! 托盘图标
//! 目前仅 Windows 可用
use std::sync::mpsc::Sender;

use druid::{commands::CLOSE_ALL_WINDOWS, ExtEventSink, Target};
use once_cell::sync::Lazy;
#[cfg(windows)]
use windows::{
    core::*,
    w,
    Win32::{
        Foundation::*,
        System::LibraryLoader::*,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};

use crate::ui::SHOW_HIPER_WINDOW;

#[derive(Copy, Clone)]
pub enum TrayMessage {
    ShowWindow,
    Exit,
}

#[cfg(windows)]
pub struct TrayIcon {
    hwnd: HWND,
    enable: bool,
    should_exit: bool,
    ctx: Option<ExtEventSink>,
    sx: Option<Sender<TrayMessage>>,
}

#[cfg(windows)]
static mut TRAY: Lazy<TrayIcon> = Lazy::new(TrayIcon::new);

pub fn init_tray() {
    #[cfg(windows)]
    {
        unsafe {
            TRAY.set_icon(false);
        }
    }
}

pub fn uninit_tray() {
    #[cfg(windows)]
    {
        unsafe {
            TRAY.delete();
        }
    }
}

pub fn set_tooltip(tooltip: &str) {
    #[cfg(windows)]
    {
        unsafe {
            TRAY.set_tooltip(tooltip);
        }
    }
}

pub fn set_icon(enable: bool) {
    #[cfg(windows)]
    {
        unsafe {
            TRAY.set_icon(enable);
        }
    }
}

pub fn notify(title: &str, message: &str) {
    #[cfg(windows)]
    {
        unsafe {
            TRAY.notify(title, message);
        }
    }
}

pub fn take_command() -> TrayMessage {
    #[cfg(windows)]
    {
        unsafe { TRAY.take_command() }
    }
    #[cfg(not(windows))]
    {
        TrayMessage::Exit
    }
}

pub fn set_ctx(ctx: ExtEventSink) {
    #[cfg(windows)]
    {
        unsafe { TRAY.set_ctx(ctx) }
    }
}

#[cfg(windows)]
impl TrayIcon {
    const WM_USER_TRAYICON: u32 = WM_USER + 1;
    const WM_USER_PROCESS_COMMANDS: u32 = WM_USER + 2;
    const ICON_GUID: GUID = GUID::from_u128(0xE1214B97484D403EB100B3EC7E4C6ECB);

    pub fn new() -> Self {
        unsafe {
            // 要保留一个托盘图标，需要维持一个窗口及其消息循环
            Self {
                hwnd: Self::tray_thread(),
                enable: false,
                should_exit: false,
                ctx: None,
                sx: None,
            }
        }
    }

    unsafe fn tray_thread() -> HWND {
        let (sx, rx) = oneshot::channel();

        std::thread::spawn(move || {
            let hinstance = GetModuleHandleW(None).unwrap();
            let class_name = w!("hiper_bridge_tray_class");

            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(Self::tray_win_proc),
                lpszClassName: class_name.into(),
                hInstance: hinstance,
                ..Default::default()
            });
            let hwnd = CreateWindowExW(
                WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW,
                class_name,
                w!("HiPer Bridge"),
                WS_OVERLAPPED,
                CW_USEDEFAULT,
                0,
                CW_USEDEFAULT,
                0,
                None,
                None,
                hinstance,
                std::ptr::null_mut(),
            );
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as _,
                guidItem: Self::ICON_GUID,
                hWnd: hwnd,
                uFlags: NIF_ICON | NIF_GUID | NIF_MESSAGE | NIF_INFO | NIF_SHOWTIP,
                uCallbackMessage: Self::WM_USER_TRAYICON,
                hIcon: LoadIconW(hinstance, w!("ICON_GRAY")).unwrap(),
                Anonymous: NOTIFYICONDATAW_0 {
                    uVersion: NOTIFYICON_VERSION_4,
                },
                ..std::mem::zeroed()
            };

            let _ = sx.send(hwnd);

            dbg!(Shell_NotifyIconW(NIM_ADD, &nid).as_bool());
            dbg!(Shell_NotifyIconW(NIM_SETVERSION, &nid).as_bool());

            let mut msg = MSG::default();
            while dbg!(GetMessageW(&mut msg, hwnd, 0, 0).0) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });

        rx.recv().unwrap()
    }

    unsafe extern "system" fn tray_win_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            Self::WM_USER_TRAYICON => {
                match lparam.0 as u32 & 0xFFFF {
                    WM_LBUTTONUP => {
                        // 显示窗口
                        TRAY.ctx
                            .as_ref()
                            .map(|x| x.submit_command(SHOW_HIPER_WINDOW, (), Target::Global));
                        if let Some(sx) = &TRAY.sx {
                            let _ = sx.send(TrayMessage::ShowWindow);
                        }
                    }
                    WM_RBUTTONUP => {
                        let hmenu = CreatePopupMenu().unwrap();
                        let mut pt = std::mem::zeroed();
                        GetCursorPos(&mut pt);

                        AppendMenuW(hmenu, MF_STRING, 1, w!("显示 HiPer Bridge"));
                        AppendMenuW(hmenu, MF_STRING, 2, w!("关闭 HiPer Bridge"));

                        let cmd = TrackPopupMenu(
                            hmenu,
                            TPM_RETURNCMD,
                            pt.x,
                            pt.y,
                            0,
                            hwnd,
                            std::ptr::null_mut(),
                        )
                        .0;

                        PostMessageW(hwnd, WM_NULL, None, None);

                        match dbg!(cmd) {
                            1 => {
                                TRAY.ctx.as_ref().map(|x| {
                                    x.submit_command(SHOW_HIPER_WINDOW, (), Target::Global)
                                });
                                if let Some(sx) = &TRAY.sx {
                                    let _ = sx.send(TrayMessage::ShowWindow);
                                }
                            }
                            2 => {
                                TRAY.should_exit = true;
                                TRAY.ctx.as_ref().map(|x| {
                                    x.submit_command(CLOSE_ALL_WINDOWS, (), Target::Global)
                                });
                                if let Some(sx) = &TRAY.sx {
                                    let _ = sx.send(TrayMessage::Exit);
                                }
                            }
                            _ => {}
                        }
                    }
                    msg => {
                        dbg!(msg);
                    }
                }
                LRESULT(0)
            }
            WM_ACTIVATEAPP => LRESULT(0),
            msg => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    pub fn set_ctx(&mut self, ctx: ExtEventSink) {
        self.ctx = Some(ctx);
    }

    pub fn take_command(&mut self) -> TrayMessage {
        if self.should_exit {
            return TrayMessage::Exit;
        }
        unsafe {
            if !IsWindow(self.hwnd).as_bool() {
                println!("Recrating tray");
                self.hwnd = Self::tray_thread();
                self.set_icon(self.enable);
            }
        }
        let (sx, rx) = std::sync::mpsc::channel();
        self.sx = Some(sx);
        println!("Waiting for tray");
        rx.recv().unwrap()
    }

    pub fn set_tooltip(&self, tooltip: &str) {
        unsafe {
            let mut tooltip: Vec<u16> = tooltip.encode_utf16().collect();
            tooltip.resize(128, 0);
            tooltip.pop();
            tooltip.push(0);
            let tooltip: [u16; 128] = tooltip.try_into().unwrap();
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as _,
                guidItem: Self::ICON_GUID,
                uFlags: NIF_GUID | NIF_TIP,
                szTip: tooltip,
                Anonymous: NOTIFYICONDATAW_0 {
                    uVersion: NOTIFYICON_VERSION_4,
                },
                ..std::mem::zeroed()
            };
            dbg!(Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool());
        }
    }

    pub fn notify(&self, title: &str, message: &str) {
        if self.should_exit {
            return;
        }
        unsafe {
            let mut title: Vec<u16> = title.encode_utf16().collect();
            title.resize(64, 0);
            title.pop();
            title.push(0);
            let mut message: Vec<u16> = message.encode_utf16().collect();
            message.resize(256, 0);
            message.pop();
            message.push(0);
            let title: [u16; 64] = title.try_into().unwrap();
            let message: [u16; 256] = message.try_into().unwrap();
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as _,
                guidItem: Self::ICON_GUID,
                uFlags: NIF_GUID | NIF_INFO | NIF_REALTIME,
                szInfoTitle: title,
                szInfo: message,
                Anonymous: NOTIFYICONDATAW_0 {
                    uVersion: NOTIFYICON_VERSION_4,
                },
                ..std::mem::zeroed()
            };
            dbg!(Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool());
        }
    }

    pub fn set_icon(&mut self, enable: bool) {
        self.enable = enable;
        unsafe {
            let hinstance = GetModuleHandleW(None).unwrap();
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as _,
                guidItem: Self::ICON_GUID,
                uFlags: NIF_ICON | NIF_GUID | NIF_MESSAGE,
                uCallbackMessage: Self::WM_USER_TRAYICON,
                Anonymous: NOTIFYICONDATAW_0 {
                    uVersion: NOTIFYICON_VERSION_4,
                },
                hIcon: if enable {
                    LoadIconW(hinstance, w!("ICON")).unwrap()
                } else {
                    LoadIconW(hinstance, w!("ICON_GRAY")).unwrap()
                },
                ..std::mem::zeroed()
            };
            dbg!(Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool());
        }
    }

    pub fn delete(&self) {
        unsafe {
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as _,
                uFlags: NIF_GUID,
                guidItem: Self::ICON_GUID,
                ..Default::default()
            };
            dbg!(Shell_NotifyIconW(NIM_DELETE, &nid).as_bool());
            DestroyWindow(self.hwnd);
        }
    }
}

#[cfg(windows)]
impl Drop for TrayIcon {
    fn drop(&mut self) {
        self.delete();
    }
}

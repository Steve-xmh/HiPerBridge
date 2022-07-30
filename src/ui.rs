use druid::{
    widget::{Flex, TextBox},
    *,
};

use scl_gui_widgets::{
    theme::icons::{IconColorKey, IconKeyPair, IconPathKey},
    widget_ext::WidgetExt as _,
    widgets::*,
};

use crate::{
    app_state::AppState,
    hiper::{run_hiper_in_thread, stop_hiper},
};

pub const CLIPBOARD_TEXT_ICON: IconKeyPair = (
    CLIPBOARD_TEXT_PATH,
    CLIPBOARD_TEXT_COLOR,
    CLIPBOARD_TEXT_COLOR,
);
pub const CLIPBOARD_TEXT_COLOR: IconColorKey = IconColorKey::new("clipboard-text-color");
pub const CLIPBOARD_TEXT_PATH: IconPathKey = IconPathKey::new("clipboard-text-path");

pub const SET_START_TEXT: Selector<&str> = Selector::new("set-start-text");
pub const SET_IP: Selector<String> = Selector::new("set-ip");
pub const SET_WARNING: Selector<String> = Selector::new("set-warning");
pub const SET_DISABLED: Selector<bool> = Selector::new("set-disabled");
pub const REQUEST_RESTART: Selector = Selector::new("request-restart");

fn main_page() -> Box<dyn Widget<AppState>> {
    Flex::column()
        // .with_child(label::new("HiPer Bridge").with_font(typography::SUBHEADER))
        .with_child(label::new("轻快若风 x 安如磐石 - 最佳跨区域组网方案"))
        .with_spacer(10.)
        .with_flex_child(
            label::dynamic(|data: &AppState, _| data.warning.to_owned())
                .with_text_color(Color::Rgba32(0x9D5D00FF))
                .scroll()
                .vertical()
                .expand(),
            1.,
        )
        .with_child(
            Flex::row()
                .with_flex_child(
                    label::dynamic(|data: &AppState, _| {
                        if data.ip.is_empty() {
                            "".into()
                        } else {
                            format!("Hiper 正在运行！\n网络地址：{}", data.ip)
                        }
                    })
                    .with_text_color(Color::Rgba32(0x0F7B0FFF))
                    .expand_width(),
                    1.,
                )
                .with_child(
                    IconButton::new(CLIPBOARD_TEXT_ICON)
                        .with_flat(true)
                        .on_click(|_, data: &mut AppState, _| {
                            use clipboard::ClipboardProvider;
                            #[cfg(windows)]
                            {
                                if let Ok(mut cb) =
                                    clipboard::windows_clipboard::WindowsClipboardContext::new()
                                {
                                    let _ = cb.set_contents(data.ip.to_owned());
                                }
                            }
                        }),
                )
                .show_if(|data: &AppState, _| !data.ip.is_empty()),
        )
        .with_spacer(5.)
        .with_child(
            TextBox::new()
                .with_placeholder("凭证密钥")
                .lens(AppState::token)
                .on_change(|_, old_data, data, _| {
                    data.token_modified |= old_data.token != data.token;
                })
                .disabled_if(|data, _| !data.ip.is_empty()),
        )
        .with_spacer(10.)
        .with_child(
            Flex::row()
                .with_flex_child(
                    Button::dynamic(|data: &AppState, _| data.start_button.to_owned())
                        .with_accent(true)
                        .on_click(|ctx, data, _| {
                            let ctx = ctx.get_external_handle();
                            if data.token_modified {
                                data.inner_token = data.token.to_owned();
                                if !data.inner_token.is_empty() {
                                    data.token = "••••••••".into();
                                }
                                data.token_modified = false;
                            }
                            let token = data.inner_token.to_owned();
                            let use_tun = data.use_tun;
                            match data.start_button {
                                "启动" => {
                                    run_hiper_in_thread(ctx, token, use_tun);
                                }
                                "关闭" => {
                                    std::thread::spawn(move || {
                                        let _ =
                                            ctx.submit_command(SET_DISABLED, true, Target::Auto);
                                        stop_hiper(ctx.to_owned());
                                        let _ =
                                            ctx.submit_command(SET_DISABLED, false, Target::Auto);
                                    });
                                }
                                _ => {
                                    println!(
                                        "Warning: Unknown start button text {}",
                                        data.start_button
                                    );
                                }
                            }
                        })
                        .expand_width()
                        .disabled_if(|data: &AppState, _| data.token.trim().is_empty()),
                    1.,
                )
                .with_spacer(10.)
                .with_child(
                    IconButton::new(scl_gui_widgets::theme::icons::SETTINGS).on_click(
                        |ctx, _, _| {
                            ctx.submit_command(ENABLE_BACK_PAGE.with(true));
                            ctx.submit_command(PUSH_PAGE.with("setting"));
                        },
                    ),
                )
                .must_fill_main_axis(true),
        )
        // .must_fill_main_axis(true)
        .cross_axis_alignment(widget::CrossAxisAlignment::Fill)
        .padding((10., 10.))
        .boxed()
}

fn setting_page() -> Box<dyn Widget<AppState>> {
    Flex::column()
        .with_child(label::new("选项"))
        .with_spacer(10.)
        .with_child(label::new("使用 WinTUN 而非 WinTAP"))
        .with_spacer(5.)
        .with_child(
            ToggleSwitch::new()
                .lens(AppState::use_tun)
                .disabled_if(|data: &AppState, _| !data.ip.is_empty()),
        )
        .with_spacer(10.)
        .with_child(label::new("发生错误时自动重启"))
        .with_spacer(5.)
        .with_child(ToggleSwitch::new().lens(AppState::auto_restart))
        .with_spacer(10.)
        .with_child(label::new("关于"))
        .with_spacer(10.)
        .with_child(label::new("HiPer Bridge v0.0.4"))
        .with_child(label::new("轻量级 HiPer Plus 启动器"))
        .with_child(label::new("By SteveXMH"))
        .cross_axis_alignment(widget::CrossAxisAlignment::Fill)
        .padding((10., 10.))
        .scroll()
        .vertical()
        .expand()
        .boxed()
}

pub fn ui_builder() -> impl Widget<AppState> {
    PageSwitcher::new()
        .with_page("main", Box::new(main_page))
        .with_page("setting", Box::new(setting_page))
}

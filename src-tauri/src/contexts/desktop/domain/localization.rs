use super::ApplicationLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeCopy {
    pub(crate) tray_show: &'static str,
    pub(crate) tray_hide: &'static str,
    pub(crate) tray_quit: &'static str,
    pub(crate) close_notice_title: &'static str,
    pub(crate) close_notice: &'static str,
    pub(crate) communications_overload: &'static str,
}

impl NativeCopy {
    pub(crate) fn for_language(language: ApplicationLanguage) -> Self {
        match language {
            ApplicationLanguage::ChineseSimplified => Self {
                tray_show: "显示 VaneHub AI",
                tray_hide: "隐藏 VaneHub AI",
                tray_quit: "退出",
                close_notice_title: "VaneHub AI 仍在运行",
                close_notice: "VaneHub AI 将在后台继续接收 IM 消息。可通过系统托盘恢复窗口或退出。",
                communications_overload: "待处理消息过多，请稍后重试。",
            },
            ApplicationLanguage::English => Self {
                tray_show: "Show VaneHub AI",
                tray_hide: "Hide VaneHub AI",
                tray_quit: "Quit",
                close_notice_title: "VaneHub AI is still running",
                close_notice: "VaneHub AI will keep receiving IM messages in the background. Use the system tray to restore or quit.",
                communications_overload: "Too many pending messages. Please try again later.",
            },
            ApplicationLanguage::ChineseTraditional => Self {
                tray_show: "顯示 VaneHub AI",
                tray_hide: "隱藏 VaneHub AI",
                tray_quit: "結束",
                close_notice_title: "VaneHub AI 仍在執行",
                close_notice: "VaneHub AI 會在背景繼續接收 IM 訊息。可透過系統匣還原視窗或結束應用程式。",
                communications_overload: "待處理訊息過多，請稍後再試。",
            },
            ApplicationLanguage::Japanese => Self {
                tray_show: "VaneHub AI を表示",
                tray_hide: "VaneHub AI を非表示",
                tray_quit: "終了",
                close_notice_title: "VaneHub AI は実行中です",
                close_notice: "VaneHub AI はバックグラウンドで IM メッセージを受信し続けます。システムトレイからウィンドウを表示するか、終了できます。",
                communications_overload: "保留中のメッセージが多すぎます。しばらくしてからもう一度お試しください。",
            },
            ApplicationLanguage::Korean => Self {
                tray_show: "VaneHub AI 표시",
                tray_hide: "VaneHub AI 숨기기",
                tray_quit: "종료",
                close_notice_title: "VaneHub AI가 실행 중입니다",
                close_notice: "VaneHub AI는 백그라운드에서 IM 메시지를 계속 수신합니다. 시스템 트레이에서 창을 복원하거나 종료할 수 있습니다.",
                communications_overload: "대기 중인 메시지가 너무 많습니다. 잠시 후 다시 시도하세요.",
            },
        }
    }

    pub(crate) fn resolve(locale: &str) -> Self {
        Self::for_language(ApplicationLanguage::resolve(locale))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_supported_locale_has_native_copy() {
        for id in ApplicationLanguage::SUPPORTED_IDS {
            let copy = NativeCopy::resolve(id);
            assert!(!copy.tray_show.is_empty(), "{id}");
            assert!(!copy.tray_hide.is_empty(), "{id}");
            assert!(!copy.tray_quit.is_empty(), "{id}");
            assert!(!copy.close_notice_title.is_empty(), "{id}");
            assert!(!copy.close_notice.is_empty(), "{id}");
            assert!(!copy.communications_overload.is_empty(), "{id}");
        }
    }

    #[test]
    fn unknown_locale_uses_simplified_chinese_fallback() {
        assert_eq!(
            NativeCopy::resolve("unknown"),
            NativeCopy::for_language(ApplicationLanguage::ChineseSimplified)
        );
    }
}

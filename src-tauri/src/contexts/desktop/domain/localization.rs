use super::ApplicationLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NativeCopy {
    pub(crate) tray_show: &'static str,
    pub(crate) tray_hide: &'static str,
    pub(crate) tray_quit: &'static str,
    pub(crate) close_notice_title: &'static str,
    pub(crate) close_notice: &'static str,
    pub(crate) communications_overload: &'static str,
    pub(crate) communications_unbound: &'static str,
    pub(crate) communications_paused: &'static str,
    pub(crate) communications_stale: &'static str,
    pub(crate) communications_pairing_invalid: &'static str,
    pub(crate) communications_pairing_established: &'static str,
    pub(crate) communications_completion: &'static str,
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
                communications_unbound: "此聊天尚未连接。请在会话 IM 面板中开始配对，然后在此发送 /bind 配对码。",
                communications_paused: "此 IM 连接已暂停。请在会话 IM 面板中恢复。",
                communications_stale: "此连接已不可用。请从有效会话重新开始配对。",
                communications_pairing_invalid: "配对码无效或已过期。",
                communications_pairing_established: "IM 连接已建立。",
                communications_completion: "会话任务已完成。",
            },
            ApplicationLanguage::English => Self {
                tray_show: "Show VaneHub AI",
                tray_hide: "Hide VaneHub AI",
                tray_quit: "Quit",
                close_notice_title: "VaneHub AI is still running",
                close_notice: "VaneHub AI will keep receiving IM messages in the background. Use the system tray to restore or quit.",
                communications_overload: "Too many pending messages. Please try again later.",
                communications_unbound: "This chat is not connected. Start pairing from the session IM panel, then send /bind CODE here.",
                communications_paused: "This IM connection is paused. Resume it from the session IM panel.",
                communications_stale: "This connection is no longer available. Start a new pairing from an active session.",
                communications_pairing_invalid: "The pairing code is invalid or expired.",
                communications_pairing_established: "IM connection established.",
                communications_completion: "The session task has completed.",
            },
            ApplicationLanguage::ChineseTraditional => Self {
                tray_show: "顯示 VaneHub AI",
                tray_hide: "隱藏 VaneHub AI",
                tray_quit: "結束",
                close_notice_title: "VaneHub AI 仍在執行",
                close_notice: "VaneHub AI 會在背景繼續接收 IM 訊息。可透過系統匣還原視窗或結束應用程式。",
                communications_overload: "待處理訊息過多，請稍後再試。",
                communications_unbound: "此聊天尚未連線。請在工作階段 IM 面板中開始配對，然後在此傳送 /bind 配對碼。",
                communications_paused: "此 IM 連線已暫停。請在工作階段 IM 面板中恢復。",
                communications_stale: "此連線已無法使用。請從有效的工作階段重新開始配對。",
                communications_pairing_invalid: "配對碼無效或已過期。",
                communications_pairing_established: "IM 連線已建立。",
                communications_completion: "工作階段任務已完成。",
            },
            ApplicationLanguage::Japanese => Self {
                tray_show: "VaneHub AI を表示",
                tray_hide: "VaneHub AI を非表示",
                tray_quit: "終了",
                close_notice_title: "VaneHub AI は実行中です",
                close_notice: "VaneHub AI はバックグラウンドで IM メッセージを受信し続けます。システムトレイからウィンドウを表示するか、終了できます。",
                communications_overload: "保留中のメッセージが多すぎます。しばらくしてからもう一度お試しください。",
                communications_unbound: "このチャットは未接続です。セッションの IM パネルでペアリングを開始し、ここで /bind ペアリングコードを送信してください。",
                communications_paused: "この IM 接続は一時停止中です。セッションの IM パネルから再開してください。",
                communications_stale: "この接続は利用できなくなりました。有効なセッションから再度ペアリングしてください。",
                communications_pairing_invalid: "ペアリングコードが無効か、期限切れです。",
                communications_pairing_established: "IM 接続が確立されました。",
                communications_completion: "セッションタスクが完了しました。",
            },
            ApplicationLanguage::Korean => Self {
                tray_show: "VaneHub AI 표시",
                tray_hide: "VaneHub AI 숨기기",
                tray_quit: "종료",
                close_notice_title: "VaneHub AI가 실행 중입니다",
                close_notice: "VaneHub AI는 백그라운드에서 IM 메시지를 계속 수신합니다. 시스템 트레이에서 창을 복원하거나 종료할 수 있습니다.",
                communications_overload: "대기 중인 메시지가 너무 많습니다. 잠시 후 다시 시도하세요.",
                communications_unbound: "이 채팅은 연결되지 않았습니다. 세션 IM 패널에서 페어링을 시작한 다음 여기에 /bind 페어링 코드를 보내세요.",
                communications_paused: "이 IM 연결은 일시 중지되었습니다. 세션 IM 패널에서 다시 시작하세요.",
                communications_stale: "이 연결은 더 이상 사용할 수 없습니다. 활성 세션에서 다시 페어링하세요.",
                communications_pairing_invalid: "페어링 코드가 잘못되었거나 만료되었습니다.",
                communications_pairing_established: "IM 연결이 설정되었습니다.",
                communications_completion: "세션 작업이 완료되었습니다.",
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
            assert!(!copy.communications_unbound.is_empty(), "{id}");
            assert!(!copy.communications_paused.is_empty(), "{id}");
            assert!(!copy.communications_stale.is_empty(), "{id}");
            assert!(!copy.communications_pairing_invalid.is_empty(), "{id}");
            assert!(!copy.communications_pairing_established.is_empty(), "{id}");
            assert!(!copy.communications_completion.is_empty(), "{id}");
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

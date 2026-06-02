use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    ZhCN,
    #[serde(rename = "en-US")]
    EnUS,
    #[serde(rename = "ja-JP")]
    JaJP,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::ZhCN => "zh-CN",
            Language::EnUS => "en-US",
            Language::JaJP => "ja-JP",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::ZhCN => "简体中文",
            Language::EnUS => "English",
            Language::JaJP => "日本語",
        }
    }

    pub fn all() -> &'static [Language] {
        &[Language::ZhCN, Language::EnUS, Language::JaJP]
    }

    /// Detect system language
    pub fn detect_system_language() -> Language {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winnls::GetUserDefaultUILanguage;
            unsafe {
                let lang_id = GetUserDefaultUILanguage();
                // Primary language ID is in the lower 10 bits
                let primary_lang_id = lang_id & 0x3FF;
                // Chinese (Simplified) = 0x04 (LANG_CHINESE with SUBLANG_CHINESE_SIMPLIFIED)
                if primary_lang_id == 0x04 {
                    return Language::ZhCN;
                }
                // Japanese = 0x11 (LANG_JAPANESE)
                if primary_lang_id == 0x11 {
                    return Language::JaJP;
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, check the preferred languages
            if let Ok(output) = std::process::Command::new("defaults")
                .args(&["read", "-g", "AppleLanguages"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("zh-Hans") || output_str.contains("zh-CN") {
                    return Language::ZhCN;
                }
                if output_str.contains("ja") || output_str.contains("ja-JP") {
                    return Language::JaJP;
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Check LANG environment variable
            if let Ok(lang) = std::env::var("LANG") {
                if lang.starts_with("zh_CN") || lang.starts_with("zh-Hans") {
                    return Language::ZhCN;
                }
                if lang.starts_with("ja") || lang.starts_with("ja_JP") {
                    return Language::JaJP;
                }
            }
            // Check LC_ALL
            if let Ok(lc_all) = std::env::var("LC_ALL") {
                if lc_all.starts_with("zh_CN") || lc_all.starts_with("zh-Hans") {
                    return Language::ZhCN;
                }
                if lc_all.starts_with("ja") || lc_all.starts_with("ja_JP") {
                    return Language::JaJP;
                }
            }
        }

        // Default to English
        Language::EnUS
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::ZhCN
    }
}

/// Translation key constants
pub mod keys {
    // App title
    pub const APP_TITLE: &str = "app_title";

    // Common buttons
    pub const BUTTON_NEXT: &str = "button_next";
    pub const BUTTON_BACK: &str = "button_back";
    pub const BUTTON_REFRESH: &str = "button_refresh";
    pub const BUTTON_CANCEL: &str = "button_cancel";
    pub const BUTTON_CONFIRM: &str = "button_confirm";
    pub const BUTTON_RETRY: &str = "button_retry";
    pub const BUTTON_SELECT_DIR: &str = "button_select_dir";

    // Loading state
    pub const LOADING_TITLE: &str = "loading_title";
    pub const LOADING_MESSAGE: &str = "loading_message";

    // Version selection
    pub const VERSION_SELECTION_TITLE: &str = "version_selection_title";
    pub const AVAILABLE_VERSIONS: &str = "available_versions";
    pub const LATEST_TAG: &str = "latest_tag";
    pub const TEMP_DIR_LABEL: &str = "temp_dir_label";
    pub const TEMP_DIR_HINT: &str = "temp_dir_hint";

    // USB selection
    pub const USB_SELECTION_TITLE: &str = "usb_selection_title";
    pub const USB_WARNING: &str = "usb_warning";
    pub const NO_USB_DETECTED: &str = "no_usb_detected";
    pub const USB_DETECTED_DEVICES: &str = "usb_detected_devices";
    pub const MANUAL_DRIVE_HINT: &str = "manual_drive_hint";
    pub const MANUAL_DRIVE_EXAMPLE: &str = "manual_drive_example";

    // Format warning
    pub const FORMAT_WARNING_TITLE: &str = "format_warning_title";
    pub const FORMAT_WARNING_MESSAGE: &str = "format_warning_message";
    pub const FORMAT_BACKUP_NOTICE: &str = "format_backup_notice";
    pub const FORMAT_CONFIRM_CHECK: &str = "format_confirm_check";

    // Downloading
    pub const DOWNLOADING_TITLE: &str = "downloading_title";
    pub const PREPARING_DOWNLOAD: &str = "preparing_download";
    pub const VERIFYING: &str = "verifying";
    pub const SKIP_VERIFY_TITLE: &str = "skip_verify_title";
    pub const SKIP_VERIFY_MESSAGE: &str = "skip_verify_message";
    pub const SKIP_VERIFY_CONFIRM: &str = "skip_verify_confirm";
    pub const SKIP_CURRENT_VERIFY: &str = "skip_current_verify";
    pub const VERIFY_SKIPPED: &str = "verify_skipped";
    pub const STATUS_PENDING: &str = "status_pending";
    pub const STATUS_DOWNLOADING: &str = "status_downloading";
    pub const STATUS_COMPLETED: &str = "status_completed";
    pub const STATUS_FAILED: &str = "status_failed";
    pub const DOWNLOAD_COMPLETE: &str = "download_complete";
    pub const NEXT_STEP_FORMAT: &str = "next_step_format";
    pub const DOWNLOAD_FAILED: &str = "download_failed";
    pub const ERROR_DETAILS: &str = "error_details";
    pub const TIP_TITLE: &str = "tip_title";
    pub const TIP_403_ERROR: &str = "tip_403_error";
    pub const TIP_403_CAUSE_1: &str = "tip_403_cause_1";
    pub const TIP_403_CAUSE_2: &str = "tip_403_cause_2";
    pub const TIP_403_CAUSE_3: &str = "tip_403_cause_3";
    pub const TIP_403_SUGGESTION: &str = "tip_403_suggestion";

    // Format confirmation
    pub const FORMAT_CONFIRMATION_TITLE: &str = "format_confirmation_title";
    pub const FINAL_WARNING: &str = "final_warning";
    pub const OPERATIONS_LIST: &str = "operations_list";
    pub const OP_CLEAR_PARTITIONS: &str = "op_clear_partitions";
    pub const OP_CREATE_EXFAT: &str = "op_create_exfat";
    pub const OP_SET_LABEL: &str = "op_set_label";
    pub const OP_ASSIGN_LETTER: &str = "op_assign_letter";
    pub const OP_COPY_FILES: &str = "op_copy_files";
    pub const TARGET_DEVICE: &str = "target_device";
    pub const START_FORMAT: &str = "start_format";

    // Formatting
    pub const FORMATTING_TITLE: &str = "formatting_title";
    pub const FORMATTING_MESSAGE: &str = "formatting_message";
    pub const FORMATTING_WARNING: &str = "formatting_warning";
    pub const STATUS_LABEL: &str = "status_label";

    // Copying
    pub const COPYING_TITLE: &str = "copying_title";
    pub const COPYING_MESSAGE: &str = "copying_message";
    pub const CURRENT_FILE: &str = "current_file";

    // Completed
    pub const COMPLETED_TITLE: &str = "completed_title";
    pub const COMPLETED_MESSAGE: &str = "completed_message";
    pub const COMPLETED_SUBMESSAGE: &str = "completed_submessage";
    pub const RETURN_HOME: &str = "return_home";

    // Error
    pub const ERROR_TITLE: &str = "error_title";
    pub const ERROR_PREFIX: &str = "error_prefix";

    // Disk space warning
    pub const DISK_SPACE_WARNING_TITLE: &str = "disk_space_warning_title";
    pub const DISK_SPACE_WARNING_MESSAGE: &str = "disk_space_warning_message";
    pub const REQUIRED_SPACE: &str = "required_space";
    pub const AVAILABLE_SPACE: &str = "available_space";
    pub const SELECT_OTHER_DIR: &str = "select_other_dir";

    // Language settings
    pub const LANGUAGE_SETTINGS: &str = "language_settings";
    pub const LANGUAGE_LABEL: &str = "language_label";
}

/// Get translations map
fn get_translations() -> &'static HashMap<Language, HashMap<&'static str, &'static str>> {
    static TRANSLATIONS: OnceLock<HashMap<Language, HashMap<&'static str, &'static str>>> = OnceLock::new();

    TRANSLATIONS.get_or_init(|| {
        let mut map = HashMap::new();

        // Chinese (Simplified) translations
        let mut zh_cn = HashMap::new();
        zh_cn.insert(keys::APP_TITLE, "ZundaLink Installer");
        zh_cn.insert(keys::BUTTON_NEXT, "下一步 →");
        zh_cn.insert(keys::BUTTON_BACK, "← 返回");
        zh_cn.insert(keys::BUTTON_REFRESH, "🔄 刷新");
        zh_cn.insert(keys::BUTTON_CANCEL, "取消");
        zh_cn.insert(keys::BUTTON_CONFIRM, "确认继续");
        zh_cn.insert(keys::BUTTON_RETRY, "[重试] 重新下载");
        zh_cn.insert(keys::BUTTON_SELECT_DIR, "选择目录");
        zh_cn.insert(keys::LOADING_TITLE, "ZundaLink Installer");
        zh_cn.insert(keys::LOADING_MESSAGE, "正在加载配置...");
        zh_cn.insert(keys::VERSION_SELECTION_TITLE, "选择安装版本");
        zh_cn.insert(keys::AVAILABLE_VERSIONS, "可用版本:");
        zh_cn.insert(keys::LATEST_TAG, "(最新)");
        zh_cn.insert(keys::TEMP_DIR_LABEL, "临时文件目录:");
        zh_cn.insert(keys::TEMP_DIR_HINT, "下载的文件将保存在此目录");
        zh_cn.insert(keys::USB_SELECTION_TITLE, "选择目标U盘");
        zh_cn.insert(keys::USB_WARNING, "[!] 警告：此操作将格式化选中的U盘并清除所有数据！");
        zh_cn.insert(keys::NO_USB_DETECTED, "未检测到U盘设备。请手动输入盘符:");
        zh_cn.insert(keys::USB_DETECTED_DEVICES, "检测到的U盘设备:");
        zh_cn.insert(keys::MANUAL_DRIVE_HINT, "手动输入盘符:");
        zh_cn.insert(keys::MANUAL_DRIVE_EXAMPLE, "例如: Q 或 Q:");
        zh_cn.insert(keys::FORMAT_WARNING_TITLE, "[!] 重要警告");
        zh_cn.insert(keys::FORMAT_WARNING_MESSAGE, "此操作将格式化U盘并删除所有数据！");
        zh_cn.insert(keys::FORMAT_BACKUP_NOTICE, "请确认您已备份重要数据。");
        zh_cn.insert(keys::FORMAT_CONFIRM_CHECK, "我已了解风险并确认继续");
        zh_cn.insert(keys::DOWNLOADING_TITLE, "下载安装文件");
        zh_cn.insert(keys::PREPARING_DOWNLOAD, "准备下载...");
        zh_cn.insert(keys::VERIFYING, "校验中");
        zh_cn.insert(keys::SKIP_VERIFY_TITLE, "确认跳过校验");
        zh_cn.insert(keys::SKIP_VERIFY_MESSAGE, "确定要跳过文件校验吗？");
        zh_cn.insert(keys::SKIP_VERIFY_CONFIRM, "跳过校验可能会导致安装损坏的文件。");
        zh_cn.insert(keys::SKIP_CURRENT_VERIFY, "跳过当前文件校验");
        zh_cn.insert(keys::VERIFY_SKIPPED, "已跳过校验");
        zh_cn.insert(keys::STATUS_PENDING, "等待中");
        zh_cn.insert(keys::STATUS_DOWNLOADING, "下载中");
        zh_cn.insert(keys::STATUS_COMPLETED, "完成");
        zh_cn.insert(keys::STATUS_FAILED, "失败");
        zh_cn.insert(keys::DOWNLOAD_COMPLETE, "[OK] 所有文件下载完成！");
        zh_cn.insert(keys::NEXT_STEP_FORMAT, "下一步：格式化U盘 →");
        zh_cn.insert(keys::DOWNLOAD_FAILED, "[X] 部分文件下载失败");
        zh_cn.insert(keys::ERROR_DETAILS, "失败详情:");
        zh_cn.insert(keys::TIP_TITLE, "[i] 提示:");
        zh_cn.insert(keys::TIP_403_ERROR, "遇到 403 错误通常表示:");
        zh_cn.insert(keys::TIP_403_CAUSE_1, "• 下载链接已过期");
        zh_cn.insert(keys::TIP_403_CAUSE_2, "• 需要更新安装器版本");
        zh_cn.insert(keys::TIP_403_CAUSE_3, "• 服务器限制了访问");
        zh_cn.insert(keys::TIP_403_SUGGESTION, "建议: 请检查是否有新版本安装器，或稍后重试。");
        zh_cn.insert(keys::FORMAT_CONFIRMATION_TITLE, "最终确认");
        zh_cn.insert(keys::FINAL_WARNING, "[!] 最后一次警告！");
        zh_cn.insert(keys::OPERATIONS_LIST, "即将执行以下操作:");
        zh_cn.insert(keys::OP_CLEAR_PARTITIONS, "1. 清除U盘所有分区和数据");
        zh_cn.insert(keys::OP_CREATE_EXFAT, "2. 创建新的exFAT分区");
        zh_cn.insert(keys::OP_SET_LABEL, "3. 设置卷标");
        zh_cn.insert(keys::OP_ASSIGN_LETTER, "4. 分配盘符为 Q:");
        zh_cn.insert(keys::OP_COPY_FILES, "5. 复制安装文件到U盘");
        zh_cn.insert(keys::TARGET_DEVICE, "目标设备:");
        zh_cn.insert(keys::START_FORMAT, "开始格式化并安装");
        zh_cn.insert(keys::FORMATTING_TITLE, "正在格式化U盘...");
        zh_cn.insert(keys::FORMATTING_MESSAGE, "请稍候，正在清除分区并创建新分区...");
        zh_cn.insert(keys::FORMATTING_WARNING, "此过程可能需要几分钟，请勿拔出U盘。");
        zh_cn.insert(keys::STATUS_LABEL, "状态:");
        zh_cn.insert(keys::COPYING_TITLE, "正在复制文件...");
        zh_cn.insert(keys::COPYING_MESSAGE, "正在将安装文件复制到U盘...");
        zh_cn.insert(keys::CURRENT_FILE, "当前文件:");
        zh_cn.insert(keys::COMPLETED_TITLE, "[OK] 安装完成！");
        zh_cn.insert(keys::COMPLETED_MESSAGE, "ZundaLink 安装U盘制作成功！");
        zh_cn.insert(keys::COMPLETED_SUBMESSAGE, "您的U盘现在可以用于安装 ZundaLink 系统。");
        zh_cn.insert(keys::RETURN_HOME, "← 返回主页面");
        zh_cn.insert(keys::ERROR_TITLE, "[X] 错误");
        zh_cn.insert(keys::ERROR_PREFIX, "错误:");
        zh_cn.insert(keys::DISK_SPACE_WARNING_TITLE, "[!] 磁盘空间不足");
        zh_cn.insert(keys::DISK_SPACE_WARNING_MESSAGE, "临时目录所在磁盘空间不足！");
        zh_cn.insert(keys::REQUIRED_SPACE, "所需空间:");
        zh_cn.insert(keys::AVAILABLE_SPACE, "可用空间:");
        zh_cn.insert(keys::SELECT_OTHER_DIR, "选择其他目录");
        zh_cn.insert(keys::LANGUAGE_SETTINGS, "语言设置");
        zh_cn.insert(keys::LANGUAGE_LABEL, "界面语言:");

        // English translations
        let mut en_us = HashMap::new();
        en_us.insert(keys::APP_TITLE, "ZundaLink Installer");
        en_us.insert(keys::BUTTON_NEXT, "Next →");
        en_us.insert(keys::BUTTON_BACK, "← Back");
        en_us.insert(keys::BUTTON_REFRESH, "🔄 Refresh");
        en_us.insert(keys::BUTTON_CANCEL, "Cancel");
        en_us.insert(keys::BUTTON_CONFIRM, "Confirm");
        en_us.insert(keys::BUTTON_RETRY, "[Retry] Redownload");
        en_us.insert(keys::BUTTON_SELECT_DIR, "Select Directory");
        en_us.insert(keys::LOADING_TITLE, "ZundaLink Installer");
        en_us.insert(keys::LOADING_MESSAGE, "Loading configuration...");
        en_us.insert(keys::VERSION_SELECTION_TITLE, "Select Installation Version");
        en_us.insert(keys::AVAILABLE_VERSIONS, "Available Versions:");
        en_us.insert(keys::LATEST_TAG, "(Latest)");
        en_us.insert(keys::TEMP_DIR_LABEL, "Temporary Directory:");
        en_us.insert(keys::TEMP_DIR_HINT, "Downloaded files will be saved to this directory");
        en_us.insert(keys::USB_SELECTION_TITLE, "Select Target USB Drive");
        en_us.insert(keys::USB_WARNING, "[!] Warning: This operation will format the selected USB drive and erase all data!");
        en_us.insert(keys::NO_USB_DETECTED, "No USB device detected. Please enter drive letter manually:");
        en_us.insert(keys::USB_DETECTED_DEVICES, "Detected USB devices:");
        en_us.insert(keys::MANUAL_DRIVE_HINT, "Enter drive letter:");
        en_us.insert(keys::MANUAL_DRIVE_EXAMPLE, "Example: Q or Q:");
        en_us.insert(keys::FORMAT_WARNING_TITLE, "[!] Important Warning");
        en_us.insert(keys::FORMAT_WARNING_MESSAGE, "This operation will format the USB drive and delete all data!");
        en_us.insert(keys::FORMAT_BACKUP_NOTICE, "Please make sure you have backed up important data.");
        en_us.insert(keys::FORMAT_CONFIRM_CHECK, "I understand the risks and confirm to continue");
        en_us.insert(keys::DOWNLOADING_TITLE, "Downloading Installation Files");
        en_us.insert(keys::PREPARING_DOWNLOAD, "Preparing download...");
        en_us.insert(keys::VERIFYING, "Verifying");
        en_us.insert(keys::SKIP_VERIFY_TITLE, "Confirm Skip Verification");
        en_us.insert(keys::SKIP_VERIFY_MESSAGE, "Are you sure you want to skip file verification?");
        en_us.insert(keys::SKIP_VERIFY_CONFIRM, "Skipping verification may result in installing corrupted files.");
        en_us.insert(keys::SKIP_CURRENT_VERIFY, "Skip Current File Verification");
        en_us.insert(keys::VERIFY_SKIPPED, "Verification Skipped");
        en_us.insert(keys::STATUS_PENDING, "Pending");
        en_us.insert(keys::STATUS_DOWNLOADING, "Downloading");
        en_us.insert(keys::STATUS_COMPLETED, "Completed");
        en_us.insert(keys::STATUS_FAILED, "Failed");
        en_us.insert(keys::DOWNLOAD_COMPLETE, "[OK] All files downloaded!");
        en_us.insert(keys::NEXT_STEP_FORMAT, "Next: Format USB →");
        en_us.insert(keys::DOWNLOAD_FAILED, "[X] Some files failed to download");
        en_us.insert(keys::ERROR_DETAILS, "Error Details:");
        en_us.insert(keys::TIP_TITLE, "[i] Tip:");
        en_us.insert(keys::TIP_403_ERROR, "403 errors usually indicate:");
        en_us.insert(keys::TIP_403_CAUSE_1, "• Download link has expired");
        en_us.insert(keys::TIP_403_CAUSE_2, "• Installer update required");
        en_us.insert(keys::TIP_403_CAUSE_3, "• Server access restrictions");
        en_us.insert(keys::TIP_403_SUGGESTION, "Suggestion: Check for new installer version or try again later.");
        en_us.insert(keys::FORMAT_CONFIRMATION_TITLE, "Final Confirmation");
        en_us.insert(keys::FINAL_WARNING, "[!] Final Warning!");
        en_us.insert(keys::OPERATIONS_LIST, "The following operations will be performed:");
        en_us.insert(keys::OP_CLEAR_PARTITIONS, "1. Clear all partitions and data");
        en_us.insert(keys::OP_CREATE_EXFAT, "2. Create new exFAT partition");
        en_us.insert(keys::OP_SET_LABEL, "3. Set volume label");
        en_us.insert(keys::OP_ASSIGN_LETTER, "4. Assign drive letter Q:");
        en_us.insert(keys::OP_COPY_FILES, "5. Copy installation files to USB");
        en_us.insert(keys::TARGET_DEVICE, "Target Device:");
        en_us.insert(keys::START_FORMAT, "Start Format and Install");
        en_us.insert(keys::FORMATTING_TITLE, "Formatting USB Drive...");
        en_us.insert(keys::FORMATTING_MESSAGE, "Please wait, clearing partitions and creating new partition...");
        en_us.insert(keys::FORMATTING_WARNING, "This may take a few minutes. Do not remove the USB drive.");
        en_us.insert(keys::STATUS_LABEL, "Status:");
        en_us.insert(keys::COPYING_TITLE, "Copying Files...");
        en_us.insert(keys::COPYING_MESSAGE, "Copying installation files to USB drive...");
        en_us.insert(keys::CURRENT_FILE, "Current File:");
        en_us.insert(keys::COMPLETED_TITLE, "[OK] Installation Complete!");
        en_us.insert(keys::COMPLETED_MESSAGE, "ZundaLink installation USB created successfully!");
        en_us.insert(keys::COMPLETED_SUBMESSAGE, "Your USB drive is now ready to install ZundaLink.");
        en_us.insert(keys::RETURN_HOME, "← Return to Home");
        en_us.insert(keys::ERROR_TITLE, "[X] Error");
        en_us.insert(keys::ERROR_PREFIX, "Error:");
        en_us.insert(keys::DISK_SPACE_WARNING_TITLE, "[!] Insufficient Disk Space");
        en_us.insert(keys::DISK_SPACE_WARNING_MESSAGE, "Not enough disk space in temporary directory!");
        en_us.insert(keys::REQUIRED_SPACE, "Required:");
        en_us.insert(keys::AVAILABLE_SPACE, "Available:");
        en_us.insert(keys::SELECT_OTHER_DIR, "Select Other Directory");
        en_us.insert(keys::LANGUAGE_SETTINGS, "Language Settings");
        en_us.insert(keys::LANGUAGE_LABEL, "Interface Language:");

        // Japanese translations
        let mut ja_jp = HashMap::new();
        ja_jp.insert(keys::APP_TITLE, "ZundaLink Installer");
        ja_jp.insert(keys::BUTTON_NEXT, "次へ →");
        ja_jp.insert(keys::BUTTON_BACK, "← 戻る");
        ja_jp.insert(keys::BUTTON_REFRESH, "🔄 更新");
        ja_jp.insert(keys::BUTTON_CANCEL, "キャンセル");
        ja_jp.insert(keys::BUTTON_CONFIRM, "確認");
        ja_jp.insert(keys::BUTTON_RETRY, "[再試行] 再ダウンロード");
        ja_jp.insert(keys::BUTTON_SELECT_DIR, "ディレクトリを選択");
        ja_jp.insert(keys::LOADING_TITLE, "ZundaLink Installer");
        ja_jp.insert(keys::LOADING_MESSAGE, "設定を読み込み中...");
        ja_jp.insert(keys::VERSION_SELECTION_TITLE, "インストールバージョンを選択");
        ja_jp.insert(keys::AVAILABLE_VERSIONS, "利用可能なバージョン:");
        ja_jp.insert(keys::LATEST_TAG, "(最新)");
        ja_jp.insert(keys::TEMP_DIR_LABEL, "一時ディレクトリ:");
        ja_jp.insert(keys::TEMP_DIR_HINT, "ダウンロードしたファイルはこのディレクトリに保存されます");
        ja_jp.insert(keys::USB_SELECTION_TITLE, "対象USBドライブを選択");
        ja_jp.insert(keys::USB_WARNING, "[!] 警告: この操作は選択したUSBドライブをフォーマットし、すべてのデータを削除します！");
        ja_jp.insert(keys::NO_USB_DETECTED, "USBデバイスが検出されませんでした。ドライブ文字を手動で入力してください:");
        ja_jp.insert(keys::USB_DETECTED_DEVICES, "検出されたUSBデバイス:");
        ja_jp.insert(keys::MANUAL_DRIVE_HINT, "ドライブ文字を入力:");
        ja_jp.insert(keys::MANUAL_DRIVE_EXAMPLE, "例: Q または Q:");
        ja_jp.insert(keys::FORMAT_WARNING_TITLE, "[!] 重要な警告");
        ja_jp.insert(keys::FORMAT_WARNING_MESSAGE, "この操作はUSBドライブをフォーマットし、すべてのデータを削除します！");
        ja_jp.insert(keys::FORMAT_BACKUP_NOTICE, "重要なデータをバックアップしたことを確認してください。");
        ja_jp.insert(keys::FORMAT_CONFIRM_CHECK, "リスクを理解し、続行することを確認します");
        ja_jp.insert(keys::DOWNLOADING_TITLE, "インストールファイルをダウンロード中");
        ja_jp.insert(keys::PREPARING_DOWNLOAD, "ダウンロードを準備中...");
        ja_jp.insert(keys::VERIFYING, "検証中");
        ja_jp.insert(keys::SKIP_VERIFY_TITLE, "検証のスキップを確認");
        ja_jp.insert(keys::SKIP_VERIFY_MESSAGE, "ファイル検証をスキップしてもよろしいですか？");
        ja_jp.insert(keys::SKIP_VERIFY_CONFIRM, "検証をスキップすると、破損したファイルがインストールされる可能性があります。");
        ja_jp.insert(keys::SKIP_CURRENT_VERIFY, "現在のファイル検証をスキップ");
        ja_jp.insert(keys::VERIFY_SKIPPED, "検証をスキップしました");
        ja_jp.insert(keys::STATUS_PENDING, "待機中");
        ja_jp.insert(keys::STATUS_DOWNLOADING, "ダウンロード中");
        ja_jp.insert(keys::STATUS_COMPLETED, "完了");
        ja_jp.insert(keys::STATUS_FAILED, "失敗");
        ja_jp.insert(keys::DOWNLOAD_COMPLETE, "[OK] すべてのファイルがダウンロードされました！");
        ja_jp.insert(keys::NEXT_STEP_FORMAT, "次へ: USBをフォーマット →");
        ja_jp.insert(keys::DOWNLOAD_FAILED, "[X] 一部のファイルのダウンロードに失敗しました");
        ja_jp.insert(keys::ERROR_DETAILS, "エラーの詳細:");
        ja_jp.insert(keys::TIP_TITLE, "[i] ヒント:");
        ja_jp.insert(keys::TIP_403_ERROR, "403エラーは通常、以下を示します:");
        ja_jp.insert(keys::TIP_403_CAUSE_1, "• ダウンロードリンクが期限切れ");
        ja_jp.insert(keys::TIP_403_CAUSE_2, "• インストーラーの更新が必要");
        ja_jp.insert(keys::TIP_403_CAUSE_3, "• サーバーアクセス制限");
        ja_jp.insert(keys::TIP_403_SUGGESTION, "提案: 新しいバージョンのインストーラーを確認するか、後でもう一度お試しください。");
        ja_jp.insert(keys::FORMAT_CONFIRMATION_TITLE, "最終確認");
        ja_jp.insert(keys::FINAL_WARNING, "[!] 最終警告！");
        ja_jp.insert(keys::OPERATIONS_LIST, "以下の操作が実行されます:");
        ja_jp.insert(keys::OP_CLEAR_PARTITIONS, "1. すべてのパーティションとデータを削除");
        ja_jp.insert(keys::OP_CREATE_EXFAT, "2. 新しいexFATパーティションを作成");
        ja_jp.insert(keys::OP_SET_LABEL, "3. ボリュームラベルを設定");
        ja_jp.insert(keys::OP_ASSIGN_LETTER, "4. ドライブ文字Q:を割り当て");
        ja_jp.insert(keys::OP_COPY_FILES, "5. インストールファイルをUSBにコピー");
        ja_jp.insert(keys::TARGET_DEVICE, "対象デバイス:");
        ja_jp.insert(keys::START_FORMAT, "フォーマットとインストールを開始");
        ja_jp.insert(keys::FORMATTING_TITLE, "USBドライブをフォーマット中...");
        ja_jp.insert(keys::FORMATTING_MESSAGE, "お待ちください。パーティションを削除し、新しいパーティションを作成中...");
        ja_jp.insert(keys::FORMATTING_WARNING, "これには数分かかる場合があります。USBドライブを取り外さないでください。");
        ja_jp.insert(keys::STATUS_LABEL, "状態:");
        ja_jp.insert(keys::COPYING_TITLE, "ファイルをコピー中...");
        ja_jp.insert(keys::COPYING_MESSAGE, "インストールファイルをUSBドライブにコピー中...");
        ja_jp.insert(keys::CURRENT_FILE, "現在のファイル:");
        ja_jp.insert(keys::COMPLETED_TITLE, "[OK] インストール完了！");
        ja_jp.insert(keys::COMPLETED_MESSAGE, "ZundaLinkインストールUSBが正常に作成されました！");
        ja_jp.insert(keys::COMPLETED_SUBMESSAGE, "USBドライブはZundaLinkのインストール準備ができました。");
        ja_jp.insert(keys::RETURN_HOME, "← ホームに戻る");
        ja_jp.insert(keys::ERROR_TITLE, "[X] エラー");
        ja_jp.insert(keys::ERROR_PREFIX, "エラー:");
        ja_jp.insert(keys::DISK_SPACE_WARNING_TITLE, "[!] ディスク容量不足");
        ja_jp.insert(keys::DISK_SPACE_WARNING_MESSAGE, "一時ディレクトリのディスク容量が不足しています！");
        ja_jp.insert(keys::REQUIRED_SPACE, "必要:");
        ja_jp.insert(keys::AVAILABLE_SPACE, "利用可能:");
        ja_jp.insert(keys::SELECT_OTHER_DIR, "別のディレクトリを選択");
        ja_jp.insert(keys::LANGUAGE_SETTINGS, "言語設定");
        ja_jp.insert(keys::LANGUAGE_LABEL, "インターフェース言語:");

        map.insert(Language::ZhCN, zh_cn);
        map.insert(Language::EnUS, en_us);
        map.insert(Language::JaJP, ja_jp);

        map
    })
}

/// Get translation for a key in the specified language
pub fn t(lang: Language, key: &str) -> String {
    let translations = get_translations();
    translations
        .get(&lang)
        .and_then(|lang_map| lang_map.get(key).copied())
        .map(|s| s.to_string())
        .unwrap_or_else(|| key.to_string())
}

/// I18n helper struct for easier usage
#[derive(Debug, Clone)]
pub struct I18n {
    current_language: Language,
}

impl I18n {
    pub fn new(lang: Language) -> Self {
        Self {
            current_language: lang,
        }
    }

    pub fn detect() -> Self {
        Self::new(Language::detect_system_language())
    }

    pub fn current_language(&self) -> Language {
        self.current_language
    }

    pub fn set_language(&mut self, lang: Language) {
        self.current_language = lang;
    }

    pub fn t(&self, key: &str) -> String {
        t(self.current_language, key)
    }
}

impl Default for I18n {
    fn default() -> Self {
        Self::detect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translation_lookup() {
        assert_eq!(t(Language::ZhCN, keys::BUTTON_NEXT), "下一步 →".to_string());
        assert_eq!(t(Language::EnUS, keys::BUTTON_NEXT), "Next →".to_string());
    }

    #[test]
    fn test_i18n_helper() {
        let i18n = I18n::new(Language::ZhCN);
        assert_eq!(i18n.t(keys::BUTTON_BACK), "← 返回".to_string());
    }
}

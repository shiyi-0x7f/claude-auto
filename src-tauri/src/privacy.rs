//! 隐私模式下系统级文本（托盘提示、系统通知）中的账号名打码。
//! 界面里用马赛克组件，不经过这里。

/// 保留前 2 个字符，其余替换为 `***`；邮箱整体按同规则处理（不保留域名）。
pub fn mask(label: &str) -> String {
    let head: String = label.chars().take(2).collect();
    format!("{head}***")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_labels() {
        assert_eq!(mask("alice@example.com"), "al***");
        assert_eq!(mask("工作号"), "工作***");
        assert_eq!(mask("a"), "a***");
    }
}

//! 切换策略（纯函数，无 IO）。
//!
//! - 触发：激活账号 5h 或周窗口已用 ≥ 对应阈值；周耗尽优先报告（恢复更慢）。
//! - 选目标：先排除任一窗口达阈值的账号，再按 5h 已用升序、周已用升序挑选。
//! - 窗口已过 `resets_at` 的数据视为已重置（已用 0），使被切走的账号到点自动恢复可用。

use serde::Serialize;

use crate::anthropic::{Usage, UsageWindow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Exhaustion {
    FiveHour,
    SevenDay,
}

impl Exhaustion {
    pub fn label(self) -> &'static str {
        match self {
            Exhaustion::FiveHour => "5 小时限额",
            Exhaustion::SevenDay => "周限额",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    pub five_hour: f64,
    pub seven_day: f64,
}

pub fn effective_util(w: Option<&UsageWindow>, now: i64) -> f64 {
    match w {
        Some(w) if w.resets_at.is_some_and(|r| r <= now) => 0.0,
        Some(w) => w.utilization,
        None => 0.0,
    }
}

pub fn exhaustion(u: &Usage, th: Thresholds, now: i64) -> Option<Exhaustion> {
    if effective_util(u.seven_day.as_ref(), now) >= th.seven_day {
        Some(Exhaustion::SevenDay)
    } else if effective_util(u.five_hour.as_ref(), now) >= th.five_hour {
        Some(Exhaustion::FiveHour)
    } else {
        None
    }
}

/// 从候选（id, 额度）里挑切换目标；无可用账号返回 None。
/// 额度未知（查询失败）的账号不参与，避免切到失效凭据。
pub fn pick_target<'a>(
    candidates: impl IntoIterator<Item = (&'a str, &'a Usage)>,
    th: Thresholds,
    now: i64,
) -> Option<&'a str> {
    let mut best: Option<(&str, f64, f64)> = None;
    for (id, u) in candidates {
        if exhaustion(u, th, now).is_some() {
            continue;
        }
        let fh = effective_util(u.five_hour.as_ref(), now);
        let sd = effective_util(u.seven_day.as_ref(), now);
        let better = match best {
            None => true,
            Some((_, bfh, bsd)) => (fh, sd) < (bfh, bsd),
        };
        if better {
            best = Some((id, fh, sd));
        }
    }
    best.map(|(id, _, _)| id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TH: Thresholds = Thresholds {
        five_hour: 95.0,
        seven_day: 90.0,
    };

    fn usage(fh: f64, sd: f64) -> Usage {
        Usage {
            five_hour: Some(UsageWindow {
                utilization: fh,
                resets_at: Some(10_000),
            }),
            seven_day: Some(UsageWindow {
                utilization: sd,
                resets_at: Some(20_000),
            }),
        }
    }

    #[test]
    fn detects_each_window_separately() {
        assert_eq!(exhaustion(&usage(50.0, 10.0), TH, 0), None);
        assert_eq!(
            exhaustion(&usage(95.0, 10.0), TH, 0),
            Some(Exhaustion::FiveHour)
        );
        assert_eq!(
            exhaustion(&usage(10.0, 90.0), TH, 0),
            Some(Exhaustion::SevenDay)
        );
        // 两者都耗尽时报告周限额
        assert_eq!(
            exhaustion(&usage(100.0, 100.0), TH, 0),
            Some(Exhaustion::SevenDay)
        );
    }

    #[test]
    fn passed_reset_counts_as_recovered() {
        // now 已过 5h 重置点但未过周重置点
        assert_eq!(exhaustion(&usage(100.0, 10.0), TH, 15_000), None);
        assert_eq!(
            exhaustion(&usage(10.0, 100.0), TH, 15_000),
            Some(Exhaustion::SevenDay)
        );
    }

    #[test]
    fn picks_lowest_five_hour_excluding_weekly_tight() {
        let a = usage(5.0, 92.0); // 周紧张，排除
        let b = usage(40.0, 10.0);
        let c = usage(20.0, 50.0);
        let got = pick_target([("a", &a), ("b", &b), ("c", &c)], TH, 0);
        assert_eq!(got, Some("c"));
    }

    #[test]
    fn tie_breaks_on_weekly() {
        let a = usage(20.0, 60.0);
        let b = usage(20.0, 30.0);
        assert_eq!(pick_target([("a", &a), ("b", &b)], TH, 0), Some("b"));
    }

    #[test]
    fn none_when_all_exhausted() {
        let a = usage(99.0, 10.0);
        let b = usage(10.0, 99.0);
        assert_eq!(pick_target([("a", &a), ("b", &b)], TH, 0), None);
    }
}

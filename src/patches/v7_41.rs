use super::RecurringTiming;
use crate::events::EventDef;

pub struct Patch741;

const MAX: i64 = 10800; // 3 hours

impl super::Patch for Patch741 {
    fn version(&self) -> &'static str {
        "7.41"
    }

    fn events(&self) -> Vec<EventDef> {
        vec![
            EventDef {
                name: "Bounty",
                icon_file: "bounty_rune.png",
                times: (240..=MAX).step_by(240).collect(),
            },
            EventDef {
                name: "Water",
                icon_file: "water_rune.png",
                times: vec![120, 240],
            },
            EventDef {
                name: "Power",
                icon_file: "power_rune.png",
                times: (360..=MAX).step_by(120).collect(),
            },
            EventDef {
                name: "Lotus",
                icon_file: "lotus_pool.png",
                times: (180..=1080).step_by(180).collect(),
            },
            EventDef {
                name: "Wisdom",
                icon_file: "wisdom_shrine.png",
                times: (420..=MAX).step_by(420).collect(),
            },
            EventDef {
                name: "Outpost",
                icon_file: "outpost.png",
                times: {
                    let mut v = vec![600];
                    v.extend((900..=MAX).step_by(300));
                    v
                },
            },
            EventDef {
                name: "Night",
                icon_file: "night.png",
                times: (300..=MAX).step_by(600).collect(),
            },
            EventDef {
                name: "Day",
                icon_file: "day.png",
                times: (600..=MAX).step_by(600).collect(),
            },
            EventDef {
                name: "Tormentor",
                icon_file: "tormentor.png",
                times: vec![900],
            },
            EventDef {
                name: "Tier 2",
                icon_file: "neutral_item.png",
                times: vec![900],
            },
            EventDef {
                name: "Tier 3",
                icon_file: "neutral_item.png",
                times: vec![1500],
            },
            EventDef {
                name: "Tier 4",
                icon_file: "neutral_item.png",
                times: vec![2100],
            },
            EventDef {
                name: "Tier 5",
                icon_file: "neutral_item.png",
                times: vec![3600],
            },
            EventDef {
                name: "Siege+",
                icon_file: "siege_creep.png",
                times: vec![1800, 3600],
            },
        ]
    }

    fn recurring_timings(&self) -> Vec<RecurringTiming> {
        vec![
            RecurringTiming {
                name: "Pull",
                icon_file: "pull.png",
                targets: vec![15, 45],
                warn_window: 15,
                active_window: 3,
            },
            RecurringTiming {
                name: "Stack",
                icon_file: "stack.png",
                targets: vec![53],
                warn_window: 13,
                active_window: 4,
            },
        ]
    }
}

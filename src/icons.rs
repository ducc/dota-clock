macro_rules! embed_icon {
    ($name:ident, $path:literal) => {
        const $name: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/", $path));
    };
}

embed_icon!(ICON_BOUNTY, "bounty_rune.png");
embed_icon!(ICON_WATER, "water_rune.png");
embed_icon!(ICON_POWER, "power_rune.png");
embed_icon!(ICON_LOTUS, "lotus_pool.png");
embed_icon!(ICON_WISDOM, "wisdom_shrine.png");
embed_icon!(ICON_OUTPOST, "outpost.png");
embed_icon!(ICON_NIGHT, "night.png");
embed_icon!(ICON_DAY, "day.png");
embed_icon!(ICON_TORMENTOR, "tormentor.png");
embed_icon!(ICON_NEUTRAL, "neutral_item.png");
embed_icon!(ICON_SIEGE, "siege_creep.png");
embed_icon!(ICON_ROSHAN, "roshan.png");
embed_icon!(ICON_PULL, "pull.png");
embed_icon!(ICON_STACK, "stack.png");

pub fn bytes(name: &str) -> &'static [u8] {
    match name {
        "bounty_rune.png" => ICON_BOUNTY,
        "water_rune.png" => ICON_WATER,
        "power_rune.png" => ICON_POWER,
        "lotus_pool.png" => ICON_LOTUS,
        "wisdom_shrine.png" => ICON_WISDOM,
        "outpost.png" => ICON_OUTPOST,
        "night.png" => ICON_NIGHT,
        "day.png" => ICON_DAY,
        "tormentor.png" => ICON_TORMENTOR,
        "neutral_item.png" => ICON_NEUTRAL,
        "siege_creep.png" => ICON_SIEGE,
        "roshan.png" => ICON_ROSHAN,
        "pull.png" => ICON_PULL,
        "stack.png" => ICON_STACK,
        _ => ICON_BOUNTY,
    }
}

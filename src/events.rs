#[derive(Clone)]
pub struct EventDef {
    pub name: &'static str,
    pub icon_file: &'static str,
    pub times: Vec<i64>,
}

#[derive(Clone)]
pub struct FlatEvent {
    pub time: i64,
    pub name: &'static str,
    pub icon_file: &'static str,
}

pub fn generate(defs: Vec<EventDef>) -> Vec<FlatEvent> {
    let mut out = Vec::new();
    for def in defs {
        for t in &def.times {
            out.push(FlatEvent {
                time: *t,
                name: def.name,
                icon_file: def.icon_file,
            });
        }
    }
    out.sort_by_key(|e| e.time);
    out
}

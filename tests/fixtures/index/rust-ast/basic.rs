use std::fmt::Debug;

pub struct Widget {
    pub id: u64,
}

pub enum Mode {
    Fast,
    Slow,
}

pub fn build_widget(id: u64) -> Widget {
    Widget { id }
}

fn helper_call() -> Mode {
    build_widget(7);
    Mode::Fast
}

// Auto-split footer flash domain shard.
#[derive(Clone, Debug)]
struct FooterFlash {
    line: Line<'static>,
    expires_at: Instant,
}

#[derive(Clone, Debug)]
struct ComposerDraft {
    text: String,
    text_elements: Vec<TextElement>,
    local_image_paths: Vec<PathBuf>,
    remote_image_urls: Vec<String>,
    mention_bindings: Vec<MentionBinding>,
    pending_pastes: Vec<(String, String)>,
    cursor: usize,
}

#[derive(Clone, Debug)]
struct ComposerMentionBinding {
    mention: String,
    path: String,
}

/// Popup state – at most one can be visible at any time.
enum ActivePopup {
    None,
    Command(CommandPopup),
    File(FileSearchPopup),
    Skill(SkillPopup),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlashValidation {
    Immediate,
    Deferred,
}

const FOOTER_SPACING_HEIGHT: u16 = 0;

/// Builds the one-line nudge that replaces the ambient footer without adding layout height.
fn plan_mode_nudge_line() -> Line<'static> {
    Line::from(vec![
        "Create a plan?".magenta(),
        "  ".into(),
        key_hint::shift(KeyCode::Tab).into(),
        " use Plan mode".into(),
        "   ".into(),
        key_hint::plain(KeyCode::Esc).into(),
        " dismiss".into(),
    ])
}

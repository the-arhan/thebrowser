//! Intelligent fallback dark mode stylesheet injection ("Dusk").

use webkit6::{UserContentInjectedFrames, UserContentManager, UserStyleLevel, UserStyleSheet};

/// Restrained dark theme CSS applied to websites lacking native dark styles.
pub const DUSK_FALLBACK_CSS: &str = r#"
/* The Browser — Dusk Fallback Dark Mode */
@media (prefers-color-scheme: light), (prefers-color-scheme: no-preference) {
    html, body {
        background-color: #121214 !important;
        color: #e4e4e7 !important;
        border-color: #27272a !important;
    }

    div, section, article, nav, header, footer, aside, main, table, tr, td, th {
        background-color: inherit;
        border-color: #27272a !important;
        color: inherit;
    }

    input, textarea, select, button {
        background-color: #1c1c20 !important;
        color: #f4f4f5 !important;
        border: 1px solid #3f3f46 !important;
    }

    input:focus, textarea:focus, select:focus {
        border-color: #38bdf8 !important;
        outline: none !important;
    }

    a, a:link, a:visited {
        color: #60a5fa !important;
    }

    a:hover {
        color: #93c5fd !important;
    }

    pre, code, kbd, samp {
        background-color: #18181b !important;
        color: #f4f4f5 !important;
        border-color: #27272a !important;
    }

    /* Strict media preservation: never invert or distort media elements */
    img, video, canvas, svg, picture, [role="img"], iframe {
        filter: none !important;
        background-color: transparent !important;
    }
}
"#;

pub struct Dusk;

impl Dusk {
    /// Creates a pre-compiled WebKit UserStyleSheet for dark fallback.
    pub fn create_stylesheet() -> UserStyleSheet {
        UserStyleSheet::new(
            DUSK_FALLBACK_CSS,
            UserContentInjectedFrames::AllFrames,
            UserStyleLevel::User,
            &[],
            &[],
        )
    }

    /// Attaches the Dusk fallback dark stylesheet to the provided UserContentManager.
    pub fn attach(manager: &UserContentManager) -> UserStyleSheet {
        let sheet = Self::create_stylesheet();
        manager.add_style_sheet(&sheet);
        sheet
    }
}


use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub bg: Color,
    pub sidebar_bg: Color,
    pub selected_bg: Color,
    pub primary: Color,
    pub muted: Color,
    pub accent: Color,
    pub error: Color,
    pub success: Color,
    pub white: Color,
    pub divider: Color,
    pub _status_bg: Color,
    pub highlight_bg: Color,
}

pub const THEMES: &[Theme] = &[
    Theme {
        name: "slack",
        bg: Color::Rgb(26, 29, 33),
        sidebar_bg: Color::Rgb(25, 23, 29),
        selected_bg: Color::Rgb(17, 100, 163),
        primary: Color::Rgb(209, 210, 211),
        muted: Color::Rgb(105, 107, 114),
        accent: Color::Rgb(236, 178, 46),
        error: Color::Rgb(224, 30, 90),
        success: Color::Rgb(43, 172, 118),
        white: Color::Rgb(255, 255, 255),
        divider: Color::Rgb(61, 63, 69),
        _status_bg: Color::Rgb(30, 33, 38),
        highlight_bg: Color::Rgb(37, 39, 41),
    },
    Theme {
        name: "dracula",
        bg: Color::Rgb(40, 42, 54),
        sidebar_bg: Color::Rgb(33, 34, 44),
        selected_bg: Color::Rgb(98, 114, 164),
        primary: Color::Rgb(248, 248, 242),
        muted: Color::Rgb(98, 114, 164),
        accent: Color::Rgb(189, 147, 249),
        error: Color::Rgb(255, 85, 85),
        success: Color::Rgb(80, 250, 123),
        white: Color::Rgb(248, 248, 242),
        divider: Color::Rgb(68, 71, 90),
        _status_bg: Color::Rgb(33, 34, 44),
        highlight_bg: Color::Rgb(68, 71, 90),
    },
    Theme {
        name: "gruvbox",
        bg: Color::Rgb(40, 40, 40),
        sidebar_bg: Color::Rgb(29, 32, 33),
        selected_bg: Color::Rgb(69, 133, 136),
        primary: Color::Rgb(235, 219, 178),
        muted: Color::Rgb(146, 131, 116),
        accent: Color::Rgb(250, 189, 47),
        error: Color::Rgb(251, 73, 52),
        success: Color::Rgb(184, 187, 38),
        white: Color::Rgb(251, 241, 199),
        divider: Color::Rgb(80, 73, 69),
        _status_bg: Color::Rgb(29, 32, 33),
        highlight_bg: Color::Rgb(60, 56, 54),
    },
    Theme {
        name: "catppuccin",
        bg: Color::Rgb(30, 30, 46),
        sidebar_bg: Color::Rgb(24, 24, 37),
        selected_bg: Color::Rgb(137, 180, 250),
        primary: Color::Rgb(205, 214, 244),
        muted: Color::Rgb(108, 112, 134),
        accent: Color::Rgb(249, 226, 175),
        error: Color::Rgb(243, 139, 168),
        success: Color::Rgb(166, 227, 161),
        white: Color::Rgb(205, 214, 244),
        divider: Color::Rgb(69, 71, 90),
        _status_bg: Color::Rgb(24, 24, 37),
        highlight_bg: Color::Rgb(49, 50, 68),
    },
    Theme {
        name: "tokyo-night",
        bg: Color::Rgb(26, 27, 38),
        sidebar_bg: Color::Rgb(22, 22, 30),
        selected_bg: Color::Rgb(122, 162, 247),
        primary: Color::Rgb(169, 177, 214),
        muted: Color::Rgb(86, 95, 137),
        accent: Color::Rgb(224, 175, 104),
        error: Color::Rgb(247, 118, 142),
        success: Color::Rgb(158, 206, 106),
        white: Color::Rgb(192, 202, 245),
        divider: Color::Rgb(59, 66, 97),
        _status_bg: Color::Rgb(22, 22, 30),
        highlight_bg: Color::Rgb(36, 40, 59),
    },
];

pub fn get_theme(name: &str) -> &'static Theme {
    THEMES.iter().find(|t| t.name == name).unwrap_or(&THEMES[0])
}

pub fn theme_names() -> Vec<&'static str> {
    THEMES.iter().map(|t| t.name).collect()
}

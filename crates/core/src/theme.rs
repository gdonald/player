#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub label: &'static str,
}

impl Theme {
    /// Where the client serves the theme's stylesheet.
    pub fn stylesheet(self) -> String {
        format!("/themes/theme-{}.css", self.name)
    }
}

/// Every theme, the default first.
pub const THEMES: [Theme; 4] = [
    Theme {
        name: "default",
        label: "Default",
    },
    Theme {
        name: "charcoal",
        label: "Charcoal",
    },
    Theme {
        name: "steel",
        label: "Steel",
    },
    Theme {
        name: "sapphire",
        label: "Sapphire",
    },
];

/// The theme with the given name, or the default when the name is missing or
/// unknown.
pub fn find(name: Option<&str>) -> Theme {
    THEMES
        .into_iter()
        .find(|theme| Some(theme.name) == name)
        .unwrap_or(THEMES[0])
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn themes_directory() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../client/themes")
    }

    #[test]
    fn a_known_name_finds_its_theme() {
        assert_eq!(find(Some("charcoal")).label, "Charcoal");
    }

    #[test]
    fn a_missing_or_unknown_name_finds_the_default() {
        assert_eq!(find(None).name, "default");
        assert_eq!(find(Some("solarized")).name, "default");
    }

    #[test]
    fn the_stylesheet_follows_the_theme_file_naming() {
        assert_eq!(
            find(Some("charcoal")).stylesheet(),
            "/themes/theme-charcoal.css"
        );
    }

    #[test]
    fn every_theme_has_a_stylesheet_file() {
        for theme in THEMES {
            let file = themes_directory().join(format!("theme-{}.css", theme.name));

            assert!(file.is_file(), "missing {}", file.display());
        }
    }

    #[test]
    fn every_stylesheet_file_is_a_listed_theme() {
        for entry in std::fs::read_dir(themes_directory()).unwrap() {
            let file_name = entry.unwrap().file_name().into_string().unwrap();
            let name = file_name
                .strip_prefix("theme-")
                .and_then(|rest| rest.strip_suffix(".css"));

            assert!(
                name.is_some_and(|name| THEMES.iter().any(|theme| theme.name == name)),
                "{file_name} is not a listed theme"
            );
        }
    }
}

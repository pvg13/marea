//! [`Avatar`] — circular user badge showing initials or a picture.

use dioxus::prelude::*;

use super::merge_class;

/// Diameter preset for an [`Avatar`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AvatarSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl AvatarSize {
    fn class(self) -> &'static str {
        match self {
            AvatarSize::Sm => " avatar--sm",
            AvatarSize::Md => "",
            AvatarSize::Lg => " avatar--lg",
        }
    }
}

/// Uppercased initials of the first two words of `name`.
fn initials(name: &str) -> String {
    name.split_whitespace()
        .take(2)
        .filter_map(|word| word.chars().next())
        .flat_map(char::to_uppercase)
        .collect()
}

/// A `div.avatar`. Renders the image when `src` is given, otherwise the
/// initials derived from `name` ("Pablo Vazquez" → "PV").
#[component]
pub fn Avatar(
    name: String,
    #[props(default)] src: Option<String>,
    #[props(default)] size: AvatarSize,
    #[props(default)] class: Option<String>,
) -> Element {
    let cls = merge_class(format!("avatar{}", size.class()), class.as_ref());
    rsx! {
        div { class: "{cls}",
            if let Some(url) = &src {
                img { src: "{url}", alt: "{name}" }
            } else {
                "{initials(&name)}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::initials;

    #[test]
    fn initials_two_words() {
        assert_eq!(initials("Pablo Vazquez"), "PV");
    }

    #[test]
    fn initials_single_word() {
        assert_eq!(initials("Pablo"), "P");
    }

    #[test]
    fn initials_extra_words_ignored() {
        assert_eq!(initials("ana maría garcía"), "AM");
    }

    #[test]
    fn initials_empty() {
        assert_eq!(initials("   "), "");
    }
}

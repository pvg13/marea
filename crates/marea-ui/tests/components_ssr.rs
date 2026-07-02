//! SSR render tests for the marea-ui component library: each component is
//! rendered through a real `VirtualDom` and the HTML is asserted on the
//! semantic `marea.css` classes it must emit.

use dioxus::prelude::*;
use marea_ui::*;

/// Render a root component to an HTML string.
fn render(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

// ── Button ──────────────────────────────────────────────────────────────

#[test]
fn button_variants_emit_their_class() {
    fn app() -> Element {
        rsx! {
            Button { "default" }
            Button { variant: ButtonVariant::Secondary, "secondary" }
            Button { variant: ButtonVariant::Ghost, "ghost" }
            Button { variant: ButtonVariant::Soft, "soft" }
            Button { variant: ButtonVariant::Danger, "danger" }
            Button { variant: ButtonVariant::Outline, "outline" }
        }
    }
    let html = render(app);
    for cls in [
        "btn-primary",
        "btn-secondary",
        "btn-ghost",
        "btn-soft",
        "btn-danger",
        "btn-outline",
    ] {
        assert!(html.contains(cls), "missing {cls} in {html}");
    }
}

#[test]
fn button_modifiers_and_disabled() {
    fn app() -> Element {
        rsx! {
            Button { small: true, block: true, disabled: true, "go" }
        }
    }
    let html = render(app);
    assert!(html.contains("btn-primary btn-sm btn-block"), "{html}");
    assert!(html.contains("disabled"), "{html}");

    fn enabled() -> Element {
        rsx! {
            Button { "go" }
        }
    }
    let html = render(enabled);
    assert!(!html.contains("disabled"), "{html}");
    assert!(!html.contains("btn-sm"), "{html}");
}

#[test]
fn button_merges_custom_class_and_passes_attributes() {
    fn app() -> Element {
        rsx! {
            Button { class: "my-hook", id: "save-btn", r#type: "submit", "save" }
        }
    }
    let html = render(app);
    assert!(html.contains("btn-primary my-hook"), "{html}");
    assert!(html.contains(r#"id="save-btn""#), "{html}");
    assert!(html.contains(r#"type="submit""#), "{html}");
}

#[test]
fn icon_button_class_and_aria_label() {
    fn app() -> Element {
        rsx! {
            IconButton { aria_label: "Close", "x" }
        }
    }
    let html = render(app);
    assert!(html.contains("btn-ghost btn-icon"), "{html}");
    assert!(html.contains(r#"aria-label="Close""#), "{html}");
}

// ── Input ───────────────────────────────────────────────────────────────

#[test]
fn input_bare_renders_field_input_only() {
    fn app() -> Element {
        rsx! {
            Input { placeholder: "Email" }
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="field-input""#), "{html}");
    assert!(html.contains(r#"placeholder="Email""#), "{html}");
    assert!(!html.contains("field-input--error"), "{html}");
    assert!(!html.contains(r#"class="field""#), "{html}");
}

#[test]
fn input_with_label_and_error_structure() {
    fn app() -> Element {
        rsx! {
            Input {
                label: "Email",
                error: "Required",
                class: "login-email",
            }
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="field""#), "{html}");
    assert!(html.contains(r#"class="field-label""#), "{html}");
    assert!(
        html.contains("field-input field-input--error login-email"),
        "{html}"
    );
    assert!(html.contains(r#"class="field-error""#), "{html}");
    assert!(html.contains("Required"), "{html}");
    // label precedes input, error follows it
    let label_pos = html.find("field-label").unwrap();
    let input_pos = html.find("field-input").unwrap();
    let error_pos = html.find("field-error").unwrap();
    assert!(label_pos < input_pos && input_pos < error_pos, "{html}");
}

// ── Checkbox ────────────────────────────────────────────────────────────

#[test]
fn checkbox_structure() {
    fn app() -> Element {
        rsx! {
            Checkbox { label: "Remember me", checked: true, onchange: move |_| {} }
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="checkbox""#), "{html}");
    assert!(html.contains(r#"type="checkbox""#), "{html}");
    assert!(html.contains("checked"), "{html}");
    assert!(html.contains("Remember me"), "{html}");
}

// ── Card ────────────────────────────────────────────────────────────────

#[test]
fn card_pad_and_modifiers() {
    fn app() -> Element {
        rsx! {
            Card { "default" }
            Card { pad: CardPad::Lg, interactive: true, "lg" }
            Card { pad: CardPad::None, inset: true, class: "extra", "none" }
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="card card-pad""#), "{html}");
    assert!(
        html.contains(r#"class="card card-pad-lg card--interactive""#),
        "{html}"
    );
    assert!(html.contains(r#"class="card card--inset extra""#), "{html}");
}

// ── Pill ────────────────────────────────────────────────────────────────

#[test]
fn pill_tones() {
    fn app() -> Element {
        rsx! {
            Pill { "neutral" }
            Pill { tone: PillTone::Primary, "primary" }
            Pill { tone: PillTone::Accent, "accent" }
            Pill { tone: PillTone::Success, "success" }
            Pill { tone: PillTone::Warning, "warning" }
            Pill { tone: PillTone::Danger, "danger" }
            Pill { tone: PillTone::Ghost, "ghost" }
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="pill""#), "{html}");
    for cls in [
        "pill pill--primary",
        "pill pill--accent",
        "pill pill--success",
        "pill pill--warning",
        "pill pill--danger",
        "pill pill--ghost",
    ] {
        assert!(html.contains(cls), "missing {cls} in {html}");
    }
}

// ── TopBar / ScreenHeader / SectionHeader ───────────────────────────────

#[test]
fn top_bar_title_and_actions() {
    fn app() -> Element {
        rsx! {
            TopBar {
                title: "Marea",
                actions: rsx! {
                    IconButton { aria_label: "Settings", "s" }
                },
            }
        }
    }
    let html = render(app);
    assert!(html.contains("<header"), "{html}");
    assert!(html.contains(r#"class="top-bar""#), "{html}");
    assert!(html.contains("top-bar__title"), "{html}");
    assert!(html.contains("top-bar__actions"), "{html}");

    fn bare() -> Element {
        rsx! {
            TopBar { title: "Marea" }
        }
    }
    let html = render(bare);
    assert!(!html.contains("top-bar__actions"), "{html}");
}

#[test]
fn screen_header_full_structure() {
    fn app() -> Element {
        rsx! {
            ScreenHeader {
                eyebrow: "Hoy",
                title: "Panel",
                subtitle: "Resumen semanal",
                right: rsx! {
                    Pill { "beta" }
                },
            }
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="screen-header""#), "{html}");
    assert!(html.contains("screen-header__body"), "{html}");
    assert!(html.contains(r#"class="eyebrow""#), "{html}");
    assert!(html.contains("<h1"), "{html}");
    assert!(html.contains("screen-header__title"), "{html}");
    assert!(html.contains("screen-header__subtitle"), "{html}");
    assert!(html.contains("pill"), "{html}");
}

#[test]
fn section_header_with_and_without_action() {
    fn app() -> Element {
        rsx! {
            SectionHeader {
                title: "Recientes",
                action_label: "Ver todo",
                on_action: move |_| {},
            }
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="section-header""#), "{html}");
    assert!(html.contains("<h2"), "{html}");
    assert!(html.contains("section-header__title"), "{html}");
    assert!(html.contains("section-header__action"), "{html}");
    assert!(html.contains("Ver todo"), "{html}");

    fn bare() -> Element {
        rsx! {
            SectionHeader { title: "Recientes" }
        }
    }
    let html = render(bare);
    assert!(!html.contains("section-header__action"), "{html}");
}

// ── Row / InfoRow ───────────────────────────────────────────────────────

#[test]
fn row_interactive_only_with_onclick() {
    fn interactive() -> Element {
        rsx! {
            Row { title: "Ajustes", onclick: move |_| {} }
        }
    }
    let html = render(interactive);
    assert!(html.contains("m-row m-row--interactive"), "{html}");

    fn plain() -> Element {
        rsx! {
            Row { title: "Ajustes" }
        }
    }
    let html = render(plain);
    assert!(html.contains(r#"class="m-row""#), "{html}");
    assert!(!html.contains("m-row--interactive"), "{html}");
}

#[test]
fn row_slots() {
    fn app() -> Element {
        rsx! {
            Row {
                title: "Cuenta",
                subtitle: "pablo@example.com",
                icon: rsx! { span { "i" } },
                trailing: rsx! { span { ">" } },
            }
        }
    }
    let html = render(app);
    for cls in [
        "m-row__icon",
        "m-row__body",
        "m-row__title",
        "m-row__sub",
        "m-row__trailing",
    ] {
        assert!(html.contains(cls), "missing {cls} in {html}");
    }

    fn minimal() -> Element {
        rsx! {
            Row { title: "Cuenta" }
        }
    }
    let html = render(minimal);
    assert!(!html.contains("m-row__icon"), "{html}");
    assert!(!html.contains("m-row__sub"), "{html}");
    assert!(!html.contains("m-row__trailing"), "{html}");
}

#[test]
fn info_row_label_and_value() {
    fn app() -> Element {
        rsx! {
            InfoRow { label: "Versión", value: "1.0.0" }
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="info-row""#), "{html}");
    assert!(html.contains("info-row__label"), "{html}");
    assert!(html.contains("info-row__value"), "{html}");
    assert!(html.contains("Versión"), "{html}");
    assert!(html.contains("1.0.0"), "{html}");
}

// ── Avatar ──────────────────────────────────────────────────────────────

#[test]
fn avatar_initials_two_words() {
    fn app() -> Element {
        rsx! {
            Avatar { name: "Pablo Vazquez" }
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="avatar""#), "{html}");
    assert!(html.contains(">PV<"), "{html}");
}

#[test]
fn avatar_initials_single_word() {
    fn app() -> Element {
        rsx! {
            Avatar { name: "pablo" }
        }
    }
    let html = render(app);
    assert!(html.contains(">P<"), "{html}");
}

#[test]
fn avatar_img_branch_and_sizes() {
    fn app() -> Element {
        rsx! {
            Avatar {
                name: "Pablo Vazquez",
                src: "https://example.com/p.png",
                size: AvatarSize::Lg,
            }
            Avatar { name: "Ana", size: AvatarSize::Sm }
        }
    }
    let html = render(app);
    assert!(html.contains("avatar avatar--lg"), "{html}");
    assert!(html.contains("avatar avatar--sm"), "{html}");
    assert!(html.contains("<img"), "{html}");
    assert!(
        html.contains(r#"src="https://example.com/p.png""#),
        "{html}"
    );
    assert!(
        !html.contains(">PV<"),
        "img branch must replace initials: {html}"
    );
}

// ── EmptyState / LoadingSkeleton / GatePending ──────────────────────────

#[test]
fn empty_state_full_structure() {
    fn app() -> Element {
        rsx! {
            EmptyState {
                title: "Sin resultados",
                description: "Prueba con otra búsqueda.",
                icon: rsx! { span { "o" } },
                action: rsx! {
                    Button { "Añadir" }
                },
            }
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="empty-state""#), "{html}");
    assert!(html.contains("empty-state__icon"), "{html}");
    assert!(html.contains("empty-state__title"), "{html}");
    assert!(html.contains("empty-state__desc"), "{html}");
    assert!(html.contains("btn-primary"), "{html}");
}

#[test]
fn loading_skeleton_line_count_and_widths() {
    fn app() -> Element {
        rsx! {
            LoadingSkeleton { lines: 5 }
        }
    }
    let html = render(app);
    assert_eq!(count(&html, "skeleton__line"), 5, "{html}");
    assert!(
        html.contains("width: 60%"),
        "last line must be short: {html}"
    );

    fn default_lines() -> Element {
        rsx! {
            LoadingSkeleton {}
        }
    }
    let html = render(default_lines);
    assert_eq!(count(&html, "skeleton__line"), 3, "{html}");
}

#[test]
fn gate_pending_defaults_to_skeleton() {
    fn app() -> Element {
        rsx! {
            GatePending {}
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="gate-pending""#), "{html}");
    assert!(html.contains("skeleton__line"), "{html}");

    fn custom() -> Element {
        rsx! {
            GatePending {
                span { "Cargando…" }
            }
        }
    }
    let html = render(custom);
    assert!(html.contains("Cargando…"), "{html}");
    assert!(!html.contains("skeleton__line"), "{html}");
}

// ── Eyebrow / Divider ───────────────────────────────────────────────────

#[test]
fn eyebrow_and_divider() {
    fn app() -> Element {
        rsx! {
            Eyebrow { "Nuevo" }
            Divider {}
        }
    }
    let html = render(app);
    assert!(html.contains(r#"class="eyebrow""#), "{html}");
    assert!(html.contains("Nuevo"), "{html}");
    assert!(html.contains("<hr"), "{html}");
    assert!(html.contains(r#"class="divider""#), "{html}");
}

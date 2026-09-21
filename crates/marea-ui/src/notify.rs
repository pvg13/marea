//! Local scheduled reminders (`local-notify` feature).
//!
//! A reminder here is a notification **the device posts to itself**, at a time
//! the app asked for, with nothing on the network. It is the opposite end of
//! the problem from [`crate::push`]: a push token exists so a *server* can
//! reach the device, and everything in this module exists so an app that has
//! no server can still reach the person.
//!
//! That distinction is the reason this is a separate capability rather than a
//! corner of the push one. An app whose data never leaves the household — see
//! marea's own consumers — can use every line of this and still have nothing
//! to send anywhere.
//!
//! # The shape of the API
//!
//! One call, [`replace_all`], which arms exactly the set it is given and
//! forgets everything armed before it. The caller owns the schedule; this
//! module owns the arming. That makes the call idempotent, which is what lets
//! an app recompute its reminders on every launch and after every edit without
//! keeping a diff, and it removes the failure mode where the OS holds an alarm
//! for a chore that was deleted a week ago.
//!
//! ```ignore
//! use marea_ui::notify::{Reminder, replace_all};
//!
//! replace_all(&[Reminder {
//!     id: "chore:abc123:2026-09-22".into(),
//!     at_epoch_ms: 1_758_528_000_000,
//!     title: "Poner una lavadora".into(),
//!     body: "Tiéndela esta mañana y estará seca esta tarde".into(),
//!     deeplink: "roommates://chore/abc123".into(),
//! }])?;
//! ```
//!
//! # What it does on each platform
//!
//! **Android** — `AlarmManager.setAndAllowWhileIdle` per reminder, posted by a
//! broadcast receiver, persisted so a reboot does not lose them. The alarms are
//! deliberately **inexact**: exact ones need `SCHEDULE_EXACT_ALARM` /
//! `USE_EXACT_ALARM`, which Play restricts to alarm clocks and calendars. A
//! reminder that arrives at 09:04 instead of 09:00 is the same reminder; an app
//! rejected from the store is not. The Kotlin half ships with this crate and
//! reaches the app through `#[manganis::ffi]`, so enabling the feature is the
//! whole integration — no file to copy, no manifest entry to add.
//!
//! **Everywhere else** — inert. Every function returns `Ok` and logs, so a
//! screen can call it unconditionally. iOS wants `UNUserNotificationCenter` and
//! is deliberately not faked here: a call that silently does nothing on a
//! platform that *could* have done something is worse than one that is
//! documented not to exist yet.
//!
//! # What it is not
//!
//! Not a scheduler. It arms a list of instants and posts what it was given;
//! everything about *which* instants — recurrence, quiet hours, what the copy
//! says — belongs to the app, where it can be tested without a device.

use serde::{Deserialize, Serialize};

/// One notification to post at one instant.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reminder {
    /// Stable key for this reminder. Arming the same id twice replaces the
    /// first, so an id that encodes *what* and *when* — rather than a fresh
    /// uuid — is what makes [`replace_all`] idempotent across launches.
    pub id: String,
    /// When to post, in milliseconds since the Unix epoch, UTC.
    ///
    /// Epoch milliseconds rather than a `DateTime` on purpose: it is what the
    /// platform layer needs, and it keeps a date-time library out of a
    /// framework crate that would otherwise have to agree with every app's.
    #[serde(rename = "at")]
    pub at_epoch_ms: i64,
    pub title: String,
    pub body: String,
    /// Opaque URL the app routes when the notification is tapped. Empty just
    /// opens the app.
    pub deeplink: String,
}

/// Why a set of reminders could not be armed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotifyError {
    /// The platform bridge could not be reached — no JVM, no Android context,
    /// or the Kotlin half is missing because the feature was enabled without
    /// a rebuild.
    Unavailable(String),
}

impl std::fmt::Display for NotifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotifyError::Unavailable(why) => write!(f, "reminders unavailable: {why}"),
        }
    }
}

impl std::error::Error for NotifyError {}

/// Serialise a set for the platform layer.
///
/// Split out from [`replace_all`] so the payload can be tested on every
/// target, including the ones where arming does nothing. It is also the part
/// that has to be right: a chore's title is a person's own words, and a
/// hand-rolled JSON string would break on the first apostrophe.
pub(crate) fn payload(items: &[Reminder]) -> String {
    serde_json::to_string(items).unwrap_or_else(|_| "[]".to_string())
}

/// Arm exactly these reminders, replacing every one armed before.
///
/// Reminders already in the past are dropped rather than fired: a phone that
/// spent the morning off should not wake up to a burst of notifications about
/// moments that have passed.
pub fn replace_all(items: &[Reminder]) -> Result<(), NotifyError> {
    imp::replace_all(items)
}

/// Forget every reminder this app armed.
pub fn cancel_all() -> Result<(), NotifyError> {
    imp::cancel_all()
}

/// Whether the OS will actually show what this module posts.
///
/// `false` when the person declined the notification permission, or turned the
/// app's notifications off in system settings afterwards. A screen offering
/// reminders should ask this before promising one, and say so plainly rather
/// than arming something that will never appear.
pub fn can_post() -> bool {
    imp::can_post()
}

#[cfg(target_os = "android")]
mod imp {
    use super::{NotifyError, Reminder, payload};
    use jni::objects::{JClass, JObject, JValue};

    const CLASS: &str = "dev.dioxus.main.MareaReminders";

    /// Run `f` with an attached JNI environment, the app's `Context`, and the
    /// `MareaReminders` class.
    ///
    /// The class is resolved through the **Context's own classloader** rather
    /// than with `FindClass`. `FindClass` on a thread Rust created resolves
    /// against the system classloader, which cannot see app classes, and fails
    /// with a `ClassNotFoundException` that surfaces only as "Java exception
    /// was thrown" — the same trap WaveSyncDB's notification helper documents.
    fn with_class<F>(f: F) -> Result<(), NotifyError>
    where
        F: FnOnce(&mut jni::JNIEnv, &JClass, &JObject) -> Result<(), jni::errors::Error>,
    {
        let ctx = ndk_context::android_context();
        if ctx.vm().is_null() || ctx.context().is_null() {
            return Err(NotifyError::Unavailable("no Android context".into()));
        }
        // SAFETY: `ndk_context` hands out the process's real JavaVM and the
        // Activity's Context, both valid for the life of the process.
        let vm = unsafe { jni::JavaVM::from_raw(ctx.vm().cast()) }
            .map_err(|e| NotifyError::Unavailable(e.to_string()))?;
        let mut env = vm
            .attach_current_thread()
            .map_err(|e| NotifyError::Unavailable(e.to_string()))?;
        let context = unsafe { JObject::from_raw(ctx.context().cast()) };

        let resolved = (|| -> Result<JClass, jni::errors::Error> {
            let loader = env
                .call_method(&context, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
                .l()?;
            let name = env.new_string(CLASS)?;
            let class = env
                .call_method(
                    &loader,
                    "loadClass",
                    "(Ljava/lang/String;)Ljava/lang/Class;",
                    &[JValue::Object(&name)],
                )?
                .l()?;
            Ok(JClass::from(class))
        })()
        .map_err(|e| NotifyError::Unavailable(format!("{CLASS}: {e}")))?;

        f(&mut env, &resolved, &context).map_err(|e| NotifyError::Unavailable(e.to_string()))
    }

    pub(super) fn replace_all(items: &[Reminder]) -> Result<(), NotifyError> {
        let json = payload(items);
        with_class(|env, class, context| {
            let json = env.new_string(&json)?;
            env.call_static_method(
                class,
                "replaceAll",
                "(Landroid/content/Context;Ljava/lang/String;)V",
                &[JValue::Object(context), JValue::Object(&json)],
            )?;
            Ok(())
        })
    }

    pub(super) fn cancel_all() -> Result<(), NotifyError> {
        with_class(|env, class, context| {
            env.call_static_method(
                class,
                "cancelAll",
                "(Landroid/content/Context;)V",
                &[JValue::Object(context)],
            )?;
            Ok(())
        })
    }

    pub(super) fn can_post() -> bool {
        let mut allowed = false;
        let _ = with_class(|env, class, context| {
            allowed = env
                .call_static_method(
                    class,
                    "canPost",
                    "(Landroid/content/Context;)Z",
                    &[JValue::Object(context)],
                )?
                .z()?;
            Ok(())
        });
        allowed
    }
}

#[cfg(not(target_os = "android"))]
mod imp {
    use super::{NotifyError, Reminder, payload};

    /// Serialises the set and logs it, which is not busywork: it is the only
    /// way to see what an app *would* arm while developing on desktop, where
    /// the dev loop actually is, and it keeps the payload on every platform's
    /// compile path rather than only Android's.
    pub(super) fn replace_all(items: &[Reminder]) -> Result<(), NotifyError> {
        log::debug!(
            "local-notify: this platform arms nothing; {} reminder(s) would have been: {}",
            items.len(),
            payload(items)
        );
        Ok(())
    }

    pub(super) fn cancel_all() -> Result<(), NotifyError> {
        Ok(())
    }

    /// `false`, and deliberately not `true`: a screen that asks this is
    /// deciding whether to promise someone a notification, and on a platform
    /// that cannot post one the honest answer is no.
    pub(super) fn can_post() -> bool {
        false
    }
}

// Bundles the Kotlin half (MareaReminders, the two receivers, and the manifest
// entries they need) as a Gradle submodule. dx reads it out of the compiled
// binary's symbol table, so an app that enables `local-notify` gets all of it
// without copying a file — the same mechanism WaveSyncDB ships its FCM service
// with.
#[cfg(all(target_os = "android", feature = "local-notify"))]
#[manganis::ffi("src/android")]
unsafe extern "Kotlin" {}

#[cfg(test)]
mod tests {
    use super::*;

    fn reminder(id: &str, at: i64) -> Reminder {
        Reminder {
            id: id.to_string(),
            at_epoch_ms: at,
            title: "Poner una lavadora".to_string(),
            body: "Tiéndela esta mañana".to_string(),
            deeplink: "roommates://chore/abc".to_string(),
        }
    }

    /// The Kotlin half reads `at`, not `at_epoch_ms`. They are one contract
    /// across two languages and nothing but this test holds them together.
    #[test]
    fn the_payload_uses_the_field_names_the_kotlin_half_reads() {
        let json = payload(&[reminder("a", 1_758_528_000_000)]);
        for key in ["\"id\"", "\"at\"", "\"title\"", "\"body\"", "\"deeplink\""] {
            assert!(json.contains(key), "{key} missing from {json}");
        }
        assert!(
            !json.contains("at_epoch_ms"),
            "the Rust field name leaked into the payload: {json}"
        );
    }

    /// A chore's title is a person's own words. Hand-rolling this JSON is the
    /// obvious shortcut and it breaks on the first quotation mark.
    #[test]
    fn a_title_with_quotes_in_it_survives_the_trip() {
        let mut awkward = reminder("b", 1);
        awkward.title = "Fregar los \"platos\" del almuerzo".to_string();
        awkward.body = "Línea 1\nLínea 2".to_string();

        let json = payload(&[awkward.clone()]);
        let back: Vec<Reminder> = serde_json::from_str(&json).expect("valid JSON");

        assert_eq!(back, vec![awkward]);
    }

    #[test]
    fn an_empty_set_is_an_empty_array_rather_than_nothing() {
        assert_eq!(payload(&[]), "[]");
    }
}

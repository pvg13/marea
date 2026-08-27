//! Synced tables.
//!
//! Every entity here is registered with WaveSyncDB under [`crate::REGISTRY`]
//! and replicated to the account's other devices. Adding one means adding the
//! module here and a `.register()` line in `bootstrap::native`.
{% if with_example %}
pub mod item;
{%- else %}
// Your entities go here.
{%- endif %}

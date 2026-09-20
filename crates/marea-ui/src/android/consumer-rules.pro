# Rust resolves these by name through the app classloader, so R8 must not
# rename or strip them.
-keep class dev.dioxus.main.MareaReminders { *; }
-keep class dev.dioxus.main.MareaAlarmReceiver { *; }
-keep class dev.dioxus.main.MareaBootReceiver { *; }

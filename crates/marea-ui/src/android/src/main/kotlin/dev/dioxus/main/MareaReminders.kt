package dev.dioxus.main

import android.app.Activity
import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import org.json.JSONArray
import org.json.JSONObject

/**
 * Local, scheduled reminders: notifications the device posts to itself at a
 * time the app asked for, with nothing on the network and no server involved.
 *
 * Called from Rust (`marea_ui::notify`) through the app classloader:
 * `MareaReminders.replaceAll(context, json)` / `cancelAll(context)`.
 *
 * ## Why the whole set is replaced every time
 *
 * The caller owns the schedule; this object owns nothing but the arming. Rust
 * recomputes every reminder it wants live and hands the list over, so there is
 * no incremental state to drift out of step with the app's own data. It makes
 * the call idempotent, which is what lets it run on every launch and after
 * every edit without bookkeeping.
 *
 * ## Why the alarms are inexact
 *
 * [AlarmManager.setAndAllowWhileIdle] rather than `setExactAndAllowWhileIdle`.
 * Exact alarms need `SCHEDULE_EXACT_ALARM` / `USE_EXACT_ALARM`, which Play
 * restricts to alarm clocks and calendars — a reminder to put a wash on is
 * neither, and declaring it is a review rejection. Inexact means "within a few
 * minutes, later rather than earlier, and longer in Doze", which for a
 * household chore is indistinguishable from what was asked for.
 *
 * ## Why the set is persisted
 *
 * Alarms do not survive a reboot. Every armed reminder is stored as JSON in
 * private SharedPreferences so [MareaBootReceiver] can re-arm the ones still in
 * the future. The store is the arming record, not app data: it holds only what
 * the caller already chose to put on the lock screen.
 */
object MareaReminders {
    private const val TAG = "MareaReminders"
    private const val CHANNEL_ID = "marea_reminders"
    private const val PREFS = "marea_reminders"
    private const val KEY_ARMED = "armed"

    /** Arbitrary, and never read back: nothing here waits for the answer. */
    private const val REQUEST_CODE = 8731

    const val EXTRA_ID = "marea.reminder.id"
    const val EXTRA_TITLE = "marea.reminder.title"
    const val EXTRA_BODY = "marea.reminder.body"
    const val EXTRA_DEEPLINK = "marea.reminder.deeplink"

    /**
     * Arm exactly this set and forget every reminder armed before it.
     *
     * [json] is an array of `{id, at, title, body, deeplink}`, `at` in epoch
     * milliseconds. Entries already in the past are dropped rather than fired
     * immediately: a phone that was off all morning should not wake up to four
     * notifications about chores whose moment has passed.
     */
    @JvmStatic
    fun replaceAll(context: Context, json: String) {
        val app = context.applicationContext
        cancelAll(app)

        val now = System.currentTimeMillis()
        val kept = JSONArray()
        val parsed = try {
            JSONArray(json)
        } catch (e: Exception) {
            Log.w(TAG, "replaceAll: unreadable payload, nothing armed: ${e.message}")
            return
        }

        for (i in 0 until parsed.length()) {
            val item = parsed.optJSONObject(i) ?: continue
            val at = item.optLong("at", 0L)
            if (at <= now) continue
            if (arm(app, item, at)) kept.put(item)
        }

        app.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .putString(KEY_ARMED, kept.toString())
            .apply()
    }

    /** Cancel every reminder this object armed, and forget them. */
    @JvmStatic
    fun cancelAll(context: Context) {
        val app = context.applicationContext
        val prefs = app.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val stored = prefs.getString(KEY_ARMED, null)
        if (stored != null) {
            val alarms = app.getSystemService(Context.ALARM_SERVICE) as? AlarmManager
            val parsed = try {
                JSONArray(stored)
            } catch (e: Exception) {
                JSONArray()
            }
            for (i in 0 until parsed.length()) {
                val item = parsed.optJSONObject(i) ?: continue
                val id = item.optString("id")
                if (id.isEmpty()) continue
                alarms?.cancel(pendingIntent(app, item, id))
            }
        }
        prefs.edit().remove(KEY_ARMED).apply()
    }

    /**
     * Re-arm the stored set after a reboot. Called by [MareaBootReceiver].
     *
     * Reads and rewrites through [replaceAll] so anything that fell into the
     * past while the phone was off is dropped by the same rule as everywhere
     * else, rather than by a second copy of it.
     */
    @JvmStatic
    fun rearm(context: Context) {
        val app = context.applicationContext
        val stored = app.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .getString(KEY_ARMED, null) ?: return
        replaceAll(app, stored)
    }

    /** Arm one alarm. Returns whether it was actually armed. */
    private fun arm(context: Context, item: JSONObject, at: Long): Boolean {
        val id = item.optString("id")
        if (id.isEmpty()) return false
        val alarms = context.getSystemService(Context.ALARM_SERVICE) as? AlarmManager ?: return false
        return try {
            alarms.setAndAllowWhileIdle(
                AlarmManager.RTC_WAKEUP,
                at,
                pendingIntent(context, item, id),
            )
            true
        } catch (e: Exception) {
            // The per-app alarm cap, or a manufacturer restriction. One
            // reminder that could not be armed is not a reason to lose the
            // rest, so this is logged and skipped.
            Log.w(TAG, "could not arm '$id': ${e.message}")
            false
        }
    }

    /**
     * The alarm's intent. `requestCode` is the reminder's own id, so re-arming
     * the same id replaces its alarm rather than stacking a second one, and
     * cancelling finds the same PendingIntent the arming created.
     */
    private fun pendingIntent(context: Context, item: JSONObject, id: String): PendingIntent {
        val intent = Intent(context, MareaAlarmReceiver::class.java).apply {
            // Distinct per id: PendingIntent equality ignores extras, and two
            // intents that differ only in their extras are the same intent.
            data = Uri.parse("marea-reminder://$id")
            putExtra(EXTRA_ID, id)
            putExtra(EXTRA_TITLE, item.optString("title"))
            putExtra(EXTRA_BODY, item.optString("body"))
            putExtra(EXTRA_DEEPLINK, item.optString("deeplink"))
        }
        return PendingIntent.getBroadcast(
            context,
            id.hashCode(),
            intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
    }

    /** Post one reminder. Called by [MareaAlarmReceiver] when an alarm fires. */
    @JvmStatic
    fun post(context: Context, id: String, title: String, body: String, deeplink: String) {
        ensureChannel(context)

        val icon = context.applicationInfo.icon
            .takeIf { it != 0 && Build.VERSION.SDK_INT >= Build.VERSION_CODES.P }
            ?: android.R.drawable.ic_popup_reminder

        val builder = NotificationCompat.Builder(context, CHANNEL_ID)
            .setContentTitle(title)
            .setContentText(body)
            .setSmallIcon(icon)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setAutoCancel(true)
        contentIntent(context, id, deeplink)?.let { builder.setContentIntent(it) }

        try {
            NotificationManagerCompat.from(context).notify(id.hashCode(), builder.build())
        } catch (e: SecurityException) {
            // POST_NOTIFICATIONS declined at runtime. Nothing to recover: the
            // person said no, and the app asks again only if they ask it to.
            Log.w(TAG, "reminder '$id' not posted (permission denied)")
        }
        // Drop it from the armed set — it has happened.
        forget(context, id)
    }

    /**
     * Whether the app may post notifications at all.
     *
     * **Both** checks, because there are two independent ways a notification
     * is silently dropped and neither implies the other:
     *
     * * `areNotificationsEnabled()` is the app's notification toggle, which a
     *   person can switch off in system settings long after granting anything;
     * * `POST_NOTIFICATIONS` is the runtime permission, which exists from
     *   API 33 and is what `notify()` actually throws on.
     *
     * They normally track each other, and the case that proved they do not is
     * ordinary enough to matter: clearing the app's data revokes the
     * permission while leaving the app-op that the first call reads set to
     * allow. A screen trusting only the first then promises a reminder that
     * will never be posted.
     */
    @JvmStatic
    fun canPost(context: Context): Boolean {
        if (!NotificationManagerCompat.from(context).areNotificationsEnabled()) return false
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return true
        return ContextCompat.checkSelfPermission(context, "android.permission.POST_NOTIFICATIONS") ==
            PackageManager.PERMISSION_GRANTED
    }

    /**
     * Ask for the notification permission, if there is anything to ask.
     *
     * On Android 13+ a notification the app was never given permission to post
     * is simply dropped, and the permission is **only** grantable through this
     * prompt — an app that never asks is an app whose reminders never work for
     * anyone who installs it. Below 33 the permission does not exist and this
     * is a no-op.
     *
     * Needs the **Activity**, not the application context, so it deliberately
     * does not call `applicationContext` the way the arming path does.
     * Returns false when there is nothing to ask — already granted, too old an
     * Android, or no Activity — so the caller can fall back to telling the
     * person where the setting lives.
     *
     * The system stops showing the prompt after two refusals. That is
     * indistinguishable from here, which is why the caller keeps its "you can
     * turn these on in settings" line rather than replacing it with a button.
     */
    @JvmStatic
    fun requestPermission(context: Context): Boolean {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return false
        val activity = context as? Activity ?: return false
        val permission = "android.permission.POST_NOTIFICATIONS"
        if (ContextCompat.checkSelfPermission(activity, permission)
            == PackageManager.PERMISSION_GRANTED
        ) {
            return false
        }
        return try {
            ActivityCompat.requestPermissions(activity, arrayOf(permission), REQUEST_CODE)
            true
        } catch (e: Exception) {
            Log.w(TAG, "could not ask for the notification permission: ${e.message}")
            false
        }
    }

    private fun forget(context: Context, id: String) {
        val prefs = context.applicationContext
            .getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        val stored = prefs.getString(KEY_ARMED, null) ?: return
        val parsed = try {
            JSONArray(stored)
        } catch (e: Exception) {
            return
        }
        val kept = JSONArray()
        for (i in 0 until parsed.length()) {
            val item = parsed.optJSONObject(i) ?: continue
            if (item.optString("id") != id) kept.put(item)
        }
        prefs.edit().putString(KEY_ARMED, kept.toString()).apply()
    }

    /**
     * Tap-to-open, with the caller's opaque deeplink as ACTION_VIEW data on an
     * explicit launch intent — no manifest filter, no app chooser. The same
     * shape WaveSyncDB's notification helper uses, so an app routes both kinds
     * of tap in one place.
     */
    private fun contentIntent(context: Context, id: String, deeplink: String): PendingIntent? {
        val launch = context.packageManager
            .getLaunchIntentForPackage(context.packageName) ?: return null
        if (deeplink.isNotEmpty()) {
            launch.action = Intent.ACTION_VIEW
            launch.data = Uri.parse(deeplink)
            launch.removeCategory(Intent.CATEGORY_LAUNCHER)
            // Without these a warm tap stacks a second activity — a second
            // webview and a second engine for a Dioxus app — instead of
            // routing inside the one already running.
            launch.addFlags(
                Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP,
            )
        }
        return try {
            PendingIntent.getActivity(
                context,
                id.hashCode(),
                launch,
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
            )
        } catch (e: Exception) {
            Log.w(TAG, "content intent not attached: ${e.message}")
            null
        }
    }

    private fun ensureChannel(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
        if (nm.getNotificationChannel(CHANNEL_ID) != null) return
        // Its own channel, separate from WaveSyncDB's "you got new data" one,
        // so a person can silence sync chatter and keep their reminders — or
        // the other way round.
        val channel = NotificationChannel(
            CHANNEL_ID,
            "Reminders",
            NotificationManager.IMPORTANCE_DEFAULT,
        ).apply {
            description = "Reminders you asked this app for"
        }
        nm.createNotificationChannel(channel)
    }
}

/** Posts one reminder when its alarm fires. */
class MareaAlarmReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val id = intent.getStringExtra(MareaReminders.EXTRA_ID) ?: return
        MareaReminders.post(
            context,
            id,
            intent.getStringExtra(MareaReminders.EXTRA_TITLE).orEmpty(),
            intent.getStringExtra(MareaReminders.EXTRA_BODY).orEmpty(),
            intent.getStringExtra(MareaReminders.EXTRA_DEEPLINK).orEmpty(),
        )
    }
}

/** Re-arms the stored reminders after a reboot or an app update. */
class MareaBootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        MareaReminders.rearm(context)
    }
}

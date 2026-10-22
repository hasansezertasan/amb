## Mixer state shared through atomics, not locks

**Id:** 12e3434c-09c6-4190-bc58-db0f8d3c99c3
**Type:** decision
**Status:** open
**Evidence:** unknown
**Source:** code reading, `src/audio.rs`; the author's own session, which recorded no reason (dated conversation, 2026-10-22)
**Verification:** uncorroborated
**Revisit when:** the audio callback needs state more complex than per-channel levels, master volume and a pause flag

Levels, master volume and the paused flag are `AtomicU32`/`AtomicBool` values
(floats stored as bits) read by the audio callback and written by the UI
thread, instead of sitting behind a `Mutex`.

**Reason:** unknown. The only recorded text is the doc comment "Levels
shared between the UI thread and the audio callback." Keeping the real-time
callback from blocking is the obvious guess, but nobody stated it, so it is
not recorded as the reason.

**Rejected alternative:** unknown — whether a `Mutex` or a channel was
weighed was never written down.

**Related:** a later review found that a NaN in the levels file would flow
through the atomics to the output; `sanitize()` and a test were added. That
fixed the review finding and is not the original reason for atomics.

## Level and master gains are squared

**Id:** 7463c139-d164-4259-98fd-0d908de60cb5
**Type:** decision
**Status:** open
**Evidence:** unknown
**Source:** code reading, `src/audio.rs` (`frame`: `t * t` and `target_master * target_master`); the author's own session, which recorded no reason (dated conversation, 2026-10-22)
**Verification:** uncorroborated

The target gain for each sound, and for the master volume, is the slider
value squared before it is smoothed and applied.

**Reason:** unknown. No comment, commit or discussion explains the squaring;
the nearby comment about smoothing (about 30 ms, "so level changes never
click") does not cover it. A perceived-loudness curve is the obvious guess
but unverified.

**Rejected alternative:** unknown — linear and dB mappings were never
compared on record, and the squaring was never tuned by listening: nobody has
heard the app yet, only automated tests have run.

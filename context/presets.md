## Built-in presets are seeded into an editable file

**Id:** ffa12e44-318b-4d21-bbed-53a9eb09f8f9
**Type:** decision
**Status:** active
**Evidence:** confirmed
**Source:** maintainer conversation, 2026-10-26; PR review thread on the empty-file case, 2026-10-26
**Revisit when:** presets need to carry more than sound levels, or a built-in has to change without touching users' files

Presets come from `presets.toml` in the config dir. On first run, when the
file is missing, the built-in presets are written into it, so the user starts
with working presets and can edit, rename or delete them. After that the file
is the source of truth: a file that exists is used as it is, even with every
section removed, and the built-ins are not written back.

**Reason:** the maintainer wanted presets to be both built-in and
user-editable instead of choosing one ("why not both?"). Seeding the file
gives defaults on day one and keeps them ordinary, deletable data.

**Rejected alternative:** built-in presets only in code, or user-saved presets
only with nothing to start from. Rejected because the maintainer wanted both
at once.

**Consequence:** a built-in that ships in a later release does not appear for
anyone whose file already exists. An emptied file stays empty until the user
resets it with `R` (see the reset entry below) or deletes the file.

## Presets switch instantly

**Id:** 55373b6f-71f4-4d89-9d9e-a3f330bb62a6
**Type:** decision
**Status:** active
**Evidence:** confirmed
**Source:** maintainer conversation, 2026-10-26
**Revisit when:** the instant jump between mixes is noticeable or unpleasant when listened to

Applying a preset sets every sound's level at once; there is no cross-fade
between the old and the new mix. The audio callback already smooths each
level change over about 30 ms (see `audio.md`), so the jump is not a click.

**Reason:** "keep it simple", in the maintainer's words.

**Rejected alternative:** a gradual fade from the old levels to the preset's.
Rejected for simplicity; nobody has listened to the instant switch yet, so
whether it sounds abrupt is untested.

## Saving a preset overwrites one with the same name

**Id:** 1d94e889-eed3-4bee-a077-1bdcf41f19de
**Type:** decision
**Status:** active
**Evidence:** unknown
**Source:** maintainer conversation, 2026-10-26

Saving the current mix under a name that already exists, compared ignoring
case, replaces that preset instead of adding a second one. The decision is
confirmed; the reason is not.

**Reason:** unknown. The maintainer chose overwriting when asked, without
saying why.

**Rejected alternative:** refusing a duplicate name, which was offered in the
same question. Why it lost is not recorded.

## Presets hold levels only, in a file separate from the remembered mix

**Id:** d1414a58-5669-4eb8-adb0-77416e7cb9c7
**Type:** decision
**Status:** active
**Evidence:** inferred
**Source:** reasoning given by the coding agent while building the feature, 2026-10-26; code reading, `src/state.rs`, `src/presets.rs`
**Verification:** uncorroborated
**Revisit when:** a maintainer wants one config file, or presets should include master volume

A preset stores each sound's level and not the master volume. The mix that is
remembered between runs (`levels`, written on every quit) stays a separate file
from the presets (`presets.toml`, which the user edits by hand).

**Reason:** inferred, not confirmed by a maintainer. `levels` is rewritten by
the program on every exit while `presets.toml` is the user's own file, so
merging them would mean rewriting a hand-edited file on every quit. Master
volume is a listening setting more than part of a mix.

**Rejected alternative:** one combined file, for example a `[last]` section in
`presets.toml`. The maintainer asked whether combining made sense; the answer
given was to keep two files, and the work went ahead on that basis without an
explicit confirmation either way. Master volume inside presets was listed as a
possible follow-up and not chosen.

## presets.toml is a hand-parsed TOML subset, edited one section at a time

**Id:** de47a10c-ad91-42bc-a0c9-06266450824f
**Type:** decision
**Status:** active
**Evidence:** inferred
**Source:** reasoning given by the coding agent while building the feature, 2026-10-26; code reading, `src/presets.rs`, `src/state.rs`
**Verification:** uncorroborated
**Revisit when:** presets need nested values, strings or arrays, or the file format has to be shared with another tool

The file only understands `[name]` sections of `sound = level` lines and is
parsed by hand, the way `levels` already is. Saving, renaming and deleting
edit only the matching section's text and leave the rest of the file,
comments included, as written.

**Reason:** inferred, not confirmed by a maintainer. It avoids a new
dependency for a format this small, and rewriting the whole file from parsed
data would drop the user's comments.

**Rejected alternative:** the `toml` crate with a parse-and-reserialise cycle.
Whether it was weighed by a maintainer is not recorded; it was not chosen
because of the two points above.

**Consequence:** comments placed inside a section that is replaced are lost;
comments between sections are kept.

## Renaming onto an existing preset's name is refused

**Id:** d87b5ae1-bbb2-48a7-b2a1-7cdb5e754ea1
**Type:** decision
**Status:** active
**Evidence:** inferred
**Source:** maintainer conversation, 2026-10-26; code reading, `src/library.rs` (`Library::rename`)
**Verification:** uncorroborated
**Revisit when:** a maintainer wants rename to overwrite like save does

Renaming the active preset to a name another preset already has (compared
ignoring case) changes nothing and shows "a preset named X already exists".
Saving under an existing name does the opposite and overwrites (see
`1d94e889-eed3-4bee-a077-1bdcf41f19de`, as of 2026-10-26).

The maintainer was told about the difference and asked for it to be recorded;
the behaviour is confirmed. The reason below is the coding agent's own.

**Reason:** inferred. Saving replaces a preset with the mix the user is
looking at and just chose a name for. A rename would instead make a different
preset vanish as a side effect of changing a label, which is easy to do by
accident.

**Rejected alternative:** overwriting on rename for consistency with save. Not
chosen when the feature was built; whether the maintainer weighed it is not
recorded.

## Reset replaces every preset with the built-ins

**Id:** 8759256f-864f-482c-8fd4-0c941fdaca31
**Type:** decision
**Status:** active
**Evidence:** unknown
**Source:** maintainer conversation, 2026-10-26; code reading, `src/library.rs` (`Library::reset`), `src/presets.rs` (`FileStore::reset`)
**Revisit when:** someone loses presets they wanted to keep, or a way to restore only the built-ins is wanted

`R`, after a `y` to confirm, rewrites `presets.toml` as just the built-in
presets. Presets the user saved are deleted and so are any comments in the
file. Only the presets are reset; the remembered mix (`levels`) and the master
volume are not touched.

The decision, and that it covers presets only, are confirmed.

**Reason:** unknown. The maintainer picked this when asked and gave no reason.

**Rejected alternative:** restoring the built-ins (re-adding missing ones and
resetting edited ones) while keeping the user's own presets and the comments
between sections. The coding agent recommended it as the less destructive
option and the maintainer chose the full reset instead; why is not recorded.
Also not chosen: resetting the remembered mix and master volume along with the
presets.

**Consequence:** the confirmation prompt says it deletes the user's own
presets, since a reset cannot be undone from inside the app.

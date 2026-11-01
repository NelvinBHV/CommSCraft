# Third-party UI sound rights review — 2026-11-01

This document records source-based research for two **proposed** CommSCraft sounds. It is **not** a certificate from GEMA, legal counsel, Freesound or the creators. The actual source audio files have **not** been downloaded, auditioned, hashed or bundled in this change.

## Ringtone — issue #83

- Title / creator: **Simple ringtone**, **TheZero**
- Exact source: https://freesound.org/people/TheZero/sounds/273540/
- Source page checked 2026-11-01. The page explicitly labels **Creative Commons 0 (CC0 1.0)** for this specific sound; release permits copying, modification, redistribution and commercial use without mandatory attribution.
- Published 2015-05-06; MP3, 7.093 s, 44.1 kHz, 128 kbps, stereo.
- Creator describes it as self-made and loopable. This is a creator statement, not an independent audit of the actual waveform or third-party sample origins.
- CC0 legal summary: https://creativecommons.org/publicdomain/zero/1.0/deed.de

**Assessment:** CC0 is suitable for distributing this recording as a separately licensed asset with open-source software, assuming the uploader has the relevant rights. Nothing on the source page establishes collecting-society membership or proves GEMA-free status. The mere fact that it is called a ringtone does not place it under the GEMA ringtone tariff, and its duration alone does not prove it is not a protectable music work. Use a case-specific sound-rights assessment, not blanket GEMA certification.

## Notification sound — issue #84

- Title / creator: **Magical UI Selection Stinger - Mysterious Fantasy Game SFX**, **TommasoMotteran**
- Exact source: https://freesound.org/people/TommasoMotteran/sounds/875968/
- Source page checked 2026-11-01 via indexed Freesound record; it specifically labels **Attribution 4.0 (CC BY 4.0)**.
- Published 2026-10-26; WAV, 1.962 s, 44.1 kHz, 24-bit, stereo.
- Creator describes synthesizer-based creation using Podolski and original sound samples; this is a self-report, not an independent rights warranty.
- CC BY 4.0 summary: https://creativecommons.org/licenses/by/4.0/deed.de

**Assessment:** Suitable for distribution as a distinct CC BY 4.0 asset with correct attribution, source URL and notice of audio conversions. A synthesized stinger can potentially be musical; neither the short duration nor the upload category excludes a protected music work. Source page does not independently establish lack of society administration.

Suggested notice once the actual file is bundled:

> “Magical UI Selection Stinger - Mysterious Fantasy Game SFX” by TommasoMotteran, source: https://freesound.org/people/TommasoMotteran/sounds/875968/, licensed under CC BY 4.0: https://creativecommons.org/licenses/by/4.0/. Changes: [describe the conversion or editing actually done, or state none].

## German law and scope

- [UrhG §2](https://www.gesetze-im-internet.de/urhg/__2.html): a protected musical work requires a personal intellectual creation; not every sound or short alert automatically qualifies, but short musical passages are not categorically exempt.
- [UrhG §85](https://www.gesetze-im-internet.de/urhg/__85.html): the recording/phonogram can have separate rights even where the underlying signal does not qualify as a musical composition. This is why recording-specific licensing matters.
- [UrhG §§16, 19a](https://www.gesetze-im-internet.de/urhg/__16.html): public repository/package distribution implicates reproduction and making-available rights, distinct from ordinary private playback through the local desktop client.
- [GEMA: GEMA presumption](https://www.gema.de/de/w/hilfe/allgemein/urheberrechte-nutzungsrechte/fragen-antworten/was-ist-die-gema-vermutung): GEMA explains the repertoire presumption for relevant **music** uses; the mere existence of a sound effect or the word “ringtone” is not proof that musical repertoire is used.
- [GEMA: music allegedly outside its repertoire](https://www.gema.de/de/hilfe/musiknutzer/preise-tarife/gema-freie-musik): the repertoire lookup is a preliminary, nonbinding indication, not a definitive negative certificate.
- [Creative Commons FAQ on collecting societies](https://creativecommons.org/faq/#can-i-use-a-creative-commons-license-if-i-am-a-member-of-a-collecting-society): collecting society membership can constrain an uploader's ability to grant CC rights in repertoire represented by that society. CC0/CC BY are not independent guarantees of society non-membership.
- CC0 and CC BY 4.0 do not grant rights owned by unidentified third parties and do not independently guarantee the sound is free from other claims.

## Decision / remaining work

1. **Verified:** the exact Freesound postings state CC0 (#83) and CC BY 4.0 (#84); both license types permit commercial redistribution and conversion, with BY attribution required for #84.
2. **Not verified:** original uploader's right to license every constituent element, whether the actual sound contains protectable musical material, and whether any relevant collecting society administers that material.
3. **No blanket GEMA clearance is inherently required for a mere UI signal.** If review of the actual recording identifies a plausible protected composition or a specific third-party claim, escalate to author/rights-holder confirmation and/or a documented repertoire search before distributing it.
4. Archive a dated copy of both licensing pages, download the authorized original files, note SHA-256 checksums and modification history, and include an attribution/provenance file in binary/source distributions. A dated source link is recorded here, but no source snapshot is archived.
5. Do **not** characterize either sound as “certified GEMA-free” or close its integration issue on this research alone.

Related: [#82](https://github.com/NelvinBHV/CommSCraft/issues/82) (full-length Starlight music-on-hold: separate and more demanding clearance), [#83](https://github.com/NelvinBHV/CommSCraft/issues/83), [#84](https://github.com/NelvinBHV/CommSCraft/issues/84).

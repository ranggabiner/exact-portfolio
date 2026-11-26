# Lumen onboarding

Single source for the onboarding script. Rules follow binerlabs-factory:
profiles `onboarding/value-first`, `components/native-hig`, `interaction/native-hig`,
`icons/sf-symbols`, `mascot/none`, `illustration/none`, `voice/warm-clear`.
Copy lives in `strings/en.json` and `strings/id.json`; the segmented language switch on the
welcome screen changes it without a restart.

## 1. Summary

- **User:** busy iPhone owners whose to-dos live in their head and in scattered notes.
- **Problem:** unfinished tasks keep intruding, pings pull attention away, plans run long.
- **Promise:** say or type it, Lumen sorts it on the phone, and the day stays realistic.
- **Steps:** 23 (welcome, 3 problems, 13 questions, 3 previews, plan, icon, permission).
- **Monetisation:** none. The owner asked for onboarding straight into the list, so there is
  no paywall step. Section 7 records what a paywall hero would reuse.

## 2. Steps

Skip is available on steps 4–17, 21 and 22 and always keeps the listed default.

| # | step | input | skip default | answer is used by | source |
| - | ---- | ----- | ------------ | ----------------- | ------ |
| 0 | welcome | English / Indonesia switch | device language | all copy | none |
| 1 | problem: open loops | tilt and tap physics bubbles | – | – | F1 |
| 2 | problem: pings | – | – | – | F2 |
| 3 | problem: plans run long | animated bars | – | – | F3 |
| 4 | name | text | empty | greeting, plan title, fit title | F4 |
| 5 | role | single: student, professional, founder, creator, caregiver, other | professional | starter tasks | F5 |
| 6 | struggles | multi: forgetting, overwhelm, procrastination, scattered, time, starting | none | Top 3 card, two-minute and "pick for me" suggestions, plan rows, fit screen | F6 |
| 7 | areas of life | multi: work, personal, study, health, home, money, family | work, personal | lists created, smart list matching | F7 |
| 8 | focus window | single: early, morning, midday, afternoon, evening, night | morning | focus suggestion on Today, plan | F8 |
| 9 | wake and wind-down | two time pickers | 07:00, 22:00 | morning plan, evening review, "close your day" card | F9 |
| 10 | daily capacity | slider 1–12 | 5 | overload warning, Balance my day, plan | F10 |
| 11 | focus length | 15, 25, 45, 60 | 25 | focus timer, Live Activity, starter estimates | F11 |
| 12 | capture style | type, speak, brain dump | type | mode the capture sheet opens in, plan | F12 |
| 13 | reminder style | gentle, persistent, none | gentle | notification scheduling | F13 |
| 14 | celebration | confetti, calm, haptic (each previews) | confetti | end-of-day celebration | F14 |
| 15 | weekly review day | Friday, Sunday, Monday | Friday | review card and notification | F15 |
| 16 | first thing on your mind | text or suggestion chips | empty | first task, due today | F16 |
| 17 | solution: say it, it's sorted | voice (SpeechAnalyzer), text, example, Apple Intelligence | – | recognised tasks are added to the list | – |
| 18 | solution: built around you | answers mapped to features | – | – | – |
| 19 | solution: one glance away | Dynamic Island demo (ActivityKit), widget, control and Siri previews with the person's tasks | – | – | – |
| 20 | plan | rows computed in code; summary streamed by Foundation Models, checked, else template | – | – | – |
| 21 | app icon | Aurora, Midnight, Sunrise, Mono (`setAlternateIconName`) | Aurora | home screen | – |
| 22 | permission | notifications | not asked | reminders and nudges | – |

## 3. Facts

Checked 2026-11-27 by opening each source. Numbers are the source's own.
A seventeenth candidate (Ariely and Wertenbroch, 2002) was dropped: PubMed marks it retracted.

| id | claim in the app | publisher, year, URL | supporting text |
| -- | ---------------- | -------------------- | --------------- |
| F1 | Specific plans stopped intrusions from unfinished goals | J. Pers. Soc. Psychol., 2011, https://pubmed.ncbi.nlm.nih.gov/21688924/ | "Allowing participants to formulate specific plans for their unfulfilled goals eliminated the various activation and interference effects." |
| F2 | Interrupted people worked faster but felt more stress | ACM CHI, 2008, https://www.ics.uci.edu/~gmark/chi08-mark.pdf | "people compensate for interruptions by working faster, but this comes at a price: experiencing more stress, higher frustration, time pressure and effort." |
| F3 | 33.9 days expected, 55.5 actual, 29.7% on time | J. Pers. Soc. Psychol., 1994, https://bear.warrington.ufl.edu/brenner/mar7588/Papers/buehler-et-al-1994.pdf | "they would finish in 33.9 days, but they actually took 55.5 days … (29.7%) finished in the time they reported" |
| F4 | Phone reminders are cognitive offloading | Trends Cogn. Sci., 2016, https://pubmed.ncbi.nlm.nih.gov/27542527/ | "programmed a smartphone to remind you of an upcoming appointment, you have engaged in cognitive offloading" |
| F5 | Slower, more error-prone right after a switch | Trends Cogn. Sci., 2003, https://pubmed.ncbi.nlm.nih.gov/12639695/ | "responses are substantially slower and, usually, more error-prone immediately after a task switch." |
| F6 | Aversive and delayed tasks predict procrastination | Psychol. Bull., 2007, https://pubmed.ncbi.nlm.nih.gov/17201571/ | "based on 691 correlations … Strong and consistent predictors of procrastination were task aversiveness, task delay…" |
| F7 | Friday's unfinished work linked to worse weekend sleep | J. Occup. Health Psychol., 2017, https://pubmed.ncbi.nlm.nih.gov/27101340/ | "relationship between unfinished tasks and sleep is mediated by affective rumination" (59 employees, 12 weeks) |
| F8 | Time of day affects cognition, by chronotype | Cogn. Neuropsychol., 2007, https://pubmed.ncbi.nlm.nih.gov/18066734/ | "time-of-day modulations affect performance … contingent upon the chronotype" |
| F9 | Bedtime to-do list: fell asleep faster | J. Exp. Psychol. Gen., 2018, https://pubmed.ncbi.nlm.nih.gov/29058942/ | "Participants in the to-do list condition fell asleep significantly faster" (57 adults, sleep lab) |
| F10 | Planning benefit did not extend to many goals | J. Consum. Res., 2012, https://doi.org/10.1086/664500 | "the benefits of implemental planning for attaining a single goal do not typically extend to multiple goals." |
| F11 | Micro-breaks lift vigour, lower fatigue | PLOS ONE, 2022, https://pubmed.ncbi.nlm.nih.gov/36044424/ | "22 independent study samples … boosting vigor (d = .36), reducing fatigue (d = .35)" |
| F12 | Speaking 2.93× faster than typing (lab, English) | Proc. ACM IMWUT, 2017, https://arxiv.org/abs/1608.07323 | "the English input rate was 2.93 times faster (153 vs. 52 WPM)" |
| F13 | A notification alone disrupts attention | J. Exp. Psychol. HPP, 2015, https://pubmed.ncbi.nlm.nih.gov/26121498/ | "notifications alone significantly disrupted performance on an attention-demanding task" |
| F14 | Positive feedback raised interest, expected tangible rewards lowered it | Psychol. Bull., 1999, https://pubmed.ncbi.nlm.nih.gov/10589297/ | "Positive feedback enhanced … self-reported interest (d = 0.31)" |
| F15 | Monitoring progress promotes goal attainment | Psychol. Bull., 2016, https://pubmed.ncbi.nlm.nih.gov/26479070/ | "138 studies (N = 19,951) … promoted goal attainment (d+ = 0.40) … larger … when the information was physically recorded." |
| F16 | If-then plans across 94 studies, 8,000+ people | Gollwitzer & Sheeran 2006, quoted in https://kops.uni-konstanz.de/server/api/core/bitstreams/eb48c8cf-7855-478e-90ec-1ea6d44f8252/content | "involving over 8,000 participants in 94 independent studies reported an effect size of d = 0.65." |

## 4. App icons

| id | name | look |
| -- | ---- | ---- |
| aurora | Aurora (primary) | violet night, ring of light around a check |
| midnight | Midnight | near black, violet-to-mint ring |
| sunrise | Sunrise | coral to gold, white ring |
| mono | Mono | paper white, ink ring |

Drawn by `swift tools/icons.swift`.

## 5. Apple technology by step

- Every step: `MeshGradient` aurora that leans with Core Motion (still under Reduce Motion and
  Low Power Mode), spring transitions, SF Symbol effects, selection and chapter haptics.
- Welcome: floating glass cards with device-motion parallax.
- Problem 1: SpriteKit bodies under real gravity; popping one plays a Core Haptics tap.
- Demo: on-device speech recognition, Foundation Models structured output, parser fallback.
- Previews: ActivityKit, WidgetKit, App Intents (Siri), Control Center control.
- Plan: Foundation Models streaming, numbers verified in Rust; confetti with Core Haptics.

## 6. Permission budget (at most two system dialogs)

1. Microphone, only if the person taps the mic on step 17 (SpeechAnalyzer needs no separate
   speech authorisation on iOS 26 and later; older systems also ask for speech recognition).
2. Notifications, on step 22.

Motion, haptics, alternate icon and Live Activities ask for nothing.

## 7. Not built

- Paywall and trial steps (owner's instruction). A hero would replay the plan from step 20 and
  the recognised tasks from step 17.
- Mobbin references: not consulted in this pass.

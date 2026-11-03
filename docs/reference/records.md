# Crew records

Sources: `luna:code/game/machinery/computer/computer.dm` (`datacore/manifest`), `medical.dm`,
`luna:code/defines/obj/computer.dm`; `NSV_Luna.dmm`: 8 security and 6 medical records computers.

Code: `game/classic/src/records.rs`, `apps/client/src/records_panel.rs`. Tests:
`game/classic/tests/records.rs`.

Click a records computer, with power and the right card held or worn, to open its window. It lists everyone
who has a job, with their name and job.

- **Security records** (`security-records`, `detective-records`) want `security` access, and show each
  person's criminal status. Clicking a line moves them on through None, Arrest, Incarcerated, Parolled and
  Released.
- **Medical records** (`medical-records`) want `medical` access, and show each person's condition.

## Differences from Luna

| Luna | Here |
|---|---|
| Records are made at round start and last, with a photo, age, fingerprints, DNA, blood type, notes and more | Only name, job, and one status, read from whoever has a job right now |
| You log in with a card put in the computer | You only need to hold or wear it |
| Medical staff set someone's condition by hand | It's their real condition, so the window can't be wrong or out of date |
| Records can be searched, printed, edited and deleted | Not built |
| Security HUDs and Beepsky read criminal status | Nothing reads it yet |

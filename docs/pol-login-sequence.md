# PlayOnline Login Key Sequence

Recorded 2026-02-07 using `login-rs --record`.

This documents the exact key sequence needed to automate logging into FFXI
through the PlayOnline Viewer, starting from the member selection screen.

## Sequence

| Step | Keys | Delay After | Description |
|------|------|-------------|-------------|
| 1. Slot navigation | UP/DOWN arrows | ~200ms between presses | Navigate to the correct member slot. Number of presses depends on current vs target slot. |
| 2. Select slot | ENTER | ~2000ms | Selects the highlighted member slot. |
| 3. Confirmation 1 | ENTER | ~1000ms | Dismiss first confirmation/info screen. |
| 4. Confirmation 2 | ENTER | ~1000ms | Dismiss second confirmation/info screen. |
| 5. Navigate to password field | UP, RIGHT, RIGHT, ENTER | ~500ms | From the on-screen keyboard area, navigate up to the password input row, move right twice to the input field, and press ENTER to focus it. |
| 6. Type password | Character keys | ~50ms between chars | Type the password using standard keyboard input. Shift is held for uppercase/symbols (e.g., Shift+F for "F", Shift+1 for "!"). |
| 7. Submit password | ENTER | ~500ms | Confirms the typed password / dismisses the text input UI. |
| 8. Connect | DOWN, ENTER | ~500ms | Navigate down to the Connect button and press it. |

## Notes

- Steps 3 and 4 (confirmation screens) were missing from the original automation
  and are required. Without them the subsequent navigation targets the wrong UI
  elements.
- Step 8 requires DOWN then ENTER. The original code used just ENTER, which
  was not reaching the Connect button.
- The on-screen keyboard navigation in step 5 assumes the cursor starts at the
  default position (the "0" key area). If POL changes its default focus, this
  may need re-recording.
- Delays are approximate minimums. The recorded session had human-speed delays;
  the automation uses shorter but safe delays.

## Raw Recording

```
#      Key                  Dir    Delay
--------------------------------------------------
0      DOWN                 DOWN   +0ms
1      DOWN                 UP     +122ms
2      UP                   DOWN   +539ms
3      UP                   UP     +78ms
4      ENTER                DOWN   +2105ms
5      ENTER                UP     +58ms
6      ENTER                DOWN   +2116ms
7      ENTER                UP     +69ms
8      ENTER                DOWN   +917ms
9      ENTER                UP     +61ms
10     UP                   DOWN   +924ms
11     UP                   UP     +104ms
12     RIGHT                DOWN   +421ms
13     RIGHT                UP     +105ms
14     RIGHT                DOWN   +275ms
15     RIGHT                UP     +112ms
16     ENTER                DOWN   +322ms
17     ENTER                UP     +59ms
18     LSHIFT               DOWN   +648ms
19     F                    DOWN   +344ms
20     LSHIFT               UP     +70ms
21     F                    UP     +14ms
22     I                    DOWN   +607ms
23     I                    UP     +77ms
24     N                    DOWN   +380ms
25     N                    UP     +57ms
26     N                    DOWN   +83ms
27     E                    DOWN   +63ms
28     N                    UP     +1ms
29     G                    DOWN   +81ms
30     E                    UP     +50ms
31     A                    DOWN   +26ms
32     G                    UP     +34ms
33     A                    UP     +78ms
34     N                    DOWN   +1ms
35     N                    UP     +70ms
36     2                    DOWN   +98ms
37     2                    UP     +90ms
38     LSHIFT               DOWN   +210ms
39     1                    DOWN   +125ms
40     1                    UP     +73ms
41     LSHIFT               UP     +15ms
42     ENTER                DOWN   +415ms
43     ENTER                UP     +74ms
44     DOWN                 DOWN   +509ms
45     DOWN                 UP     +79ms
46     ENTER                DOWN   +327ms
47     ENTER                UP     +46ms
```

Events 0-3: Manual slot navigation (application calculates this dynamically)
Events 4-5: Select slot (Step 2)
Events 6-9: Confirmation screens (Steps 3-4)
Events 10-17: Navigate to password field (Step 5)
Events 18-41: Password entry (Step 6)
Events 42-43: Submit password (Step 7)
Events 44-47: Connect (Step 8)

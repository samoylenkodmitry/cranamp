# Mobile appearance

Open Settings, then Appearance, then Theme. Day uses light controls and Night uses dark controls. Auto follows changes to the device appearance and is the default.

From skin background samples the top-left pixel of the active player skin. That color becomes the background of Library, Settings and Skin Studio. The interface chooses light or dark controls using the sampled color's relative luminance. Choosing a different skin updates the background immediately. An unavailable skin falls back to Auto.

The Player keeps its skin colors in every mode. The choice is saved under `cranamp.appearance.v1`; preference reads and writes run through Cranpose's blocking dispatcher. A late preference read cannot replace a newer choice, and one writer saves changes in order. A failed write leaves the current appearance active and offers Retry on the Theme screen.

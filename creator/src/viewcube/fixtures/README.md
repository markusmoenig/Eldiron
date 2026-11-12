# ViewCube reference fixtures

These 26 RGBA8 PNGs came from the unchanged `ViewCube::draw` at checkpoint
fdbe1e64, before paint extraction. Original capture passed in 4.13 seconds:
`/private/tmp/eldiron-viewcube-baseline.log`.

They cover the minimum visible 100×150 view and 800×360, default orbit, four
azimuth/elevation quadrants, all six axis snaps and both sides of the face
visibility threshold. The independent patterned background exposes translucent
face ordering and repeated triangle coverage. Fixture readers never regenerate
these files. Swash text metrics and full workflow/platform parity remain open.

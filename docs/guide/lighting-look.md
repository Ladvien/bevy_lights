# The lighting look

The numbers a cinematographer sets, as plain functions in `bevy_lights::model::cinematography`:
`key_to_fill` and `stops` (with `LOW_KEY` 8:1, `MOON_KEY` 4:1 and `HIGH_KEY` 2:1), `fill_for`,
`fire_lumens` (the lumens a pool of a given size and brightness needs), `blackbody_colour` (a light's
colour from its temperature), `blackbody_luminance` (that temperature's **magnitude** in cd/m², which a
glowing *surface* needs and a light's colour does not carry) and `white_balanced` (the same as a camera
balanced elsewhere records it). Size a fire by the pool it lights and colour it by the temperature it
burns at, instead of picking lumens and an RGB by eye.

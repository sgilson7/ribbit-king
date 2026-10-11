# Plan

The decisions, and why. Everything follows the friend's design (BRIEF.md); where the design was silent, this says what was chosen.

| # | Decision | Why |
| --- | --- | --- |
| P1 | Built on Long Time Comin's engine: Q16.16 core, instanced WebGL 2 draws, ink outlines, toon bands, fixed 60 ticks. | The house 3D engine, tested in three shipped games. |
| P2 | Ground is a heightfield (2 m grid) built from data shapes; drawn as a per-hole mesh with a tint per vertex (a new vertex attribute). | Collision and drawing share one surface; holes are written as hills, plateaus and ridges, not by hand. |
| P3 | The meter: a needle round a dial, clockwise from twelve, 1.3 s a lap. The red bar runs 105°–255°, and the gold 7° either side of its middle is perfect. Power falls to 72% at the bar's ends; early hooks, late slices, off the bar is a whiff at 30%. | "A circle that spins around and has a power bar with perfect shot in the middle." Distance is set before the swing with a power setting (the mallet's weight), so the meter judges the hit and does not choose the distance. |
| P4 | The catapult is a plank on a fulcrum with a striker pad. A mallet on a post swings down from the side, with a striped striker tower and bell beside it. The bell rings on a perfect hit. | "Similar to the ring the bell strength test games at carnivals." The mallet swings in from the side so the view behind the frog stays clear. |
| P5 | Air control: W A S D accelerate the frog sideways and along (7 m/s²), relative to the camera. | "Players take control of their frog as it flies." |
| P6 | Extra jump: Space in the air, carried up to three. Wings: a 6.5 s glide in which A and D bank, W dives and S climbs, trading speed for height. Mushroom: on touch, 24 m/s up and 15 m/s away. | The design's three power-ups. Power-ups come back every stroke, so each player gets the same course. |
| P7 | Flies are counted per frog across the round. Each frog can catch each fly once per hole. The tongue costs 10 flies, reaches 11 m, and works while aiming or flying. | "Like rings from Sonic", "collecting 10 flies allows… E… knockback". |
| P8 | Fly lines: the first line on each hole is placed along the arc of an imagined real shot, and the rest are straight or arched lines. | Flies at a fixed height under a 20 m-high arc were never caught (SECOND-ORDER S3). |
| P9 | Turn order as in golf: everyone tees off in order, then the frog farthest from the lotus plays. Hazards cost one stroke, with stroke and distance. A hole is given up at par + 5. | "Players all start at the same driving tee just like a game of regular golf." |
| P10 | Four holes, one per theme the design names: swamp (par 3), desert oasis (4), busy city street with traffic (4), volcano lava flow (5). | The design's four themes. The traffic is what makes the street busy. |
| P11 | Eight frogs in the roster: red-eyed tree frog, blue and yellow-banded dart frogs, tomato frog, cane toad, golden toad, horned frog, glass frog. | "Different colours and patterns like tree frogs and toads… large eyes." |
| P12 | CPU players: imagine shots over five lines × three lofts × nineteen powers; re-test the best eight against a weak or crooked hit; miss the meter by up to 22°. | So one person can play a full round. |
| P13 | The leader after each hole wears a crown on the course; the final card is headed "The Ribbit King". | The title, made into the prize. |
| P14 | No pointer lock; the mouse drags the camera. Touch buttons appear on coarse pointers. | A turn-based golf game needs the cursor for its screens. |
| P15 | Records (best per hole, best full round, rounds played) are kept in local storage, for human players only. | Allowed by Builder; a project decision. |

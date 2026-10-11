# Brief

## The request (Sam, 10 October 2026)

> Please use the Builder repo as a base in the directory above, and use the work and repos of Sam gilson at sgilson7.github.io, which can all be found one directory up. please build out the MVP and deploy it to my website. you have approval for creating a public github repo, github pages, and approval to deploy to my website when its ready. any time you need to make 2D art and any kind of asset for the game, use the tikz latex prompt you can find in other repos, and use sam comm final as an inspiration image, alongside the menu screens of all of the sgilson7 games which use a similar build pipeline for images. now build the mvp for the game and deploy it, dont stop till its deployed, good luck. here is the prompt from my friend to follow to create the game containing all design decisions: the game should be 3d.

## The design (a friend's, verbatim)

> The game is based on the fictional sport of Frolf, which is a golf-like game that is played with frogs. The frogs sit on catapults, which the player whacks with a hammer to send the frog flying into the air. This looks similar to the ring the bell strength test games at carnivals. Players will take turns hitting their frogs with a timing based power-meter, the meter is a circle that spins around and has a power bar with perfect shot in the middle of the bar. After hitting the frog, players subsequently take control of their frog as it flies through the air. The frogs can collect power-ups that are scattered throughout the course and these can have a variety of effects like extra-jumps, wings for soaring through the air like mario 64 wing cap, and explosive mushrooms that detonate and launch your frog away. There are also trampoline like spider-webs and lily pads that bounce the frogs when they land upon them. The goal is to get your frog into the goal shaped like a lotus flower at the end of the course. Your score is recorded based on the amount of strokes taken to get the frog into the lotus flower. Players all start at the same driving tee just like a game of regular golf. Collectable insects function like rings from sonic and are scattered throughout the course in lines. The insects look like fly enemies from the binding of Isaac. Collecting 10 flies allows frogs to shoot their tongue out by pressing E and knockback other frogs. The various frogs have different colors and patterns like tree frogs and toads, they also have large eyes. Maps can have different themes such as desert oasis, swamp, busy city street, and volcano lava flow.
> The game is called Ribbit King
> it was released on the GameCube in 2003.

## Round 2 (Sam, 10 October 2026)

> ribbit king is great, now make it 4 player online multiplayer using other multiplayer frameworks from builder and sgilson7 repos

Done means: up to four people, each in their own browser, play one round together. They meet by a code, the way Floodline and Vagrancy do, with no server of ours. The game stays deployed on Pages. The earlier approvals (public repository, Pages, website) stand.

## Done means

A 3D browser game, deployed on GitHub Pages and listed on sgilson7.github.io, in which up to four frogs (people or CPU) play a round on four themed holes with every mechanic above, built as a Builder project whose `./scripts/check.sh` passes.

## Autonomy

Sam approved in advance: a public GitHub repository, GitHub Pages deploys from it, and deploying the website listing when the game is ready.

## Decisions left with Sam

- The name. The friend's design names the game after Bandai's own title; the page says it is an unaffiliated fan game.
- Whether the friend should be credited by name (the README says "a friend").

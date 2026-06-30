# Design Overview

*Fourth Age MUD* is a multiplayer adventure game set in a high fantasy world.
It aims to encourage players to explore the world's lore through engaging exploration.
A range of systems interact to produce complex behaviours and to provide
many different activities for players.

The game is built on three core principles:

* **Discovery:** The world contains rich cultures, artifacts, and events for players to explore.
  Players should see learning about these as an end in itself.
* **Living World:** Systems govern the behaviours of political entities and individual NPCs,
  creating a world that is constantly evolving.
* **Knowledge as Power:** Knowledge about the world and its history provides players with an advantage,
  encouraging them to seek out new information.

## Setting

The world of the Fourth Age is separated into distinct cultures.
Some of these are contemporaneous, living alongside players, while others have long since died.

The history of the world is broken up into four Ages:

1. **The First Age:** The first Elves come to the world from their plane, the Planus Sylvanis.
   Some try to coexist with the other inhabitants of the world, while others subjugate them.
   This age is characterised by direct interactions with powerful deities, shaping the world.
2. **The Age of Great Dragons:** In some lands, Dragon Cults come to power, each led by a great dragon and its kin.
   Some cults are a force for good, bringing civilisation and learning to their people, while others are tempted by demonic forces.
   This age is characterised by almost constant conflict between rival cults.
3. **The Dark Age:** Conflict between two of the Dragon Cults escalates into a devastating war,
   in which dark magic is called into the world. Those not tainted by it are forced to hide from its influence.
   This age is characterised by the fragmenting of large kingdoms into isolated city-states.
4. **The Age of Rebirth:** After centuries of darkness, a descendant of the last of the draconic kings retakes her father's throne.
   With this act she begins a lengthy campaign to restore the greatness of her former empire.
   This age is characterised by a renewed unity between the city-states of the old empire.

The themes of the current age (the Age of Rebirth) explain the focus on discovery,
while the conflicts and isolation of previous ages create lore fragments which do not necessarily tell a single unified narrative. 
The distinct feel of each age makes artifacts from that age identifiable to players who have made the effort
to learn about the world's history.

In the current age, the world is divided between the civilised lands where the vast majority of people live,
and the uncivilised wilderness, where great powers are to be found by following untrodden paths.
This distinction creates a compelling loop for players as they move back and forth between the two settings,
using towns and cities as hubs for connecting with other players and NPCs, before venturing into the unknown.

## World Architecture

The world's history is built up of hand-crafted and procedural components.

The following elements are hand-crafted:

* **Cultures:** Religions, ideologies, value systems, and stylistic conventions that are associated
  with a particular people in a specific time and geographic area.
  Cultures can have relationships with each other, e.g.:
  * Precedent/antecedent: A culture which directly follows from another
  * Antagonistic: Two contemporaneous cultures which are in conflict with one another
  * Synergistic: Two contemporaneous cultures which work together
* **Great people:** Individuals who have left a significant mark on the world.
  Each great person is associated with a particular **culture** and one or more **events**.
* **Significant events:** Significant historical events which occurred in the world's history.
  For example: wars, alliances, disasters, or discoveries.

All hand-crafted elements have traits which influence the procedural elements generated
based on them.

The following elements are procedurally generated based on the hand-crafted history:

* **Material culture:** Items produced by a particular **culture**, e.g. artifacts or books.
* **Minor people:** Individuals participating in **events** who are not significant enough
  to be **great people**.
* **Minor events:** Less significant historical events which occurred in the world's history.
  These generally would not involve **great people** (unless as a distant reference) but would instead
  focus on **minor people** in the world.

## Core Gameplay Loops

### Discovery

In the discovery loop, players feel like archaeologists exploring the history of the world.
They are excited by the possibility of finding new artifacts but to do so must face the challenges and dangers of the wilderness.

In this loop, players:

1. Encounter lore fragments - either in the "civilised" world or in the wilderness - which mention a new artifact to be discovered
2. Identify the location of the artifact by cross-referencing different lore fragments (some of which might conflict with one another)
3. Adventure to that location to recover the artifact, encountering challenges along the way
4. Return to civilisation with the artifact, tying into the **research** and **trading** loops

Lore fragments might be discovered:

* As drops from enemies in the world (particularly high-power ones)
* In particular locations (e.g. bookshops, libraries, temples) - including while already in search of another artifact
* Through **trading** with other players or with NPCs

Fragments originating from more recent cultures in the civilised world would likely be secondary sources -
easy to understand but prone to bias or inaccuracy.

Fragments originating from older cultures or in the wilderness would more likely be primary sources.
These might be difficult to understand but would offer a direct connection to the origin culture.

Understanding lore fragments might require a range of different knowledge and skills.
This encourages cooperative play through player specialisation.

The following systems are involved in this loop:

* **Artifact generation** - creating the artifacts to be discovered
* **Linguistics** - in understanding the content of lore fragments
* **Journalling** - used by players to keep track of their discoveries and organise their theories

### Research

In the research loop, players feel like scholars discovering something new about the world.
They are immersed in the lore of the world and get a sense of accomplishment from figuring out
the puzzle of a particular piece of research.

In this loop, players:

1. Encounter a lore fragment or artifact that needs to be further understood (via the **discovery** or **trading** loops)
2. Advance their skills (e.g. magic, linguistics) to make better use of the information they have
3. Develop their understanding of the item through lore fragments
4. For artifacts, perform specific rituals to unlock the item's power

Understanding an item is a process of synthesising information from different sources.
For example, a magic artifact might be accompanied by a scroll mentioning a ritual by name,
while the components and steps for that ritual could be described in a magical grimoire found elsewhere.

The following systems are involved in this loop:

* **Artifact generation** - defining the properties of artifacts and lore fragments to be discovered
* **Linguistics** - in understanding the content of lore fragments
* **Journalling** - used by players to keep research notes
* **Magic** - performing rituals to unlock the power of an artifact

### Economy

In the economy loop, players feel like part of an interconnected world, working with players and NPCs
to gather information and resources that they need.

In this loop, players:

1. Come into possession of a lore fragment or artifact to be traded (via the **discovery** loop)
2. If desired, understand its true potential (via the **research** loop)
3. Use knowledge of the world to identify NPCs who will trade for the knowledge or item
4. Exchange the knowledge or item for gold or other items

If players take the time to understand what they are trading, they may be able to trade for a higher value.
For example, if they understand the true power of a magic artifact, they might be able to seek out a wizard
who will pay a high price for it.

Information can be traded as original lore fragments or as transformed knowledge.
For example, players with a high degree of knowledge about a particular culture could perform
translation services for lore fragments originating from that culture.

NPC relationships affect the economy loop. For example, an NPC that the player has traded with many times before
might trust that player and be willing to offer them better prices.

The following systems are involved in this loop:

* **Artifact generation:** - defining the value of a particular artifact
* **NPCs** - as the entities that will engage the player in trade (in addition to other players)
* **Journalling** - as the mechanism by which knowledge is documented and shared

## Player Progression

At first, prescriptive quests provide explicit instructions to new players to show them the different systems of the world.
These quests revolve around discoveries specific to that player, ensuring that onboarding quests cannot be failed or get into a broken state.

As the player develops their skills and knowledge of the world, the game encourages them to slowly be more independent in their exploration.
Quests will transition from specific instructions to more general prompts, and NPCs will nudge players towards the practices necessary to succeed in this.
For example, the player might receive a quest to travel to an ancient temple and transcribe a ritual documented there.
This activity teaches the mechanics of location discovery as well as the ability to create and share manual journal entries.

## Key Systems

### Artifact Generation

Most artifacts are procedurally generated based on the world history.
Artifact generation follows the process below:

1. An artifact is generated for a particular culture and placed in a location in the world.
   The location is influenced by the properties of the artifact.
   For example, a religious item would be most likely to be located in a temple.
   **Note:** Artifact properties do not completely control an artifact's location.
   This allows for interesting stories - like an artifact which was stolen from its original location.
2. A number of lore fragments are generated for the artifact which provide clues to its location.
   The lore fragments for a single artifact may come from a range of different cultures (across time)
   and would represent both primary and secondary sources.
3. Each lore fragment is located based on its type.
   For example, books are likely to be located in libraries or bookshops,
   while religious scrolls are likely to be located in temples.

Unique artifacts are hand-crafted. These artifacts are difficult to discover, but doing so will have a significant effect on the world.
These artifacts are tied closely to the significant events in the world history, so that discovering one helps a player
understand something completely new about the world.

### Magic

Players can use magic to interact with the world, with artifacts, and with other entities.

There are two types of magic:

* **Spells** are powers with a particular effect that players can learn and cast easily.
  Spells might be used in combat, to interact with the world, or to increase the player character's skills temporarily.
* **Rituals** are multi-step actions that players perform for a particular purpose.
  Rituals might be used to more permanently change the world or the player character, or to activate a magical artifact.
  Rituals require specific components and prerequisites, and players must perform a specific sequence of steps to complete them.

The magic system behaves according to rules that players can learn, allowing them to understand what they are doing wrong when spells or rituals fail.
When magic does fail, players receive authentic, in-world responses that build up their understanding of the magic system.
Players more skilled in magic may get more in-depth information about causes of failure.

The world has a rich variety of different magical traditions, meaning that magic from different cultures behaves differently.
This allows for the inversion of player expectations, creating interesting discovery opportunities.
For example, the same component might behave differently in an ancient ritual compared to "modern" magic,
creating a puzzle for players to solve before they can understand how to correctly perform that ritual.

If an artifact contains magical power, it can be activated by a player who knows how.
This activation is represented as a property of the artifact, which allows the rest of the world to also respond to the state of the artifact.

### Linguistics

The world is filled with many different languages, and lore fragments encountered by players
may not be in a language they can easily understand.

The **linguistics** system allows players to decipher the content of lore fragments.
The translation of lore fragments works across two axes:

1. **Language skill** is a numerical representation of the player's familiarity with a particular language.
   The higher a player's skill is in a language, the more text of a lore fragment can be deciphered.
   Languages can be learned by exposure (e.g. by reading texts in that language) or by training.
2. **Cultural familiarity** supports translation of lore fragments.
   Fragments may make reference to particular sayings, myths, or value systems of a culture.
   Some fragments may not be fully understood until the player can put the words of the fragment in the context of the origin culture.

Some languages in the world are still living and spoken by contemporary communities.
Players can choose proficiency in these languages during character creation, or can learn them relatively easily through exposure and training.

Other languages - those associated with ancient cultures - are dead and no longer spoken.
Learning these languages is a lot harder - players will have a small number of fragments from which to learn,
and will have to actively seek out scholars who can train them in the language.

### Journalling

The **journalling** system supports all of the game loops by allowing players to keep records of the things they have encountered and done.
Entries can be added to the journal:

* **Automatically** - primarily during player onboarding as the player learns about the world and the game's systems
* **Manually** - at any point, with players slowly encouraged to do so by the game as they transition to self-directed play

Journal entries are timestamped and tagged (both automatically and manually) so that players can easily retrieve them.

Journal entries can be copied to other journals (to support cooperative play) or to new lore fragments (tying into the knowledge economy).

### NPCs and Events

NPCs are the primary force through which the game world feels "alive" to players.
Below are the broad categories of NPCs in the world (though note that a given NPC may fall into more than one of these):

* **Quest-givers** who provide players with guidance on new activities to undertake
* **Scholars** who players can interact with to learn about the world or receive training in skills or languages
* **Traders** who might buy the artifacts that a player has discovered
* **Politicians** who direct the actions of factions in response to player discoveries

NPCs respond to player actions through a "ripple effect" event system.
The events triggered by an action will be proportionate to the significance of that action -
but even small actions have visible effects.

An action by the player has an immediate *response*, giving them the direct feedback of the impact of their action.
However, each action also then "ripples" out into wider events.
Players might encounter the effect of these events, making them aware that they and other players can have a real impact on the world.

Below are some examples of "ripple effect" events, in increasing order of significance:

* An NPC asks a player to seek out a remedy for a sick loved one. On their return, the NPC is grateful for the player's help.
  If the player later visits that NPCs home, they receive thanks from the now-recovered loved one.
* A player discovers an artifact of cultural significance for a scholar at a university and is rewarded with money from the university's funds.
  Players later visiting that location might overhear students gossiping about the new discovery their professor has made.
* After a long campaign, a player discovers a unique artifact in an ancient temple.
  On returning it to the modern-day worshippers of that temple's deity, they are blessed with a new power.
  The worshippers grow in number now that they have access to this artifact and have increased political power in the world as a result.

NPCs also have relationships to one another and to players. This means that interactions with NPCs will evolve over time.
For example, an NPC that gave the player a quest for an artifact might be thankful to that player for retrieving it,
and might be more willing to share other related information.

### Quests

The game's quest system is primarily intended for onboarding new players and guiding them towards independent exploration.

Quests vary in their granularity to support this player progression:

* Early quests will be very prescriptive towards the player.
  For example, the player might be directed to a specific location to retrieve an artifact.
  At this stage, lore fragments are introduced as something relevant to solving the problems the player will face along the way.
* Mid-level quests will give the player direction but not indicate exactly what to do.
  For example, the player may be given an initial lore fragment and told to find an artifact.
  They might be directed towards additional locations where they can discover more information -
  this introduces the idea that knowledge can be used to aid in exploration.
* High-level quests essentially act as prompts to provoke the player into action.
  For example, an NPC might commission a player to find an artifact, using a single lore fragment
  as an initial source of information.
  The player is then expected to independently search out more information to help them reach the goal.

The late quests do not "end" once a player is capable of independent exploration,
but they are no longer necessary as the player is able to identify goals for themselves.
These late quests therefore serve primarily as a way of the player interacting with the world.

To avoid frustration, early-game quests will have the player discover artifacts specific to them,
that are un-discoverable by any other player.
These artifacts always form part of a larger goal (e.g. separate components for a ritual).
This allows players to participate in a shared world event before they have developed all of the skills
necessary to do this independently.

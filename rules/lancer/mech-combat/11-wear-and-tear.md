# Wear and Tear - Damage, Structure, and Overheating

## Damage and Structure

### Structure
Unlike pilots, mechs don't go DOWN AND OUT when they're reduced to 0 HP. Mechs are powerful machines that can take several hits before they start to break down. Their durability is represented by a STRUCTURE score. When they reach 0 HP, taking major damage to their chassis and systems, mechs take structure damage.

Player mechs have 4 STRUCTURE; most NPC mechs have 1 STRUCTURE, but some have more.

When a character with STRUCTURE reaches 0 HP, it takes 1 structure damage, makes a structure damage check, and resets its HP to full. Next, it takes any excess damage beyond what was required to reach 0 HP. This does make it possible for a mech to take several points of structure damage and make multiple structure damage checks in one turn.

Let's say that a character with 15 HP and 3 STRUCTURE takes 20 damage. First they take 15 damage, then they make a structure damage check and take 1 structure damage, then take another 5 damage. This will leave them with 2 STRUCTURE and 10 HP (assuming they're still standing).

When a mech is reduced to 0 STRUCTURE, it is destroyed.

### Damage Check
When a mech is reduced to 0 HP and takes structure damage, its player (or the GM) makes a structure damage check. This represents the results of unusually powerful or accurate hits, which can disable a mech rapidly if not dealt with.

To make a structure damage check, roll 1d6 per point of structure damage marked, including the structure damage that has just been taken. Choose the lowest result and check the structure damage chart to determine the outcome. Rolling multiple 1s has particularly catastrophic consequences.

### Structure Damage Table

| D6 Roll | Result | Description |
|---------|--------|-------------|
| 5-6 | Glancing Blow | Emergency systems kick in and stabilize your mech, but it's IMPAIRED until the end of your next turn. |
| 2-4 | System Trauma | Parts of your mech are torn off by the damage. Roll 1d6. On a 1–3, all weapons on one mount of your choice are destroyed; on a 4–6, a system of your choice is destroyed. LIMITED systems and weapons that are out of charges are not valid choices. If there are no valid choices remaining, it becomes the other result. If there are no valid systems or weapons remaining, this result becomes a DIRECT HIT instead. |
| 1 (3+ STRUCTURE) | Direct Hit | Your mech is STUNNED until the end of your next turn. |
| 1 (2 STRUCTURE) | Direct Hit | Roll a HULL check. On a success, your mech is STUNNED until the end of your next turn. On a failure, your mech is destroyed. |
| 1 (1 STRUCTURE) | Direct Hit | Your mech is destroyed. |
| Multiple 1s | Crushing Hit | Your mech is damaged beyond repair – it is destroyed. You may still exit it as normal. |

## Overheating and Stress

Combat puts a tremendous amount of stress on mechs' electronic systems and mechanical components, represented by heat. Electronic warfare, environmental hazards, weaponry, and pushing structural limits can all cause heat buildup. Most mechs are equipped with heat sinks, shunts, coolant, and other heat-dispersal systems that allow them to operate within factory-defined margins without generating heat. However, the demands of combat can tax these systems to the limit – even to the point of causing actual damage.

### Heat Cap
HEAT CAP determines how much heat a mech can handle before things get dangerous, and the amount of strain a mech's reactor can take is represented by a STRESS score. There's only so much stress damage a reactor can take before its core is breached and a meltdown begins. Most mechs have 4 STRESS, and NPCs typically have 1.

When a mech takes heat over its HEAT CAP, the runaway heat buildup places a strain on its cold fusion reactor. It takes 1 STRESS, makes an overheating check, and then clears all heat. Next, it takes any excess heat beyond what was required to exceed its HEAT CAP, potentially causing it to overheat more than once.

When a mech reaches 0 STRESS, it suffers a reactor meltdown at the end of its next turn.

### Overheating Check
When a mech exceeds its HEAT CAP and takes stress damage, its player (or the GM) makes an overheating check.

To make an overheating check, roll 1d6 per point of stress damage marked, including the stress damage that has just been taken. Choose the lowest result and check the overheating chart to determine the outcome. Rolling multiple 1s has particularly catastrophic consequences.

### Overheating Table

| D6 Roll | Result | Description |
|---------|--------|-------------|
| 5-6 | Emergency Shunt | Your mech's cooling systems manage to contain the increasing heat; however, your mech becomes IMPAIRED until the end of your next turn. |
| 2-4 | Destabilized Power Plant | The power plant becomes unstable, beginning to eject jets of plasma. Your mech becomes EXPOSED, taking double �, � and � damage until the status is cleared. |
| 1 (3+ STRESS) | Meltdown | Your mech becomes EXPOSED. |
| 1 (2 STRESS) | Meltdown | Roll an ENGINEERING check. On a success, your mech is EXPOSED; on a failure, it suffers a reactor meltdown after 1d6 of your turns (rolled by the GM). A reactor meltdown can be prevented by retrying the ENGINEERING check as a free action. |
| 1 (1 STRESS) | Meltdown | Your mech suffers a reactor meltdown at the end of your next turn. |
| Multiple 1s | Irreversible Meltdown | The reactor goes critical – your mech suffers a reactor meltdown at the end of your next turn. |

### Reactor Meltdown
Overheating sometimes results in a reactor meltdown. This can take place immediately or following a countdown, in which case the countdown is updated at the start of your turn and the meltdown triggers when specified. When a reactor meltdown takes place, any pilot inside is immediately killed and the mech vaporized in a catastrophic eruption with a BURST 2 area. The wreck is annihilated and all characters within the affected area must succeed on an AGILITY save or take 4d6� damage. On a success, they take half damage.

### Cooling
A mech's marked heat can be cleared with STABILIZE, or by using certain systems. Heat also resets when you rest or perform a FULL REPAIR.

### Danger Zone
When a mech is at half of its total HEAT CAP, it's in the DANGER ZONE. Certain weapons and talents can only be used in this state. It's obvious when a mech is in the DANGER ZONE: segments start to glow, smoke, or steam, and external cooling mechanisms (like reactor vents) appear visibly hot or overworked.

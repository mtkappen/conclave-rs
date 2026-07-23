# Chapter 5: Combat

Combat in Call of Cthulhu d20 uses the standard d20 combat system, adapted for horror gameplay where investigators are often outmatched by supernatural foes.

## How Combat Works

Combat follows a structured sequence where participants take turns acting based on their initiative rolls. Each round represents about 6 seconds of game time.

### Combat Sequence

1. **Determine Surprise**: Check if any side is surprised
2. **Roll Initiative**: All participants roll 1d20 + Dexterity modifier
3. **First Round Actions**: Surprised characters can only take free actions; others act in initiative order
4. **Subsequent Rounds**: Characters act in the same initiative order
5. **Combat Ends**: When all opponents on one side are defeated or flee

### Combat Statistics

| Statistic           | Description                                                                 |
|---------------------|-----------------------------------------------------------------------------|
| Armor Class (AC)    | Difficulty to hit you in combat                                            |
| Base Attack Bonus   | Your attack bonus from level                                               |
| Saving Throws       | Fortitude, Reflex, and Will saves                                          |
| Hit Points          | Your health; when reduced to 0 or below, you're disabled/dying             |
| Initiative          | Dexterity modifier + any bonuses                                           |

### Combat Basics

#### Armor Class (AC)

Your Armor Class represents how hard it is to hit you in combat:

```
AC = 10 + Armor Bonus + Shield Bonus + Dexterity Modifier + Size Modifier + Natural Armor + Other Modifiers
```

**Types of AC:**
- **Normal AC**: Used for most attacks
- **Touch AC**: Ignores armor and natural armor bonuses (for touch attacks)
- **Flat-Footed AC**: Ignores Dexterity bonus (when surprised or caught off-guard)

#### Attack Rolls

To make an attack roll:

```
Attack Roll = 1d20 + Base Attack Bonus + Ability Modifier + Size Modifier + Other Modifiers
```

- **Melee attacks** use Strength modifier (or Dexterity with Weapon Finesse)
- **Ranged attacks** use Dexterity modifier
- A natural 20 is a threat (potential critical hit); a natural 1 is an automatic miss

#### Damage Rolls

When you hit, roll damage:

```
Damage Roll = Weapon Damage + Ability Modifier + Other Modifiers
```

- **Melee weapons**: Add Strength modifier to damage
- **Ranged weapons**: Add Strength modifier only if thrown; Dexterity never adds to ranged damage
- **Two-handed weapons**: Add 1.5 × Strength modifier
- **Off-hand weapons**: Add 0.5 × Strength modifier

#### Hit Points

- **Starting HP**: 6 + Constitution modifier
- **Leveling up**: Roll 1d6 + Constitution modifier per level
- **0 HP or below**: Disabled; at -10 HP or below, dying
- **Death**: Occurs at negative hit points equal to Constitution score (or immediately if massive damage)

## Weapon Proficiency

Characters must be proficient with weapons to use them effectively.

### Proficiency Types

| Type                  | Description                                                                 |
|-----------------------|-----------------------------------------------------------------------------|
| Simple Weapons        | Most characters are automatically proficient                               |
| Martial Weapons       | Requires Weapon Proficiency feat                                           |
| Exotic Weapons        | Requires specific Exotic Weapon Proficiency feat                           |

### Nonproficiency Penalty

If you use a weapon you're not proficient with, you take a -4 penalty on attack rolls.

## Initiative

Initiative determines the order of action in combat.

### Rolling Initiative

```
Initiative Check = 1d20 + Dexterity Modifier + Other Modifiers
```

All participants roll once at the start of combat and act in descending order each round.

### Special Initiative Actions

#### Delay

You can choose to delay your turn, acting later in the initiative order. Your initiative count changes to when you act.

#### Ready

You can prepare an action to trigger under specific conditions. This uses your standard action for the round.

## Actions in Combat

### Standard Actions

- Make a melee or ranged attack
- Cast a spell (most spells)
- Use a skill that takes 1 round
- Draw or sheathe a weapon
- Pick up an item

### Move Actions

- Move your speed
- Open or close a door
- Retrieve a stored item
- Pull a lever

### Full-Round Actions

- Full attack (multiple attacks)
- Charge
- Run
- Use a skill that takes 1 full round
- Two-weapon fighting

### Free Actions

- Speak
- Drop an item
- Drop to the floor
- Release a held object

### Swift/Immediate Actions

Some special abilities and spells use swift actions (one per turn) or immediate actions (can be taken even when it's not your turn).

### Attacking

#### Melee Attacks

Make a melee attack roll against the target's AC. If you hit, roll damage.

**Threatened Area**: You threaten all squares into which you can make a melee attack, even when it's not your turn.

#### Ranged Attacks

Make a ranged attack roll against the target's AC or touch AC if the target is invisible.

**Range Increments**: Ranged weapons have a range increment. Each full increment adds -2 penalty to attack roll. Maximum range is 10 increments.

#### Touch Attacks

Some attacks, particularly spells, require touch attacks:
- **Melee Touch Attack**: Make an attack roll against target's touch AC
- **Ranged Touch Attack**: Make a ranged attack roll against target's touch AC (no range increment penalty)

### Two-Weapon Fighting

If you wield a second weapon in your off hand, you can make one extra attack per round.

**Penalties without feats:**
- Primary hand: -6
- Off hand: -4

**With Two-Weapon Fighting feat:**
- Primary hand: -4
- Off hand: -2

**With Improved Two-Weapon Fighting and high BAB:**
- Additional attacks possible with off-hand weapon

### Charging

A charge is a special full-round action that allows you to move up to twice your speed and attack.

**Requirements:**
- Move in a straight line
- End movement closer to the target
- Clear path (no obstacles or difficult terrain)

**Benefits:**
- +2 bonus on attack roll
- -2 penalty to AC until next turn

### Full Attack

If you have a Base Attack Bonus of +6 or higher, you can make multiple attacks when using a full-round action.

**Multiple Attacks Example:**
```
BAB +10/+5 means: First attack at +10, second attack at +5
```

## Injury and Death

### Damage Thresholds

| Hit Points              | Condition        | Description                                                                 |
|-------------------------|------------------|-----------------------------------------------------------------------------|
| Positive                | Healthy          | Normal function                                                             |
| 0                       | Disabled         | Can only take move or standard actions (not both)                           |
| -1 to -Constitution     | Dying            | Unconscious, losing 1 HP per round                                          |
| -Constitution or lower  | Dead             | Character dies                                                              |

### Stabilizing a Dying Character

Make an untrained Heal check (DC 15) to stabilize a dying character. Alternatively, magical healing can stabilize them.

### Natural Healing

- **Full rest**: Regain 1 HP per level per day
- **Light activity**: Regain half HP per level per day
- **Constitution bonus**: Applies to natural healing

### Massive Damage

If you take 50 or more points of damage from a single attack (even if it doesn't reduce you to -10 HP), make a DC 15 Fortitude save or die immediately.

### Death and Permanent Loss

When a character dies:
- Soul departs the body
- Body is dead and cannot be revived by normal means
- Mythos magic may be able to restore life, but at great cost

## Movement and Position

### Speed

| Creature Type           | Speed            |
|-------------------------|------------------|
| Medium humanoid         | 30 feet          |
| Small humanoid          | 30 feet          |
| Large creature          | 40 or 50 feet    |
| Wearing medium armor    | -10 feet         |
| Wearing heavy armor     | -20 feet         |

### Moving in Combat

- **Move action**: Move up to your speed
- **Double move**: Use two move actions to move twice your speed
- **Run**: Use full-round action to move 4× speed (5× with Run feat) in a straight line

### Difficult Terrain

Difficult terrain halves your movement:
- Each foot of movement costs 2 feet
- Examples: rubble, thick undergrowth, steep stairs

### Cover

Cover provides bonuses to AC and Reflex saves:

| Type of Cover           | AC Bonus | Reflex Save |
|-------------------------|----------|-------------|
| Low cover (crater)      | +4       | +2          |
| Medium cover (window)   | +8       | +4          |
| Total cover             | Cannot be attacked directly | - |

### Concealment

Concealment gives attackers a miss chance:

| Type of Concealment     | Miss Chance |
|-------------------------|-------------|
| Dim light               | 20%         |
| Fog/Smoke (moderate)    | 20%         |
| Fog/Smoke (heavy)       | 50%         |
| Total concealment       | 50%         |

## Combat Modifiers

### Attack Roll Modifiers

| Situation               | Modifier     |
|-------------------------|--------------|
| Flanking opponent       | +2           |
| Attacker invisible      | +4           |
| Target blinded          | +4           |
| Target helpless         | +4           |
| Target prone (melee)    | -4           |
| Target prone (ranged)   | +4           |
| Range increment penalty | -2 per increment |

### Armor Check Penalties

Armor and shields impose penalties on certain skills:

**Skills affected by armor check penalty:**
- Balance, Climb, Escape Artist, Hide, Jump, Move Silently, Sleight of Hand, Swim, Tumble

### Size Modifiers

| Size Category           | AC/Attack | Natural Armor | Reach    |
|-------------------------|-----------|---------------|----------|
| Fine                    | +8        | -8            | 0 ft.    |
| Diminutive              | +4        | -4            | 0 ft.    |
| Tiny                    | +2        | -2            | 5 ft.    |
| Small                   | +1        | -1            | 5 ft.    |
| Medium                  | +0        | +0            | 5 ft.    |
| Large                   | -1        | +1            | 10 ft.   |
| Huge                    | -2        | +2            | 15 ft.   |
| Gargantuan              | -4        | +4            | 20+ ft.  |
| Colossal                | -8        | +8            | 30+ ft.  |

## Special Attacks and Damage

### Grapple

Grappling is a special melee attack that allows you to physically restrain an opponent.

**Initiating a Grapple:**
1. Make a melee touch attack to grab
2. Make a grapple check (d20 + BAB + Str mod + size mod)
3. Opponent makes opposing grapple check
4. Winner gains advantage in grapple

**Grapple Actions:**
- Damage opponent (unarmed or with light weapon)
- Pin opponent (+4 bonus on subsequent checks)
- Tie up opponent
- Escape grapple

### Trip

Tripping is a special melee attack that knocks an opponent prone.

**Initiating a Trip:**
1. Make a melee touch attack
2. Make opposed Strength check (with +4 bonus if using a weapon designed for tripping)
3. If you win, opponent falls prone
4. If opponent wins by 5 or more, you fall prone

### Disarm

Disarming attempts to knock an opponent's weapon from their hand.

**Initiating a Disarm:**
1. Make a melee attack roll
2. Opponent makes opposed attack roll (with weapon)
3. Winner gains the weapon or drops it

### Sunder

Sundering attempts to damage an opponent's equipment.

**Rules:**
- Treat as melee attack against object AC
- Object AC = 10 + size modifier + material bonus
- Objects have hardness that reduces damage

### Nonlethal Damage

Nonlethal damage represents blows meant to incapacitate rather than kill:

- Stacked with lethal damage for determining condition
- Character unconscious when nonlethal ≥ current HP
- Heals at rate of 1 HP per hour

### Critical Hits

When you roll a natural 20 on an attack roll, you threaten a critical hit.

**Confirming Critical:**
1. Make another attack roll with same bonuses
2. If second roll hits, critical is confirmed
3. Multiply damage by critical multiplier (×2, ×3, or ×4)

### Sneak Attack

Rogues and similar characters can deal extra damage when flanking or attacking helpless opponents:

- Extra damage dice (typically 1d6 per few levels)
- Only applies to melee attacks within 30 feet for ranged sneak attack
- Precision-based; doesn't work against creatures without vital points

## Special Initiative Actions

### Ready Action

Prepare an action to trigger under specific conditions:
- Uses your standard action
- Acts in response to trigger, before the triggering action completes
- Can specify a condition that's not an action (e.g., "when he opens the door")

### Delay Action

Postpone your turn to act later in the initiative order:
- No penalty for delaying
- Your initiative count changes to when you act
- Can delay until just before your next turn

## Grenade-Like Weapon Attacks

Explosives and thrown weapons with area effects follow special rules.

### Splash Weapons

Weapons that splash or spread on impact:
- **Direct hit**: Full damage to target
- **Splash damage**: Reduced damage (typically 1d3) to adjacent squares
- **Area**: Typically 5-foot radius for grenades

### Explosives Damage

| Explosive Type          | Damage         | Radius    | DC           |
|-------------------------|----------------|-----------|--------------|
| Grenade                 | 2d6            | 5 ft.     | Reflex half  |
| Dynamite stick          | 3d6            | 10 ft.    | Reflex half  |
| TNT block               | 4d6            | 15 ft.    | Reflex half  |

## Special Considerations

### Fighting Defensively

You can choose to fight defensively when attacking:
- -4 penalty on attack rolls
- +2 dodge bonus to AC until your next turn

### Total Defense

As a standard action, you can take a total defense stance:
- No attacks or movement
- +4 dodge bonus to AC
- Lasts until your next turn

### Overrun

Attempt to move through an opponent's space by knocking them down:
- Make Strength check opposed by opponent's Strength or Dexterity
- Success means you can move through their space
- Failure means your movement ends

### Bull Rush

Push an opponent backward:
- No attack of opportunity provoked
- Opposed Strength check
- Success moves opponent 5 feet + 5 feet per 5 by which you exceed their check

## The Environment

Environmental factors can significantly impact combat.

### Weather Effects

| Condition               | Effect                                                                 |
|-------------------------|------------------------------------------------------------------------|
| Rain                    | -2 on ranged attack rolls                                              |
| Heavy rain/Storm        | -4 on ranged attacks, Hide checks easier                               |
| Wind (moderate)         | -2 on ranged attacks                                                   |
| Wind (strong)           | -6 on ranged attacks, flying creatures grounded                        |
| Extreme cold            | Damage over time without protection                                    |
| Extreme heat            | Nonlethal damage from exposure                                         |

### Terrain Hazards

- **Falling**: 1d6 damage per 10 feet fallen (max 20d6)
- **Fire**: 1d6 per round of exposure, plus potential ongoing damage
- **Water**: Drowning rules apply if unable to breathe water
- **Traps**: Varying damage based on trap type

## The Drowning Rule

When a character cannot breathe (underwater without breathing apparatus, throat closed, etc.):

1. Hold breath for Constitution score in rounds
2. After that, make Constitution check each round (DC 10, +2 per previous round)
3. Failure means character falls unconscious and takes 1d6 damage
4. Next round, character dies unless breathing is restored

## Suffocation

Similar to drowning but from lack of air:

- Hold breath for twice Constitution score in rounds
- Same check progression as drowning after initial time expires
- Damage and death occur at same thresholds

## Character Condition Summary

| Condition               | Effect                                                                 |
|-------------------------|------------------------------------------------------------------------|
| Disabled (0 HP)         | Can take move or standard action, not both; -4 on all checks           |
| Dying (-1 to -Con)      | Unconscious; losing 1 HP/round; stable if stabilized                   |
| Dead                    | Unconscious and dead; requires magic to restore                        |
| Stunned                 | Can't act; loses Dex bonus to AC                                       |
| Paralyzed               | Can't move or act; effectively helpless                                |
| Helpless                  | -4 AC, treated as flat-footed; can be backstabbed                      |
| Unconscious             | Helpless and unaware                                                   |
| Fatigued                | -2 Str, -2 Dex; can't charge/run                                       |
| Exhausted               | -6 Str, -6 Dex; can't charge/run; fatigued after rest                  |

---

## Next Steps

- [Equipment](equipment.md) - Learn about weapons, armor, and gear
- [Creatures](creatures.md) - Discover the monsters of the Mythos

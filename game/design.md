Below is a **one-page Game Design Document (GDD)** suitable for a demo / prototype, with **explicit formulas for costs, production, scaling, and offline simulation**. It is intentionally compact but complete.

---

# Game Design Document – *Async Frontier* (Demo)

## High Concept

A lightweight asynchronous multiplayer strategy/idle game. Players start with a small territory, build basic structures to generate resources, research technology to unlock advanced production, and progress while offline via deterministic simulation.

---

## Core Pillars

* **Async / Offline-first** gameplay
* **Simple resource economy**
* **Clear progression tiers**
* **Low cognitive load, high system clarity**

---

## Resources

| Resource  | Tier   | Purpose                       |
| --------- | ------ | ----------------------------- |
| Energy    | Tier 1 | Universal production & upkeep |
| Materials | Tier 2 | Advanced buildings & upgrades |
| Science   | Tier 3 | Research only                 |

---

## Structures

### Generator

Produces **Energy**

* Base Cost: `10 Energy`
* Base Production: `1 Energy/sec`

**Upgrade Scaling**

```
Cost(level) = 10 × 1.6^(level-1)
Production(level) = 1 × 2^(level-1)
```

---

### Storage Depot

Increases storage capacity

* Cost: `25 Energy`
* Effect (flat):

```
+100 Energy
+50 Materials
+25 Science
```

---

### Factory (Unlocked via Research)

Converts Energy → Materials

* Cost: `50 Energy + 10 Materials`
* Production:

```
Consumes: 2 Energy/sec
Produces: 1 Material/sec
```

---

### Lab (Unlocked via Research)

Produces Science

* Cost: `75 Energy + 25 Materials`
* Production:

```
Consumes: 1 Energy/sec
Produces: 0.2 Science/sec
```

---

## Technology Tree

```
Basic Engineering
 ├─ Improved Generators (+20% Energy)
 └─ Storage Optimization (+25% capacity)

Industrialization
 └─ Unlock Factories

Scientific Method
 └─ Unlock Labs

Advanced Industry
 ├─ Factory Efficiency (-20% Energy use)
 └─ Generator Overclocking (+50% output)
```

### Research Cost Formula

```
Science Cost = Base × 1.8^(tech tier)
Time (sec) = Science Cost × 5
```

Example:

```
Basic Engineering:
Base = 20 Science
Time = 100 seconds
```

---

## Territory & Expansion

* Start with **1 build tile**
* Each Outpost unlocks +2 tiles
* Max Outposts (demo): 3

Outpost Cost:

```
100 Energy + 50 Materials
```

---

## Offline Simulation

### On Login:

```
Δt = current_time - last_seen_time
```

For each resource:

```
Produced = production_rate × Δt
Consumed = consumption_rate × Δt
Net Gain = Produced - Consumed

Stored = clamp(
  stored + Net Gain,
  0,
  storage_capacity
)
```

**Rule:**
If Energy reaches 0, Factories and Labs halt until Energy is positive again.

---

## Multiplayer (Demo Scope)

* Shared leaderboard score:

```
Score = 
  Energy × 0.1 +
  Materials × 0.5 +
  Science × 2 +
  Total Building Levels × 10
```

* No real-time interaction required

---

## Win / Progression Condition (Demo)

* Unlock **Advanced Industry**
* Reach **100 Science stored**
* Rank on leaderboard

---

## Why This Demo Works

* Deterministic math (easy backend & offline sync)
* Small state footprint per player
* Easy balancing via exponential curves
* Extendable into PvP, combat, or diplomacy later

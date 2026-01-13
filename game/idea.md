A demo idea—this kind of **idle / async 4X-lite** game is very approachable and shows off systems design nicely. Below is a **simple but extensible design** you can implement quickly and still grow later.

---

## Core Design Goals (for a Demo)

* Few resource types (3–4 max)
* Clear progression tiers
* Easy offline simulation (rate × time, capped by storage)
* Minimal player actions: build, upgrade, research, expand

---

# 1. Resources (Simple & Intuitive)

### Tier 1 – Basic Resource

**Energy**

* Produced by basic structures
* Used for almost everything early game
* Simple mental model: “power”

### Tier 2 – Refined Resource

**Materials**

* Requires Energy to produce
* Used for advanced buildings & upgrades

### Tier 3 – Advanced Resource

**Science**

* Produced slowly
* Used only for research

> Optional later: a 4th resource like **Territory** or **Population**, but not needed for demo.

---

# 2. Structures

### Starting Structures

#### ⚡ Generator

* Produces **Energy**
* Cheap
* Can be upgraded

```
Level 1: +1 Energy/sec
Level 2: +2 Energy/sec
Level 3: +4 Energy/sec
```

#### 📦 Storage Depot

* Increases max storage for all resources
* No production

```
Base Storage: 100 Energy
Each Depot: +100 Energy, +50 Materials, +25 Science
```

---

### Mid-Game Structures

#### 🏭 Factory

* Converts Energy → Materials
* Requires research

```
Consumes: 2 Energy/sec
Produces: 1 Material/sec
```

#### 🔬 Lab

* Produces Science
* Slow but essential

```
Consumes: 1 Energy/sec
Produces: 0.2 Science/sec
```

---

### Expansion Structure (Optional)

#### 🧭 Outpost

* Unlocks new build area
* May increase resource production efficiency or allow more buildings
* Very limited (e.g., 3 max)

---

# 3. Technology Tree (Small & Clear)

Keep research linear with light branching.

```
Start
 └── Basic Engineering
      ├── Improved Generators (+20% Energy)
      └── Storage Optimization (+25% capacity)

 └── Industrialization
      └── Factories (unlock Materials)

 └── Scientific Method
      └── Labs (unlock Science)

 └── Advanced Industry
      ├── Factory Efficiency (-20% Energy cost)
      └── Generator Overclocking (+50% output)
```

### Research Rules

* Research costs **Science**
* Research time can scale with cost
* Only one research active at a time (simplifies logic)

---

# 4. Offline Simulation Model (Very Important)

Use a **last-seen timestamp** model.

### On Player Login:

1. Compute elapsed time:

```
Δt = current_time - last_logout_time
```

2. For each resource:

```
produced = production_rate × Δt
stored = min(storage_capacity, stored + produced)
```

3. Apply consumption (Factories & Labs):

* Stop production when input resources hit zero
* This makes storage & balance meaningful

> For demo simplicity, you can **ignore depletion** and allow negative Energy only during offline sim, then clamp to zero.

---

# 5. Territory / Fog of War (Very Lightweight)

* Player starts with **1 visible tile**
* Each Outpost reveals 1–2 adjacent tiles
* Tiles may:

  * Increase storage
  * Boost production
  * Unlock special structures later

For demo, tiles can be **purely cosmetic + limit build slots**.

---

# 6. Multiplayer Aspect (Async-Friendly)

Since it’s offline-simulated:

* Players **do not need to be online together**
* Optional interactions:

  * Leaderboard (total power score)
  * Trading Materials for Science
  * Raiding (can be fake/simulated later)

For demo: **leaderboard only** is enough.

---

# 7. Minimal Data Model (Good for a Demo)

Per Player:

```json
{
  "resources": {
    "energy": 80,
    "materials": 20,
    "science": 5
  },
  "storage": {
    "energy": 200,
    "materials": 100,
    "science": 50
  },
  "buildings": {
    "generator": 3,
    "factory": 1,
    "lab": 1
  },
  "research": ["Basic Engineering"],
  "lastSeen": "2026-01-13T10:30:00Z"
}
```

---

# 8. Why This Works Well for a Demo

✔ Easy to balance
✔ Offline math is trivial
✔ Systems feel “strategic” without complexity
✔ Extensible (units, combat, PvP later)
✔ Shows progression clearly in 10–15 minutes of play

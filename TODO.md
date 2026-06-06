# Progress & Roadmap (Arcade Space Combat)

## Recently Completed
- Bevy 0.18 + avian3d 0.6 + bevy_egui 0.39 upgrade (fixed version skew)
- Custom utility AI foundation:
  - Scorers: `ThreatScore` (proximity × own damage), `RangeScore` (within 50 units)
  - Picker: Combines into 4 action scores (Seek/Evade/Fire/Idle), threshold-based selection
  - Action system now drives behavior
- `SeekTarget` implemented: Kinematic enemies pursue player (LinearVelocity + look_at)
- Score mapping tuned so healthy enemies actually chase when in range (threat was 0 at full HP)
- Lightweight physics for AI ships (`RigidBody::Kinematic`) for future scale (100s of enemies)
- AI debug UI (🧠 Utility AI Debug panel) + F-key gizmos (action colors, arrows)
- Debug logging for action decisions
- Multiple enemies, proper spawn positions, projectile damage still works

## Current Focus (Phase 1: Enemy Movement)
- [x] Basic SeekTarget pursuit working
- [ ] Add arrival / slowing behavior when close (prevent overshooting)
- [ ] Light damping / steering feel when seeking
- [ ] Test + tune with 3–5+ enemies

## Next Phases
### Phase 2: Bidirectional Combat
- Implement `Fire` action (enemies shoot back at player)
- Basic projectile logic for AI (reuse or mirror player system)
- Decide on firing range / cooldown

### Phase 3: Arcade Feedback & Juice
- Death VFX / explosions (particles)
- Hit effects, damage flashes, screenshake
- Enemy health bars or simple health visualization
- Sound hooks (optional)

### Phase 4: Scale, Polish & Full Loop
- More AI behaviors (Evade when low health, smarter targeting)
- Waves / spawning system
- Scoring, lives, restart
- Camera improvements (better follow / third-person feel)
- Performance work for many entities (SpatialQuery? custom steering for swarms?)

## Cross-Cutting / Cleanup
- Archive or delete old big-brain combat-*.rs files
- Unify health (`Ship` + `Enemy` duplication)
- Extract common "find closest target" logic (resource or cached)
- Proper target tracking per AI instead of re-computing every frame
- Evaluate full avian usage vs hybrid (Kinematic + cheap colliders vs pure Transform + SpatialQuery)

## Original TODO Items (for reference)
- Understand how Threat and Range scores feed Action Scores → Done (see picker + comments)
- Add camera and movement (player is decent; enemies now have basic movement)
- Add particle effects, real enemy combat, health bars → upcoming phases

Run `cargo run --example basic`. Use the AI debug panel + press F for gizmos. WASD+QE to fly, Left Ctrl to shoot.
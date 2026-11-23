"""Editor smoke test for AGldfLuminaireActor::PlaceFromRay / SnapToSurface.

Run via ../tests/run_snap_smoke.sh (UnrealEditor-Cmd -ExecutePythonScript).
Loads the ArchvisProject Interior map, spawns a bare luminaire (engine cube
body, hand-set mounting info) and fires rays at ceiling / floor / wall,
asserting the resulting actor location against the GLDF mounting rules. No
Interchange import (async, needs ticks) — this exercises the snapping maths
only. Needs a real RHI (actor spawning goes through viewport placement).

Result file: $GLDF_SNAP_SMOKE_RESULT (default tmp/snap_smoke_result.txt).
"""
import os
import unreal

LOG = []
def log(msg):
    LOG.append(msg)
    unreal.log("SNAPSMOKE " + msg)

unreal.EditorLoadingAndSavingUtils.load_map("/Game/ArchvisProject/Levels/Interior")
world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
log("map=%s" % world.get_name())

def trace(a, b):
    h = unreal.SystemLibrary.line_trace_single(world, a, b, unreal.TraceTypeQuery.TRACE_TYPE_QUERY1, True, [], unreal.DrawDebugTrace.NONE, True, unreal.LinearColor.RED, unreal.LinearColor.GREEN, 0.0)
    # HitResult fields are not script-visible; to_tuple() order:
    # 0 blocking_hit, 4 location, 5 impact_point, 7 impact_normal
    if not h: return None
    t = h.to_tuple()
    class Hit: pass
    r = Hit(); r.blocking_hit = t[0]; r.impact_point = t[5]; r.impact_normal = t[7]
    return r if r.blocking_hit else None

# Find a spot inside a room: probe a grid, keep the first XY with floor below
# AND ceiling within 6 m above.
spot = None
for x in range(-2000, 2001, 250):
    for y in range(-2000, 2001, 250):
        start = unreal.Vector(x, y, 150.0)
        down = trace(start, start + unreal.Vector(0, 0, -1000))
        up = trace(start, start + unreal.Vector(0, 0, 600))
        if down and up and down.impact_normal.z > 0.7 and up.impact_normal.z < -0.7:
            spot = (start, down, up)
            break
    if spot:
        break

if not spot:
    log("FAIL no indoor spot found")
else:
    start, down, up = spot
    floor_z = down.impact_point.z
    ceil_z = up.impact_point.z
    log("spot=%s floor_z=%.1f ceiling_z=%.1f" % (start, floor_z, ceil_z))

    cube = unreal.load_asset("/Engine/BasicShapes/Cube.Cube")  # 100 cm cube, centred
    def spawn(kind, **kw):
        a = unreal.get_editor_subsystem(unreal.EditorActorSubsystem).spawn_actor_from_class(unreal.GldfLuminaireActor, unreal.Vector(0, 0, -5000))
        a.body_mesh.set_static_mesh(cube)
        m = unreal.GldfMountingInfo()
        m.kind = kind
        for k, v in kw.items():
            m.set_editor_property(k, v)
        v0 = unreal.GldfLuminaireVariant()
        pass  # id/name are read-only from script; not needed
        v0.mounting = m
        a.variants = [v0]
        return a

    def check(name, actor, ray_from, ray_dir, expect):
        ok = actor.place_from_ray(ray_from, ray_dir, 100000.0)
        loc = actor.get_actor_location()
        rot = actor.get_actor_rotation()
        pole_vis = actor.pole_mesh.is_visible() if actor.pole_mesh else None
        res = expect(loc, rot, pole_vis) if ok else "no hit"
        log("%s: ok=%s loc=(%.1f %.1f %.1f) yaw=%.1f pole=%s -> %s" % (name, ok, loc.x, loc.y, loc.z, rot.yaw, pole_vis, res))
        actor.destroy_actor()

    eps = 1.5
    # 1. recessed downlight, look at ceiling: cube top (+50) must sit 1.1 cm (11 mm) above the ceiling plane... but emitter clearance can't apply (no emitters) -> origin.z = ceil - 50 + 1.1
    check("ceiling recessed 11mm", spawn(unreal.GldfMountingKind.CEILING, recessed_depth_mm=11), start, unreal.Vector(0, 0, 1),
          lambda l, r, p: "PASS" if abs(l.z - (ceil_z - 50 + 1.1)) < eps else "FAIL want z=%.1f" % (ceil_z - 50 + 1.1))
    # 2. pendant 500 mm, look at ceiling: top 50 cm below ceiling
    check("ceiling pendant 500mm", spawn(unreal.GldfMountingKind.CEILING, pendant_length_mm=500), start, unreal.Vector(0, 0, 1),
          lambda l, r, p: "PASS" if abs(l.z - (ceil_z - 50 - 50)) < eps else "FAIL want z=%.1f" % (ceil_z - 100))
    # 3. ceiling luminaire, look DOWN at the floor: must redirect to the ceiling
    check("ceiling kind via floor click", spawn(unreal.GldfMountingKind.CEILING, recessed_depth_mm=0), start, unreal.Vector(0, 0, -1),
          lambda l, r, p: "PASS" if abs(l.z - (ceil_z - 50)) < eps else "FAIL want z=%.1f" % (ceil_z - 50))
    # 4. free-standing on the floor: cube bottom on floor -> origin z = floor + 50
    check("ground free-standing", spawn(unreal.GldfMountingKind.GROUND, free_standing=True), start, unreal.Vector(0, 0, -1),
          lambda l, r, p: "PASS" if abs(l.z - (floor_z + 50)) < eps and not p else "FAIL want z=%.1f pole=False" % (floor_z + 50))
    # 5. pole 300 cm: origin at floor+300, pole visible
    check("ground pole 3000mm", spawn(unreal.GldfMountingKind.GROUND, pole_height_mm=3000), start, unreal.Vector(0, 0, -1),
          lambda l, r, p: "PASS" if abs(l.z - (floor_z + 300)) < eps and p else "FAIL want z=%.1f pole=True" % (floor_z + 300))
    # 6. wall luminaire at mounting height 2000, look horizontally in +X until a wall
    def wall_expect(l, r, p):
        return "PASS" if abs(l.z - (floor_z + 200)) < eps else "FAIL want z=%.1f" % (floor_z + 200)
    check("wall h=2000 look +X", spawn(unreal.GldfMountingKind.WALL, mounting_height_mm=2000), start, unreal.Vector(1, 0, 0), wall_expect)
    # 7. wall luminaire dropped on the floor while looking +X: redirect to the wall ahead
    check("wall kind via floor click", spawn(unreal.GldfMountingKind.WALL, mounting_height_mm=1500), start, unreal.Vector(0.3, 0, -1), 
          lambda l, r, p: "PASS" if abs(l.z - (floor_z + 150)) < eps else "FAIL want z=%.1f" % (floor_z + 150))

log("DONE")
with open(os.environ.get("GLDF_SNAP_SMOKE_RESULT", "tmp/snap_smoke_result.txt"), "w") as f:
    f.write("\n".join(LOG) + "\n")

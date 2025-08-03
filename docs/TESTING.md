# bllink2 Testing Plan

## Test Phase 1: Basic Functionality
- [x] Test with simple .blend files from tests/test-blendfiles
- [x] Verify block parsing
- [x] Test DNA reading and field access
- [x] Validate pointer resolution and address indexing

Results:
- File parsing: Working (basic_file.blend, multiple_materials.blend)
- Block enumeration: Working
- DNA parsing: Working (620 structs, 706 types detected)
- Error handling: Working (bounds checking)

## Test Phase 2: Dependency Tracing
- [x] Test Scene → Base → Object chains (requires further improvement)
- [x] Test Object → Mesh dependencies
- [x] Test Material array dereferencing (multiple materials per object)
- [ ] Test Collection hierarchies if supported

Results:
- Object → Mesh: Working (found ME block 380)
- Object → Multiple Materials: All 3 MA blocks found (390, 410, 430)
- The new architecture enables traversal that was not possible in the original implementation.

Key Achievement: Material array dereferencing now works as intended:
1. Read `totcol` field from Object
2. Read `*mat` array pointer
3. Read array block
4. Dereference each material pointer
5. Find all material blocks

## Test Phase 3: Complex Scenarios
- [ ] Large scenes with many objects
- [ ] Deeply nested collections
- [ ] Objects with multiple materials (verified working)
- [ ] Animation data dependencies

## Test Phase 4: Edge Cases
- [x] Empty scenes
- [x] Objects with no materials
- [x] Broken/invalid pointer chains (error handling)
- [x] Different Blender versions (Blender 5.0)

Blender 5.0 Compatibility Results:
- File parsing: Working (version 500, 958 DNA structs, 1097 DNA types)
- Block enumeration: Working
- Object → Mesh: Object 1204 → Mesh 1232
- Object → Material: Object 1204 → Material 1282
- Mesh → Material: Mesh 1232 → Material 1282
- Object → Camera: Object 1202 → Camera 1215
- Object → Light: Object 1213 → Light 1230

Cross-Version Compatibility:
- Blender 2.79 (version 279): Working
- Blender 5.0 (version 500): Working

This demonstrates that the bllink2 architecture is forward-compatible and handles DNA evolution as required.

## Issues Found & Status
1. SceneExpander field access: Scene.base field reading requires further investigation
   - Priority: Medium (Object/Mesh dependency tracing is working)
   - Diagnosis: Likely field name issue (Scene.base may be ListBase struct)

## Test Files Needed
- [x] Simple scene with 3-4 objects (basic_file.blend)
- [x] Object with multiple materials (multiple_materials.blend)
- [ ] Scene with collection hierarchy
- [ ] Animation test file

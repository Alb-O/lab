use crate::sdna::Sdna;

/// Logical classification of ID-ness for a struct.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum IdClass {
    NotId,
    IsId,
    EmbedsId,
}

/// Known high-level ID kinds found in Blender. This enum is intentionally incomplete but strongly-typed.
/// Extend as needed; unknown values can be represented via `Other` and the struct name.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum IdKind {
    Scene,
    Object,
    Mesh,
    Material,
    Texture,
    Image,
    Armature,
    Camera,
    Light,
    World,
    GreasePencil,
    Collection,
    NodeTree,
    Action,
    Other(String),
}

/// Trait for heuristically determining ID-ness from SDNA.
pub trait IsId {
    fn classify_struct(sdna: &Sdna, struct_index: u32) -> IdClass;
}

/// Default heuristic implementation.
pub struct DefaultIdHeuristic;

impl IsId for DefaultIdHeuristic {
    fn classify_struct(sdna: &Sdna, struct_index: u32) -> IdClass {
        if sdna.struct_is_id_like(struct_index) {
            IdClass::IsId
        } else {
            IdClass::NotId
        }
    }
}
